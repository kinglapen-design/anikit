//! 命令 API：储存空间与资源库管理（创建 / 打开 / 当前状态 / 空间内库列表 / 最近列表 / 合并）。
//! 层级：储存空间（space.json + assets/ 池）→ 资源库（纯数据目录），PRD 01_储存空间。

use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::core::config::{self, LibraryEntry};
use crate::core::json_io::DataError;
use crate::core::library::{self, Library};
use crate::core::space::{self, Space};
use crate::core::{backup, merge, paths, relations};

/// 应用状态：当前储存空间 + 当前打开的素材库（同一时刻只开一个库）+ 导入任务注册表 + 收件箱实例 + 撤销栈
pub struct AppState {
  pub space: Mutex<Option<Space>>,
  pub library: Mutex<Option<Library>>,
  /// 导入 / 导出任务取消标记：task_id → 置位即取消（任务结束从表中移除）
  pub imports: Mutex<std::collections::HashMap<u64, std::sync::Arc<std::sync::atomic::AtomicBool>>>,
  /// 收件箱运行实例（3.2）：空间级共用，随空间打开 / 自动导入开关 / 退出由 services::inbox 启停
  pub inbox: Mutex<Option<crate::services::inbox::InboxHandle>>,
  /// 工作空间监听实例集（PRD 02 步骤 1）：主监听 + 每条「junction 在且目标在」的登记链接
  /// 各一条 target 监听（2026-10-02 junction 方向 b：spike 实测 RDC 不穿透 junction）；
  /// 随空间打开启停、链接拓扑变化后由 services::workspace 重建收
  pub workspace: Mutex<Vec<crate::adapters::inbox_watch::WatchHandle>>,
  /// 撤销 / 重做双栈（6.0）：会话级不持久化，库切换清空
  pub undo: Mutex<crate::core::undo::UndoManager>,
  /// 工作空间爬盘搜索任务取消标记（2026-10-03 工作空间搜索回退层）：task_id → 置位即取消，
  /// 任务结束从表中移除（imports 同构）
  pub ws_searches: Mutex<std::collections::HashMap<u64, std::sync::Arc<std::sync::atomic::AtomicBool>>>,
  /// 语义分析任务控制（PRD 08-04：暂停 / 恢复 / 取消共享 worker 与 ffmpeg 标志）
  pub semantic_tasks: Mutex<std::collections::HashMap<String, std::sync::Arc<crate::services::semantic::control::AnalysisControl>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceInfo {
  pub name: String,
  pub path: String,
  /// 自动导入开关（空间级设置 space.json，2026-08-12 用户定）：关 = 不启动收件箱（不挂载盘符、不监听）
  pub auto_import: bool,
}

/// pub(crate)：server/mcp（AI 外接通道 get_current_library）同用
pub(crate) fn space_info(space: &Space) -> SpaceInfo {
  SpaceInfo { name: space.name.clone(), path: space.root.to_string_lossy().into_owned(), auto_import: space.auto_import }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
  pub name: String,
  pub path: String,
  pub asset_count: usize,
  pub folder_count: usize,
}

/// pub(crate)：server/mcp（AI 外接通道 get_current_library）同用
pub(crate) fn to_info(lib: &Library) -> LibraryInfo {
  LibraryInfo {
    name: lib.root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
    path: lib.root.to_string_lossy().into_owned(),
    asset_count: lib.index.assets.len(),
    folder_count: lib.index.folders.len(),
  }
}

/// 空间进入打开状态：更新应用状态 + 记录到全局配置最近列表 + 按空间级 autoImport 启停收件箱
/// （2026-08-12 用户定：inbox 空间级共用，盘符挂载恒定指向空间 inbox/、不随切库变化；
/// 关 = 防御性停止，换掉开着收件箱的旧空间时卸干净）+ 工作空间缓存版本迁移（2026-09-26
/// 有界命名切换：旧命名 thumbs/peaks 一次性清空，纯缓存可重建）+ 启停工作空间监听
/// （PRD 02 步骤 1：workspace/ 存在才监听，.anikit 缓存写回自激已滤）
fn activate_space(app: &AppHandle, state: &State<AppState>, space: Space) -> Result<SpaceInfo, String> {
  let info = space_info(&space);
  let root = space.root.clone();
  let auto_import = space.auto_import;
  *state.space.lock().map_err(|e| e.to_string())? = Some(space);
  config::record_space_open(&info.name, &info.path).map_err(|e| e.to_string())?;
  if auto_import {
    crate::services::inbox::start(app, &root);
  } else {
    crate::services::inbox::stop(app);
  }
  crate::services::workspace::migrate_cache(&root);
  crate::services::workspace::start_watch(app, &root);
  // 空间文件夹品牌图标（2026-10-03 用户定，Eagle .library 同款）：创建 / 打开同覆盖，
  // 现有空间惰性补写；微秒级 IO 同步可接受（migrate_cache 同径），失败内部静默
  crate::adapters::space_icon::apply(&root);
  Ok(info)
}

/// 库进入打开状态：更新应用状态 + 记录到全局配置最近列表（切库不动收件箱——空间级共用）；
/// 库切换停止全部帧流会话（会话绑定旧库 assets 池）与撤销双栈（6.0：会话级不持久化，
/// 栈内快照绑定旧库路径）。开库自检扫出无主资产 → emit 前端弹警告窗（2026-09-02）
fn activate(app: &AppHandle, state: &State<AppState>, lib: Library) -> Result<LibraryInfo, String> {
  crate::services::frame_stream::stop_all(); // 帧流会话绑定旧库 assets 池（阶段四）
  crate::services::anim_preview_queue::clear_pending(); // 小动画生成任务同绑旧库 assets 池
  state.undo.lock().map_err(|e| e.to_string())?.clear();
  if !lib.orphan_infos.is_empty() {
    let _ = app.emit("library-orphan-infos", &lib.orphan_infos);
  }
  let info = to_info(&lib);
  *state.library.lock().map_err(|e| e.to_string())? = Some(lib);
  config::record_library_open(&info.name, &info.path).map_err(|e| e.to_string())?;
  Ok(info)
}

/// 取当前空间（短锁克隆）；未打开空间给中文错
fn current_space(state: &State<AppState>) -> Result<Space, String> {
  state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())
}

/// 在选定位置创建储存空间（前端系统对话框选定父目录后调用；文件夹名即空间名）。
/// 同名位置已是合法空间则视为打开（PRD 01_储存空间-3.1）。auto_import 写 space.json（空间级设置）
#[tauri::command]
pub async fn create_space(
  app: AppHandle,
  state: State<'_, AppState>,
  parent_path: String,
  name: String,
  auto_import: bool,
) -> Result<SpaceInfo, String> {
  let space = space::create(Path::new(&parent_path), &name, auto_import).map_err(|e| e.to_string())?;
  activate_space(&app, &state, space)
}

/// 打开储存空间（打开前校验 space.json，不是 AniKit 储存空间的文件夹会被拒绝）
#[tauri::command]
pub async fn open_space(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<SpaceInfo, String> {
  let space = space::open(Path::new(&path)).map_err(|e| e.to_string())?;
  activate_space(&app, &state, space)
}

/// 当前打开的储存空间；未打开返回 null（前端据此决定是否进入初次打开向导）
#[tauri::command]
pub fn get_current_space(state: State<AppState>) -> Option<SpaceInfo> {
  state.space.lock().ok()?.as_ref().map(space_info)
}

/// 当前空间内的资源库名列表，按 space.json libraryOrder 排序返回（3.2 拖动排序持久化；
/// 不在 order 内的外来目录排尾、按名字典序）；无当前空间返回空列表
#[tauri::command]
pub async fn list_space_libraries(state: State<'_, AppState>) -> Result<Vec<String>, String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone();
  match space {
    Some(space) => space::ordered_libraries(&space).map_err(|e| e.to_string()),
    None => Ok(Vec::new()),
  }
}

/// 在当前储存空间内创建资源库（文件夹名即资源库名）；新库名自动排尾进 libraryOrder（3.2）
#[tauri::command]
pub async fn create_library(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<LibraryInfo, String> {
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  let lib = library::create(space, &name).map_err(|e| e.to_string())?;
  drop(guard); // activate 走 AppState 其他锁，先放空间锁（与 set_auto_import 同风格）
  activate(&app, &state, lib)
}

/// 在当前储存空间内按名打开资源库（打开前校验结构；旧版库结构明确报错引导新建）
#[tauri::command]
pub async fn open_library(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<LibraryInfo, String> {
  let space = current_space(&state)?;
  let lib = library::open(&space.root, &space.root.join(&name)).map_err(|e| e.to_string())?;
  activate(&app, &state, lib)
}

/// 当前打开的素材库；未打开返回 null
#[tauri::command]
pub fn get_current_library(state: State<AppState>) -> Option<LibraryInfo> {
  state.library.lock().ok()?.as_ref().map(to_info)
}

/// 冷启动 deep-link 消费（2026-10-08 deep-link 复活）：anikit:// 协议链接拉起进程时 setup 暂存的
/// 待跳转链接，前端 initMediaOpenListener 注册后 take 一次（一次性消费；库未就绪由前端等 placements）
#[tauri::command]
pub fn take_pending_deep_link() -> Option<(String, String)> {
  crate::adapters::deep_link::take_pending()
}

/// 按磁盘文件名（originName）解析最新一笔资产 ID（2026-10-23 编辑器贴图直链 by-name 深链消费，
/// PRD 02-09）：前端拿到 anikit://item/by-name/{文件名} 后调本命令换 ID，再走既有 item 跳转定位；
/// 库未打开 / 解析不到（尚未导入或已删除）返回 null
#[tauri::command]
pub fn resolve_asset_by_origin_name(state: State<AppState>, name: String) -> Result<Option<String>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let Some(lib) = guard.as_ref() else {
    return Ok(None);
  };
  Ok(lib.index.latest_by_origin_name(&name, &lib.recycle).map(|m| m.id.clone()))
}

/// 开关自动导入（2.1：向导第二步初值 / 快速访问"自动导入"确认弹窗开启）：
/// 空间级设置（space.json 单写点，2026-08-12 用户定：inbox 空间级共用）；
/// 开 = 启动收件箱（盘符挂载 + 监听空间 inbox/），关 = 停止收件箱（卸载盘符）
#[tauri::command]
pub async fn set_auto_import(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<SpaceInfo, String> {
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  space.auto_import = enabled;
  space::save_space(space).map_err(|e| e.to_string())?;
  let info = space_info(space);
  let root = space.root.clone();
  drop(guard); // 收件箱启停走 AppState 锁，先放空间锁防交叉
  if enabled {
    crate::services::inbox::start(&app, &root);
  } else {
    crate::services::inbox::stop(&app);
  }
  Ok(info)
}

/// 全局配置中的资源库最近列表
#[tauri::command]
pub fn list_recent_libraries() -> Result<Vec<LibraryEntry>, String> {
  config::load().map(|c| c.libraries).map_err(|e| e.to_string())
}

/// 持久化库列表自定义顺序（3.2 菜单抓手拖动排序）：order 必须与空间内实际库名集合一致，
/// 多 / 少都报中文错；落盘后返回按新顺序排好的最新库名列表
#[tauri::command]
pub async fn reorder_libraries(state: State<'_, AppState>, order: Vec<String>) -> Result<Vec<String>, String> {
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  space::reorder_libraries(space, order).map_err(|e| e.to_string())
}

/// 统计库内孤儿素材数（2026-08-12 定案：删库确认弹窗提示"仅被该库引用的素材"数量用）。
/// 孤儿 = 仅被该库引用的资产（引用口径见 core::space::library_orphans）；库不存在报中文错
#[tauri::command]
pub async fn count_library_orphans(state: State<'_, AppState>, name: String) -> Result<u64, String> {
  apply_count_orphans(&state, &name)
}

/// 统计实现（命令层薄壳，便于单测）：编排在命令层，孤儿判定在 core::space
fn apply_count_orphans(state: &AppState, name: &str) -> Result<u64, String> {
  let space =
    state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  let orphans = space::library_orphans(&space.root, name).map_err(|e| e.to_string())?;
  Ok(orphans.len() as u64)
}

/// 无主资产「在资源管理器中打开」（OrphanInfosAlert 行尾按钮，2026-10-13 修复：原复用工作空间
/// reveal_path，其校验口径 = 当前空间 workspace 内，assets 池路径必败「路径不在当前工作空间内」）：
/// 校验基准 = 当前空间 assets 池内 + 存在，再 shell_open::reveal
#[tauri::command]
pub fn reveal_orphan_info(state: State<AppState>, abs_path: String) -> Result<(), String> {
  crate::adapters::shell_open::reveal(&validated_pool_path(&state, &abs_path)?)
}

/// 按路径校验（无主资产打开）：当前空间 assets 池内 + 存在
/// （commands::workspace::validated_ws_path 同构，校验基准从 workspace 根换成空间级素材池）
fn validated_pool_path(state: &AppState, abs_path: &str) -> Result<std::path::PathBuf, String> {
  let space =
    state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  let pool = paths::assets_dir(&space.root);
  let target = std::path::PathBuf::from(abs_path);
  if !crate::services::workspace::is_under(&pool, &target) {
    return Err("路径不在素材池内".to_string());
  }
  if !target.exists() {
    return Err("条目已不存在（可能刚被移动或删除）".to_string());
  }
  Ok(target)
}

/// 删除资源库（3.2 删除×；确认弹窗在前端）：返回删除后剩余库名列表（按 libraryOrder 序），
/// 前端据此决定切到剩余第一个库或回创建流程。delete_orphans = true 时一并删除仅被该库引用
/// 的孤儿素材实体（2026-08-12 定案：统一 assets 池默认保留，可选清理）
#[tauri::command]
pub async fn delete_library(
  _app: AppHandle,
  state: State<'_, AppState>,
  name: String,
  delete_orphans: bool,
) -> Result<Vec<String>, String> {
  apply_delete_library(&state, &name, delete_orphans)
}

/// 删除资源库的实现（命令层薄壳，便于单测）：删的是当前打开的库时先做关库收尾——
/// 清动图解包队列 pending、清撤销双栈、state.library 置 None（收件箱不动，空间级共用）；
/// 再整目录递归删除。库内数据文件删除不可恢复；素材实体默认保留在空间级 assets 池，
/// delete_orphans = true 时孤儿素材实体的 info 文件夹一并从 assets 池递归删除
fn apply_delete_library(state: &AppState, name: &str, delete_orphans: bool) -> Result<Vec<String>, String> {
  let is_current = state
    .library
    .lock()
    .map_err(|e| e.to_string())?
    .as_ref()
    .and_then(|l| l.root.file_name().map(|n| n.to_string_lossy().into_owned()))
    .as_deref()
    == Some(name);
  if is_current {
    crate::services::frame_stream::stop_all();
    crate::services::anim_preview_queue::clear_pending();
    state.undo.lock().map_err(|e| e.to_string())?.clear();
    *state.library.lock().map_err(|e| e.to_string())? = None;
  }
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  space::delete_library(space, name, delete_orphans).map_err(|e| e.to_string())
}

/// 合并资源库弹窗候选行（2026-09-26 功能轮，PRD 01_储存空间 3.2）：库名 + 容量
///（容量 = 引用资产集 metadata.size 求和，core::space::library_size）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeCandidate {
  pub name: String,
  pub size_bytes: u64,
}

/// 合并资源库弹窗列表：当前空间内**全部**资源库（libraryOrder 序，含当前打开库——2026-09-26
/// 二轮用户定：同时展示所有库，勾选两个以上合并）+ 各库容量；容量读取失败按 0 计不阻塞列表；
/// 无当前空间返回空列表
#[tauri::command]
pub async fn list_merge_candidates(state: State<'_, AppState>) -> Result<Vec<MergeCandidate>, String> {
  let space = current_space(&state)?;
  let names = space::ordered_libraries(&space).map_err(|e| e.to_string())?;
  Ok(names
    .into_iter()
    .map(|n| MergeCandidate { size_bytes: space::library_size(&space.root, &n).unwrap_or(0), name: n })
    .collect())
}

/// 合并资源库（2026-09-26 功能轮 + 二轮交互修订，PRD 01_储存空间 3.2）：names = 弹窗勾选集
///（≥2，含当前库与否均可），**目标库 = 勾选集中 libraryOrder 最靠前者**，其余并入它；
/// 合并完成即打开目标库（目标非当前库时表现为切库，经 activate 统一出口：停帧流 / 清撤销 /
/// 记最近 / 无主资产 emit）。返回目标库最新 LibraryInfo
#[tauri::command]
pub async fn merge_libraries(app: AppHandle, state: State<'_, AppState>, names: Vec<String>) -> Result<LibraryInfo, String> {
  let lib = apply_merge_libraries(&state, &names)?;
  activate(&app, &state, lib)
}

/// 合并编排（命令层薄壳便于单测；activate 出口在命令层）：校验（≥2 / 都存在）→ 定目标
///（libraryOrder 最靠前勾选者）→ 目标库就位（当前库即目标 = 取出直接用；否则打开目标——
/// 库全部写操作即时落盘，源库虽可能正开着也只是内存态 == 磁盘态，合并读盘无分叉）→
/// 备份门禁 → 逐源并入 + 封面迁移 → 单写点落盘（失败 reload 对齐盘面）→ 索引重建 →
/// relations.json 端点改写 → 删源目录 + libraryOrder 更新（删除失败容忍记日志，残留可再删）→
/// 清撤销栈 → 返回目标库（交 activate 收尾）。
/// 失败路径：目标是当前库时 reload 后放回 state（不留 None 空档）；非当前库直接丢
///（未激活无副作用，源目录此时均未动）
fn apply_merge_libraries(state: &AppState, names: &[String]) -> Result<Library, String> {
  let mut seen = HashSet::new();
  let names: Vec<&str> = names.iter().map(|s| s.trim()).filter(|s| !s.is_empty() && seen.insert(*s)).collect();
  if names.len() < 2 {
    return Err("请勾选两个以上要合并的资源库".to_string());
  }
  // 目标 = 勾选集中 libraryOrder 最靠前者
  let (space_root, ordered) = {
    let guard = state.space.lock().map_err(|e| e.to_string())?;
    let space = guard.as_ref().ok_or_else(|| "请先打开储存空间".to_string())?;
    (space.root.clone(), space::ordered_libraries(space).map_err(|e| e.to_string())?)
  };
  for n in &names {
    if !ordered.iter().any(|o| o == n) {
      return Err(format!("资源库「{n}」不存在"));
    }
  }
  let target_name = ordered.iter().find(|o| names.contains(&o.as_str())).expect("已校验全部存在").clone();
  let sources: Vec<&str> = names.iter().copied().filter(|n| *n != target_name).collect();

  // 目标库就位
  let current_name = state
    .library
    .lock()
    .map_err(|e| e.to_string())?
    .as_ref()
    .and_then(|l| l.root.file_name().map(|n| n.to_string_lossy().into_owned()));
  let target_is_current = current_name.as_deref() == Some(target_name.as_str());
  let mut lib = if target_is_current {
    state.library.lock().map_err(|e| e.to_string())?.take().expect("current_name 来自同一状态")
  } else {
    library::open(&space_root, &space_root.join(&target_name)).map_err(|e| e.to_string())?
  };
  let outcomes = match merge_sources_into(&mut lib, &space_root, &sources) {
    Ok(o) => o,
    Err(e) => {
      if target_is_current {
        let _ = library::reload(&mut lib); // 盘面对齐（未动盘时为重读，无妨）
        *state.library.lock().map_err(|e| e.to_string())? = Some(lib);
      }
      return Err(e);
    }
  };

  // relations.json 端点改写（空间级，core::relations 单写点）：指向源库的端点 lib 改目标库、
  // 档案端点 id 随重映射；读取 / 落盘失败记日志不阻断合并（同开库关系自检口径）
  match relations::load_relations(&space_root) {
    Ok(mut rels) => {
      let mut changed = false;
      for (n, outcome) in &outcomes {
        changed |= relations::rewrite_lib_refs(&mut rels, n, &target_name, &outcome.folder_id_map);
      }
      if changed {
        if let Err(e) = relations::save_relations(&space_root, &rels) {
          log::error!("合并改写 relations.json 落盘失败（数据文件保持原样）：{e}");
        }
      }
    }
    Err(e) => log::error!("合并改写 relations.json 读取失败（跳过）：{e}"),
  }

  // 删源库目录 + libraryOrder 更新（复用 delete_library；delete_orphans = false——
  // 引用已随合并归目标库，无孤儿产生）
  {
    let mut guard = state.space.lock().map_err(|e| e.to_string())?;
    let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
    for (n, _) in &outcomes {
      if let Err(e) = space::delete_library(space, n, false) {
        log::error!("合并后删除源库目录失败（数据已并入目标库，残留可再删）：{n}：{e}");
      }
    }
  }
  // 清撤销栈：栈内快照绑定合并前数据文件，撤销会把并入内容从盘上抹掉（同删当前库口径）
  state.undo.lock().map_err(|e| e.to_string())?.clear();
  for (n, outcome) in &outcomes {
    log::info!(
      "资源库「{n}」已并入「{target_name}」：文件夹 +{} / Placement +{}（重复跳 {}）/ 回收站 +{} / origin 降级 {} / 封面迁移 {}",
      outcome.folders,
      outcome.placements,
      outcome.skipped_dup,
      outcome.recycle,
      outcome.demoted_origins,
      outcome.covers.len()
    );
  }
  Ok(lib)
}

/// 逐源并入（apply_merge_libraries 的数据段）：备份门禁 → 读源 → 纯数据并入（core::merge）+
/// 封面迁移 → 五文件单写点落盘（tags 不并——体系已取消且无写点；任一失败 reload 对齐盘面
/// 后报错）→ 内存索引重建。返回各源 outcome
fn merge_sources_into(lib: &mut Library, space_root: &Path, sources: &[&str]) -> Result<Vec<(String, merge::MergeOutcome)>, String> {
  // 备份门禁：合并改写目标库数据文件 + 删源目录，先把目标库备一份；失败不继续（同开库自检口径。
  // 源库不备——其数据（文件夹树 / Placement / 回收站 / 档案）随并入全部活进目标库，无额外所失）
  backup::backup_library(&lib.root, &lib.assets_dir).map_err(|e| format!("合并前备份失败，已取消合并：{e}"))?;
  let mut outcomes: Vec<(String, merge::MergeOutcome)> = Vec::new();
  for n in sources {
    let src_dir = space_root.join(n);
    let source = merge::read_source(&src_dir).map_err(|e| format!("读取资源库「{n}」失败：{e}"))?;
    let mut outcome = {
      let mut t = merge::MergeTarget {
        folders: &mut lib.folders,
        pins: &mut lib.pins,
        recycle: &mut lib.recycle,
        recent: &mut lib.recent,
        archives: &mut lib.archives,
      };
      merge::merge_into(&mut t, source)
    };
    migrate_covers(lib, &src_dir, &mut outcome);
    outcomes.push((n.to_string(), outcome));
  }
  // 落盘（单写点逐一原子写；任一失败 → reload 与盘面对齐后报错）
  if let Err(e) = (|| -> Result<(), DataError> {
    library::save_folders(lib)?;
    library::save_recycle(lib)?;
    library::save_recent(lib)?;
    library::save_pins(lib)?;
    library::save_archives(lib)
  })() {
    let _ = library::reload(lib);
    return Err(format!("合并数据保存失败：{e}"));
  }
  // 索引重建：assets 池不动（合并不迁实体），文件夹表 + Placement 反查重建
  lib.index.folders = lib.folders.folders.iter().map(|f| (f.id.clone(), f.clone())).collect();
  lib.index.rebuild_placements(&lib.folders.items);
  Ok(outcomes)
}

/// 重命名资源库（2026-09-26 用户定：合并命名失误的补救口 + 库管理基础能力）：库目录改名 +
/// libraryOrder 同步（core::space::rename_library）；relations.json 端点 lib 名改写（档案 id
/// 不动——folder 不随库改名）；全局配置最近列表同步（按旧 path 匹配，失败仅记日志——改名
/// 已生效不回滚）；改名的是当前打开库时内存 root 同步更新，返回其最新 LibraryInfo，否则 null
#[tauri::command]
pub async fn rename_library(state: State<'_, AppState>, name: String, new_name: String) -> Result<Option<LibraryInfo>, String> {
  let (info, old_path, new_path) = apply_rename_library(&state, &name, &new_name)?;
  if let Err(e) = config::rename_library_entry(&old_path, new_name.trim(), &new_path) {
    log::error!("改名后同步全局配置最近列表失败（不影响改名）：{e}");
  }
  Ok(info)
}

/// 改名编排（命令层薄壳便于单测；config 最近列表同步在命令层——保持测试不碰真实用户配置）：
/// 返回（当前库改名时其最新 LibraryInfo，否则 None；旧路径；新路径）
fn apply_rename_library(state: &AppState, name: &str, new_name: &str) -> Result<(Option<LibraryInfo>, String, String), String> {
  let new_name = new_name.trim();
  let is_current = state
    .library
    .lock()
    .map_err(|e| e.to_string())?
    .as_ref()
    .and_then(|l| l.root.file_name().map(|n| n.to_string_lossy().into_owned()))
    .as_deref()
    == Some(name);
  let (space_root, old_path, new_path) = {
    let mut guard = state.space.lock().map_err(|e| e.to_string())?;
    let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
    space::rename_library(space, name, new_name).map_err(|e| e.to_string())?;
    (
      space.root.clone(),
      space.root.join(name).to_string_lossy().into_owned(),
      space.root.join(new_name).to_string_lossy().into_owned(),
    )
  };
  // relations.json 端点 lib 名改写（folder_map 空——档案 id 不随库改名变化）
  match relations::load_relations(&space_root) {
    Ok(mut rels) => {
      if relations::rewrite_lib_refs(&mut rels, name, new_name, &std::collections::HashMap::new()) {
        if let Err(e) = relations::save_relations(&space_root, &rels) {
          log::error!("改名改写 relations.json 落盘失败（数据文件保持原样）：{e}");
        }
      }
    }
    Err(e) => log::error!("改名改写 relations.json 读取失败（跳过）：{e}"),
  }
  // 当前库改名：内存 root 同步（全部路径经 lib.root / covers_dir 等现取，无缓存绝对路径）
  if is_current {
    let mut guard = state.library.lock().map_err(|e| e.to_string())?;
    if let Some(lib) = guard.as_mut() {
      lib.root = std::path::PathBuf::from(&new_path);
    }
    return Ok((guard.as_ref().map(to_info), old_path, new_path));
  }
  Ok((None, old_path, new_path))
}

/// 封面迁移（合并编排段，目标库落盘前执行）：并入档案引用的封面文件自源库 cache/covers/
/// 拷贝进目标库 covers/；目标撞名（仅"原名保留"路径理论可撞）唯一化并回写档案记录
/// avatar.file。源文件缺失 / 拷贝失败只记日志——avatar 引用保留，展示侧按无封面回退
///（cover_file_info 文件缺失视为无封面口径）
fn migrate_covers(lib: &mut Library, src_dir: &Path, outcome: &mut merge::MergeOutcome) {
  if outcome.covers.is_empty() {
    return;
  }
  let dst_dir = library::covers_dir(&lib.root);
  if let Err(e) = std::fs::create_dir_all(&dst_dir) {
    log::error!("合并封面迁移失败（目标目录创建失败 {}）：{e}", dst_dir.display());
    return;
  }
  for mv in &outcome.covers {
    let src = library::covers_dir(src_dir).join(&mv.from);
    if !src.is_file() {
      log::warn!("合并封面迁移跳过（源文件缺失）：{}", src.display());
      continue;
    }
    let mut name = mv.to.clone();
    if dst_dir.join(&name).exists() {
      let existing: HashSet<String> = std::fs::read_dir(&dst_dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
      name = library::unique_name(&existing, &name);
      if let Some(rec) = lib.archives.archives.iter_mut().find(|r| r.folder_id == mv.folder_id) {
        if let Some(av) = rec.avatar.as_mut() {
          av.file = name.clone();
        }
      }
    }
    if let Err(e) = std::fs::copy(&src, dst_dir.join(&name)) {
      log::error!("合并封面迁移失败（{} → {}）：{e}", src.display(), name);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 建测试 AppState：空间 + 两个库（库A 为当前打开），返回 (临时目录, AppState)
  fn make_state() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut space = space::create(dir.path(), "空间", false).unwrap();
    let lib_a = library::create(&mut space, "库A").unwrap();
    library::create(&mut space, "库B").unwrap();
    let state = AppState {
      space: Mutex::new(Some(space)),
      library: Mutex::new(Some(lib_a)),
      imports: Mutex::new(std::collections::HashMap::new()),
      inbox: Mutex::new(None),
      workspace: Mutex::new(Vec::new()),
      undo: Mutex::new(crate::core::undo::UndoManager::default()),
      ws_searches: Mutex::new(std::collections::HashMap::new()),
      semantic_tasks: Mutex::new(std::collections::HashMap::new()),
    };
    (dir, state)
  }

  /// 删当前打开的库：先关库收尾（解包队列 / 撤销栈 / library 置 None），目录消失，返回剩余列表
  #[test]
  fn delete_current_library_closes_state() {
    let (_dir, state) = make_state();
    // 撤销栈里压一条，验证清栈
    {
      let mut undo = state.undo.lock().unwrap();
      undo.begin("测试事务");
      undo.commit().unwrap();
    }
    assert!(state.undo.lock().unwrap().state().can_undo, "删除前撤销栈非空");

    let remaining = apply_delete_library(&state, "库A", false).unwrap();
    assert_eq!(remaining, vec!["库B"]);
    assert!(state.library.lock().unwrap().is_none(), "删当前库后 state.library 应置 None");
    assert!(!state.undo.lock().unwrap().state().can_undo, "删当前库应清撤销栈");
    let space = state.space.lock().unwrap();
    let space = space.as_ref().unwrap();
    assert!(!space.root.join("库A").exists(), "库目录应整体消失");
    assert_eq!(space.library_order, vec!["库B"], "order 应移除被删库");
    assert!(space.root.join("库B").is_dir(), "其余库不动");
  }

  /// 删非当前库：state.library 不动
  #[test]
  fn delete_other_library_keeps_current() {
    let (_dir, state) = make_state();
    let remaining = apply_delete_library(&state, "库B", false).unwrap();
    assert_eq!(remaining, vec!["库A"]);
    assert!(state.library.lock().unwrap().is_some(), "删非当前库不应动 state.library");
  }

  /// 删不存在的库：中文报错，状态不动
  #[test]
  fn delete_missing_library_errors() {
    let (_dir, state) = make_state();
    assert!(apply_delete_library(&state, "不存在", false).is_err());
    assert!(state.library.lock().unwrap().is_some(), "删除失败不应动 state.library");
  }

  /// 造库引用关系辅助：写 folders.json items + recycle.json items，并在 assets 池建对应 info
  /// （含原始文件，验证删除是递归的）。返回 assets 池路径便于断言
  fn seed_library(
    state: &AppState,
    lib: &str,
    folder_items: &[&str],
    recycle_items: &[&str],
  ) -> std::path::PathBuf {
    use crate::core::models::*;
    use crate::core::paths;
    let guard = state.space.lock().unwrap();
    let space = guard.as_ref().unwrap();
    let lib_dir = space.root.join(lib);
    let folders = FoldersFile {
      version: DATA_VERSION,
      blocks: Vec::new(),
      folders: vec![Folder { id: "F01".into(), name: "夹".into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 }],
      items: folder_items
        .iter()
        .map(|id| Placement { asset: id.to_string(), folder: "F01".into(), name: format!("{id}.fla"), order: 0, origin: true })
        .collect(),
    };
    crate::core::json_io::write_json(&lib_dir.join(crate::core::library::FOLDERS_FILE), &folders).unwrap();
    let recycle = RecycleFile {
      version: DATA_VERSION,
      items: recycle_items
        .iter()
        .map(|id| RecycleItem { asset: id.to_string(), placements: Vec::new(), deleted_at: "2026-08-12T00:00:00+08:00".into() })
        .collect(),
    };
    crate::core::json_io::write_json(&lib_dir.join(crate::core::library::RECYCLE_FILE), &recycle).unwrap();
    let assets = paths::assets_dir(&space.root);
    for id in folder_items.iter().chain(recycle_items) {
      let info = paths::info_dir(&assets, id);
      std::fs::create_dir_all(&info).unwrap();
      std::fs::write(info.join("原始.fla"), b"x").unwrap();
    }
    assets
  }

  /// 孤儿清理（2026-08-12 定案）：delete_orphans=true 删库后，共享资产 info 保留、
  /// 仅被该库引用的孤儿 info 从 assets 池递归删除
  #[test]
  fn delete_orphans_removes_unique_keeps_shared() {
    let (_dir, state) = make_state();
    let assets = seed_library(&state, "库A", &["SHARED", "ONLY-A"], &[]);
    seed_library(&state, "库B", &["SHARED"], &[]);

    let remaining = apply_delete_library(&state, "库A", true).unwrap();
    assert_eq!(remaining, vec!["库B"]);
    assert!(!crate::core::paths::info_dir(&assets, "ONLY-A").exists(), "孤儿 info 应递归删除");
    assert!(crate::core::paths::info_dir(&assets, "SHARED").is_dir(), "共享 info 应保留");
  }

  /// delete_orphans=false：保持现状，assets 池一概不动
  #[test]
  fn delete_without_orphans_keeps_all_info() {
    let (_dir, state) = make_state();
    let assets = seed_library(&state, "库A", &["SHARED", "ONLY-A"], &[]);
    seed_library(&state, "库B", &["SHARED"], &[]);

    apply_delete_library(&state, "库A", false).unwrap();
    assert!(crate::core::paths::info_dir(&assets, "ONLY-A").is_dir(), "false 时孤儿 info 也不动");
    assert!(crate::core::paths::info_dir(&assets, "SHARED").is_dir());
  }

  /// 回收站算引用：资产仅在被删库的 folders，但在另一库 recycle → 保留；
  /// 被删库自己 recycle 里独有的 → 孤儿，一并删除
  #[test]
  fn recycle_reference_counts_as_reference() {
    let (_dir, state) = make_state();
    let assets = seed_library(&state, "库A", &["IN-OTHER-RECYCLE"], &["OWN-RECYCLE-ONLY"]);
    seed_library(&state, "库B", &[], &["IN-OTHER-RECYCLE"]);

    assert_eq!(apply_count_orphans(&state, "库A").unwrap(), 1, "他库回收站引用的不算孤儿");
    apply_delete_library(&state, "库A", true).unwrap();
    assert!(crate::core::paths::info_dir(&assets, "IN-OTHER-RECYCLE").is_dir(), "另一库回收站引用应保留");
    assert!(!crate::core::paths::info_dir(&assets, "OWN-RECYCLE-ONLY").exists(), "本库回收站独有应删除");
  }

  /// 孤儿计数：差集口径（folders ∪ recycle − 其余库 folders ∪ recycle 并集）
  #[test]
  fn count_orphans_matches_difference_set() {
    let (_dir, state) = make_state();
    seed_library(&state, "库A", &["SHARED", "ONLY-1", "ONLY-2"], &["ONLY-3"]);
    seed_library(&state, "库B", &["SHARED"], &[]);

    assert_eq!(apply_count_orphans(&state, "库A").unwrap(), 3);
    assert_eq!(apply_count_orphans(&state, "库B").unwrap(), 0, "库B 引用全是共享，无孤儿");
    // 库不存在报中文错
    let err = apply_count_orphans(&state, "不存在").unwrap_err();
    assert!(err.contains("资源库"), "应报中文错：{err}");
  }

  /// 无主资产打开校验（2026-10-13）：池内现存 info 放行；workspace 内 / 池内不存在 / `..` 逃逸三拒
  /// （原复用工作空间 reveal_path 对 assets 池路径必败「不在当前工作空间内」的回归钉）
  #[test]
  fn pool_path_validation_accepts_info_rejects_others() {
    let (_dir, state) = make_state();
    let assets = seed_library(&state, "库A", &["AN-ORPHAN"], &[]);
    let info = crate::core::paths::info_dir(&assets, "AN-ORPHAN");
    assert!(validated_pool_path(&state, &info.to_string_lossy()).is_ok(), "池内现存 info 应放行");

    let ws_root = state.space.lock().unwrap().as_ref().unwrap().root.join(crate::core::space::WORKSPACE_DIR);
    let err = validated_pool_path(&state, &ws_root.to_string_lossy()).unwrap_err();
    assert!(err.contains("素材池"), "workspace 路径应报素材池越界：{err}");

    let missing = assets.join("AN-GONE.info");
    let err = validated_pool_path(&state, &missing.to_string_lossy()).unwrap_err();
    assert!(err.contains("已不存在"), "池内不存在应报已不存在：{err}");

    let escape = assets.join("..").join("workspace");
    let err = validated_pool_path(&state, &escape.to_string_lossy()).unwrap_err();
    assert!(err.contains("素材池"), "`..` 逃逸应报素材池越界：{err}");
  }

  /// 合并端到端（2026-09-26）：源库五文件 + 档案封面 + 空间级 relations 并入当前库（落盘 + 索引），
  /// 源目录删除、libraryOrder 更新、撤销栈清空；返回目标库最新计数
  #[test]
  fn merge_libraries_end_to_end() {
    let (_dir, state) = make_state();
    let space_root = state.space.lock().unwrap().as_ref().unwrap().root.clone();
    let lib_b = space_root.join("库B");
    // 库B 数据：区块 + 两文件夹（其一档案）+ 两 Placement + 回收站 + 最近 + pins + 档案（带封面）
    std::fs::write(
      lib_b.join(crate::core::library::FOLDERS_FILE),
      r#"{
        "version": 1,
        "blocks": [ { "id": "dir", "name": "文件夹", "order": 0 }, { "id": "B-S", "name": "参考", "order": 1 } ],
        "folders": [
          { "id": "F-S", "name": "源夹", "parent": null, "block": "B-S", "order": 0 },
          { "id": "F-ARCH", "name": "雪牧", "parent": null, "kind": "archive", "order": 1 }
        ],
        "items": [
          { "asset": "AN-S", "folder": "F-S", "name": "来.fla", "order": 0, "origin": true },
          { "asset": "AN-A", "folder": "root", "name": "共.fla", "order": 1, "origin": true }
        ]
      }"#,
    )
    .unwrap();
    std::fs::write(
      lib_b.join(crate::core::library::RECYCLE_FILE),
      r#"{ "version": 1, "items": [ { "asset": "AN-R", "deletedAt": "2026-09-20T00:00:00+08:00",
        "placements": [ { "asset": "AN-R", "folder": "F-S", "name": "站.fla", "order": 0, "origin": true } ] } ] }"#,
    )
    .unwrap();
    std::fs::write(
      lib_b.join(crate::core::library::RECENT_FILE),
      r#"{ "version": 1, "items": [ { "asset": "AN-S", "viewedAt": "2026-09-25T00:00:00+08:00" } ] }"#,
    )
    .unwrap();
    std::fs::write(
      lib_b.join(crate::core::library::PINS_FILE),
      r#"{ "version": 1, "pins": [ { "target": "F-S", "order": 0 } ] }"#,
    )
    .unwrap();
    std::fs::write(
      lib_b.join(crate::core::library::ARCHIVES_FILE),
      r#"{ "version": 1, "archives": [ { "folderId": "F-ARCH", "avatar": { "file": "F-ARCH.webp" }, "props": ["主角"], "descriptions": [], "links": [] } ] }"#,
    )
    .unwrap();
    let covers_b = lib_b.join("cache").join("covers");
    std::fs::create_dir_all(&covers_b).unwrap();
    std::fs::write(covers_b.join("F-ARCH.webp"), b"cover").unwrap();
    // 空间级 relations：目标库 asset → 源库 archive
    crate::core::json_io::write_json(
      &space_root.join(crate::core::relations::RELATIONS_FILE),
      &serde_json::json!({ "version": 1, "relations": [ {
        "id": "RL-1", "kind": "free", "label": "参考",
        "from": { "lib": "库A", "kind": "asset", "id": "AN-T", "name": "在" },
        "to": { "lib": "库B", "kind": "archive", "id": "F-ARCH", "name": "雪牧" }
      } ] }),
    )
    .unwrap();
    // 撤销栈压一条，验证合并清栈
    {
      let mut undo = state.undo.lock().unwrap();
      undo.begin("测试事务");
      undo.commit().unwrap();
    }
    assert!(state.undo.lock().unwrap().state().can_undo);

    let lib = apply_merge_libraries(&state, &["库A".to_string(), "库B".to_string()]).unwrap();
    assert_eq!(lib.root.file_name().unwrap().to_string_lossy(), "库A", "勾选集含当前库且 order 最前 = 目标");
    assert_eq!(lib.index.folders.len(), 2, "库A 原 0 文件夹 + 源 2");
    assert!(state.library.lock().unwrap().is_none(), "当前库被取出、交命令层 activate 放回");
    assert!(!state.undo.lock().unwrap().state().can_undo, "合并清撤销栈");
    assert!(!lib_b.exists(), "源库目录删除");
    assert_eq!(state.space.lock().unwrap().as_ref().unwrap().library_order, vec!["库A"], "order 移除源库");

    // 落盘核对：目标库 folders.json 含并入内容（区块 dir 唯一 + B-S、Placement 全在、origin 保留）
    let lib_a = space_root.join("库A");
    let folders: crate::core::models::FoldersFile =
      crate::core::json_io::read_json(&lib_a.join(crate::core::library::FOLDERS_FILE)).unwrap();
    assert_eq!(folders.blocks.len(), 2, "默认区块不重复并入");
    assert!(folders.folders.iter().any(|f| f.id == "F-S" && f.block.as_deref() == Some("B-S")));
    assert_eq!(folders.items.len(), 2);
    assert!(folders.items.iter().all(|p| p.origin), "目标无同名资产，源 origin 全保留");
    // 回收站 / 最近 / pins / 档案 + 封面
    let recycle: crate::core::models::RecycleFile =
      crate::core::json_io::read_json(&lib_a.join(crate::core::library::RECYCLE_FILE)).unwrap();
    assert_eq!(recycle.items.len(), 1);
    assert_eq!(recycle.items[0].placements[0].folder, "F-S", "回收站快照 folder 引用保留（未撞 id）");
    let recent: crate::core::models::RecentFile =
      crate::core::json_io::read_json(&lib_a.join(crate::core::library::RECENT_FILE)).unwrap();
    assert_eq!(recent.items.len(), 1);
    let pins: crate::core::models::PinsFile =
      crate::core::json_io::read_json(&lib_a.join(crate::core::library::PINS_FILE)).unwrap();
    assert_eq!(pins.pins[0].target, "F-S");
    let archives: crate::core::models::ArchivesFile =
      crate::core::json_io::read_json(&lib_a.join(crate::core::library::ARCHIVES_FILE)).unwrap();
    assert_eq!(archives.archives[0].folder_id, "F-ARCH");
    assert_eq!(archives.archives[0].avatar.as_ref().unwrap().file, "F-ARCH.webp");
    assert!(
      crate::core::library::covers_dir(&lib_a).join("F-ARCH.webp").is_file(),
      "封面跨库拷贝到目标 covers/"
    );
    // relations 端点 lib 改写
    let rels: crate::core::models::RelationsFile =
      crate::core::json_io::read_json(&space_root.join(crate::core::relations::RELATIONS_FILE)).unwrap();
    assert_eq!(rels.relations[0].to.lib, "库A", "源库端点改写为目标库");
    assert_eq!(rels.relations[0].to.id, "F-ARCH", "未撞 id 不重映射");
    // 备份门禁：目标库合并前已留备份
    assert!(lib_a.join(crate::core::library::BACKUP_DIR).read_dir().unwrap().next().is_some(), "合并前备份目标库");
  }

  /// 合并校验（2026-09-26 二轮交互修订）：去重后不足两个 / 源库不存在 → 中文报错，状态不动
  ///（Library 无 Debug  derive，断言走 .err() 不走 unwrap_err）
  #[test]
  fn merge_libraries_validates_input() {
    let (_dir, state) = make_state();
    let err_of = |names: &[&str]| {
      apply_merge_libraries(&state, &names.iter().map(|s| s.to_string()).collect::<Vec<_>>()).err().expect("应报错")
    };
    assert!(err_of(&[]).contains("两个以上"));
    assert!(err_of(&["库A"]).contains("两个以上"));
    // 去重后不足两个同报
    assert!(err_of(&["库A", "库A"]).contains("两个以上"));
    let missing = err_of(&["库A", "不存在"]);
    assert!(missing.contains("不存在"), "{missing}");
    // 状态不动：库 A 仍开着、库 B 目录在、order 不变
    assert!(state.library.lock().unwrap().is_some());
    let space = state.space.lock().unwrap();
    let space = space.as_ref().unwrap();
    assert!(space.root.join("库B").is_dir());
    assert_eq!(space.library_order, vec!["库A", "库B"]);
  }

  /// 目标规则（2026-09-26 二轮）：目标 = 勾选集中 libraryOrder 最靠前者，不一定是当前库——
  /// order [库B, 库A] 时合并 → 库B 为目标，当前打开的库A 作为源并入并删目录；
  /// 返回的 Library 即库B（命令层 activate 后当前库切换为它）
  #[test]
  fn merge_target_is_order_first_not_necessarily_current() {
    let (_dir, state) = make_state();
    {
      let mut guard = state.space.lock().unwrap();
      let space = guard.as_mut().unwrap();
      space.library_order = vec!["库B".into(), "库A".into()];
      crate::core::space::save_space(space).unwrap();
    }
    // 当前库库A 造内容（直接写盘——全部写操作即时落盘，内存态 == 磁盘态）
    let space_root = state.space.lock().unwrap().as_ref().unwrap().root.clone();
    std::fs::write(
      space_root.join("库A").join(crate::core::library::FOLDERS_FILE),
      r#"{ "version": 1, "folders": [ { "id": "F-A", "name": "甲夹", "parent": null, "order": 0 } ],
        "items": [ { "asset": "AN-A", "folder": "F-A", "name": "甲.fla", "order": 0, "origin": true } ] }"#,
    )
    .unwrap();

    let lib = apply_merge_libraries(&state, &["库A".to_string(), "库B".to_string()]).unwrap();
    assert_eq!(lib.root.file_name().unwrap().to_string_lossy(), "库B", "order 最前的为目标库");
    assert!(lib.index.folders.contains_key("F-A"), "库A 内容并入库B");
    assert!(!space_root.join("库A").exists(), "源（当前库）目录删除");
    assert_eq!(state.space.lock().unwrap().as_ref().unwrap().library_order, vec!["库B"]);
    // 非当前目标不动 state.library（等 activate 切换）；盘上库B folders.json 一致
    assert!(state.library.lock().unwrap().is_some());
    let folders: crate::core::models::FoldersFile =
      crate::core::json_io::read_json(&space_root.join("库B").join(crate::core::library::FOLDERS_FILE)).unwrap();
    assert!(folders.folders.iter().any(|f| f.id == "F-A" && f.name == "甲夹"));
    assert!(folders.items.iter().any(|p| p.asset == "AN-A" && p.origin));
  }

  /// 封面撞名唯一化（legacy 原名保留路径才可能撞）：目标已有同名文件 → 唯一化并回写档案记录
  #[test]
  fn merge_cover_name_collision_uniqueified() {
    let (_dir, state) = make_state();
    let space_root = state.space.lock().unwrap().as_ref().unwrap().root.clone();
    let lib_a = space_root.join("库A");
    let lib_b = space_root.join("库B");
    std::fs::write(
      lib_b.join(crate::core::library::ARCHIVES_FILE),
      // 手编 legacy：avatar.file 不以 folderId 开头 → 原名保留路径
      r#"{ "version": 1, "archives": [ { "folderId": "F-B", "avatar": { "file": "随手.webp" }, "props": [], "descriptions": [], "links": [] } ] }"#,
    )
    .unwrap();
    std::fs::write(
      lib_b.join(crate::core::library::FOLDERS_FILE),
      r#"{ "version": 1, "folders": [ { "id": "F-B", "name": "档", "parent": null, "kind": "archive", "order": 0 } ], "items": [] }"#,
    )
    .unwrap();
    let covers_b = lib_b.join("cache").join("covers");
    std::fs::create_dir_all(&covers_b).unwrap();
    std::fs::write(covers_b.join("随手.webp"), b"new").unwrap();
    // 目标库已有同名封面（别的档案的）
    let covers_a = crate::core::library::covers_dir(&lib_a);
    std::fs::create_dir_all(&covers_a).unwrap();
    std::fs::write(covers_a.join("随手.webp"), b"old").unwrap();

    apply_merge_libraries(&state, &["库A".to_string(), "库B".to_string()]).unwrap();
    let archives: crate::core::models::ArchivesFile =
      crate::core::json_io::read_json(&lib_a.join(crate::core::library::ARCHIVES_FILE)).unwrap();
    assert_eq!(archives.archives[0].avatar.as_ref().unwrap().file, "随手_1.webp", "撞名唯一化并回写记录");
    assert_eq!(std::fs::read(covers_a.join("随手.webp")).unwrap(), b"old", "原有文件不被覆盖");
    assert_eq!(std::fs::read(covers_a.join("随手_1.webp")).unwrap(), b"new");
  }

  /// 重命名（2026-09-26）：非当前库 = 目录改名 + order 同步 + relations 端点 lib 改写；
  /// 当前库 = 内存 root 同步 + 返回最新 LibraryInfo（config 最近列表同步在命令层，测试不碰真实配置）
  #[test]
  fn rename_library_full_chain() {
    let (_dir, state) = make_state();
    let space_root = state.space.lock().unwrap().as_ref().unwrap().root.clone();
    crate::core::json_io::write_json(
      &space_root.join(crate::core::relations::RELATIONS_FILE),
      &serde_json::json!({ "version": 1, "relations": [ {
        "id": "RL-1", "kind": "free", "label": "参考",
        "from": { "lib": "库B", "kind": "asset", "id": "AN-1", "name": "一" },
        "to": { "lib": "库A", "kind": "asset", "id": "AN-2", "name": "二" }
      } ] }),
    )
    .unwrap();

    // 非当前库改名：库B → 素材库
    let (info, _, _) = apply_rename_library(&state, "库B", "素材库").unwrap();
    assert!(info.is_none(), "非当前库不返回 info");
    assert!(!space_root.join("库B").exists() && space_root.join("素材库").is_dir(), "目录已改");
    assert_eq!(state.space.lock().unwrap().as_ref().unwrap().library_order, vec!["库A", "素材库"]);
    let rels: crate::core::models::RelationsFile =
      crate::core::json_io::read_json(&space_root.join(crate::core::relations::RELATIONS_FILE)).unwrap();
    assert_eq!(rels.relations[0].from.lib, "素材库", "relations 端点 lib 改写");
    assert_eq!(rels.relations[0].to.lib, "库A", "他端不动");

    // 当前库改名：库A → 主库
    let (info, _, _) = apply_rename_library(&state, "库A", "主库").unwrap();
    let info = info.expect("当前库改名返回 info");
    assert_eq!(info.name, "主库");
    assert!(info.path.ends_with("主库"));
    let guard = state.library.lock().unwrap();
    assert_eq!(guard.as_ref().unwrap().root.file_name().unwrap().to_string_lossy(), "主库", "内存 root 同步");
    drop(guard);
    let rels: crate::core::models::RelationsFile =
      crate::core::json_io::read_json(&space_root.join(crate::core::relations::RELATIONS_FILE)).unwrap();
    assert_eq!(rels.relations[0].to.lib, "主库");
  }

  /// 改名校验：不存在库 / 撞名 / 非法名报错；同名 no-op；失败后状态不动
  #[test]
  fn rename_library_validates() {
    let (_dir, state) = make_state();
    assert!(apply_rename_library(&state, "不存在", "x").is_err());
    assert!(apply_rename_library(&state, "库B", "库A").is_err(), "撞既有库");
    assert!(apply_rename_library(&state, "库B", "bad:name").is_err(), "非法名");
    assert!(apply_rename_library(&state, "库B", "库B").is_ok(), "同名 no-op");
    let space = state.space.lock().unwrap();
    let space = space.as_ref().unwrap();
    assert!(space.root.join("库B").is_dir() && space.root.join("库A").is_dir(), "状态不动");
    assert_eq!(space.library_order, vec!["库A", "库B"]);
  }
}
