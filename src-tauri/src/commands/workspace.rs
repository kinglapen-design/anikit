//! 命令 API：索引式工作空间（PRD 02）——根信息 / 目录列举 / 缩略图请求（步骤 1 只读底座）
//! 与基础整理写操作（步骤 2：新建文件夹 / 重命名 / 回收站删除 / 移动 / 复制粘贴）。
//! 未开空间一律返回空（null / []）不报错（参照 list_space_libraries 先例）；写操作未开
//! 空间报错；业务在 services::workspace，本层只做取空间快照与参数传递。

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::commands::library::AppState;
use crate::adapters::shell_open;
use crate::core::space::WORKSPACE_DIR;
use crate::services::workspace::{self, WsEntry};

/// 工作空间根信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRootInfo {
  /// workspace/ 目录绝对路径（前端 convertFileSrc 直读其下文件）
  pub root_path: String,
  /// 目录是否存在（空间创建初期 / 目录被删时 false，前端据此显空态）
  pub exists: bool,
}

/// 当前空间的工作空间根；未开空间返回 null
#[tauri::command]
pub fn workspace_root(state: State<AppState>) -> Option<WorkspaceRootInfo> {
  let space = state.space.lock().ok()?.clone()?;
  let root = space.root.join(WORKSPACE_DIR);
  Some(WorkspaceRootInfo { root_path: root.to_string_lossy().into_owned(), exists: root.is_dir() })
}

/// 列举工作空间目录：relPath 空串 = 根；目录在前、同名不区分大小写排序；thumbPath 仅缓存新鲜时给。
/// 未开空间 / workspace 目录缺失 / 越界或读取失败 → Ok(空列表)（降级不报错，参照 list_space_libraries）。
/// 同步 IO + exe 图标的 COM 提取（SHELL_ICON_LOCK 进程级互斥）挪 spawn_blocking（2026-09-26
/// 性能优化 B）：大目录列举阻塞 tokio worker 会饿死同线程的帧流 WS 等异步任务
#[tauri::command]
pub async fn list_workspace_dir(state: State<'_, AppState>, rel_path: String) -> Result<Vec<WsEntry>, String> {
  let Some(space) = state.space.lock().map_err(|e| e.to_string())?.clone() else { return Ok(Vec::new()) };
  let root = space.root.join(WORKSPACE_DIR);
  if !root.is_dir() {
    return Ok(Vec::new());
  }
  tauri::async_runtime::spawn_blocking(move || {
    Ok(workspace::list_dir(&root, &rel_path).unwrap_or_else(|e| {
      log::warn!("工作空间目录列举失败（rel={rel_path}）：{e}");
      Vec::new()
    }))
  })
  .await
  .map_err(|e| format!("工作空间目录列举任务异常：{e}"))?
}

/// 在文件管理器中打开并选中（工作空间条目右键；按真实路径，约束在当前空间 workspace 内）
#[tauri::command]
pub fn reveal_path(state: State<AppState>, abs_path: String) -> Result<(), String> {
  shell_open::reveal(&validated_ws_path(&state, &abs_path)?)
}

/// 按路径校验（工作空间右键打开类 / 复制数据 / 系统菜单共用）：当前空间 workspace 内 + 存在
/// pub(crate)：lib.rs ws_drag_paths 与本文件剪贴板命令复用同一越界口径
pub(crate) fn validated_ws_path(state: &State<AppState>, abs_path: &str) -> Result<std::path::PathBuf, String> {
  let root = ws_root(state)?;
  let target = std::path::PathBuf::from(abs_path);
  if !workspace::is_under(&root, &target) {
    return Err("路径不在当前工作空间内".to_string());
  }
  if !target.exists() {
    return Err("条目已不存在（可能刚被移动或删除）".to_string());
  }
  Ok(target)
}

/// rel → 校验后的绝对路径（2026-10-01 ws 剪贴板 / 拖出命令族共用）：root 拼接后经
/// validated_ws_path 同口径校验（`..` / 绝对路径组件越界由 is_under 规范化比对拦下）
pub(crate) fn validated_ws_rel(state: &State<AppState>, rel: &str) -> Result<std::path::PathBuf, String> {
  let root = ws_root(state)?;
  validated_ws_path(state, &root.join(rel).to_string_lossy())
}

/// 用系统默认程序打开（工作空间文件右键移植库「在默认应用打开」；按真实路径）
#[tauri::command]
pub fn open_path_with_default_app(state: State<AppState>, abs_path: String) -> Result<(), String> {
  shell_open::open(&validated_ws_path(&state, &abs_path)?)
}

/// 在其他应用打开（系统「打开方式」对话框；按真实路径）
#[tauri::command]
pub fn open_path_with_dialog(state: State<AppState>, abs_path: String) -> Result<(), String> {
  shell_open::open_as(&validated_ws_path(&state, &abs_path)?)
}

/// 复制 base64（复制数据子菜单移植；格式白名单与 10MB 上限同库侧口径，services::asset_menu 复用，
/// psd/psb/svg 走解码合成图 → PNG；纯读操作）
#[tauri::command]
pub fn ws_file_base64(state: State<AppState>, abs_path: String) -> Result<String, String> {
  let target = validated_ws_path(&state, &abs_path)?;
  if !target.is_file() {
    return Err("文件夹不支持 base64 复制".to_string());
  }
  let ext = target.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
  let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
  crate::services::asset_menu::base64_from_origin(&target, &ext, size)
}

/// 系统右键菜单（Windows 原生 IContextMenu：压缩解压 / Git 等 shell 扩展全量，多选整组弹出）：
/// 模态菜单在 spawn_blocking 线程的 STA 内弹出（光标处），主线程不冻结；命令返回 = 菜单已关
/// 且选中命令已执行；用户取消 = Ok(())
#[tauri::command]
pub async fn ws_shell_menu(state: State<'_, AppState>, abs_paths: Vec<String>) -> Result<(), String> {
  let mut paths = Vec::with_capacity(abs_paths.len());
  for p in &abs_paths {
    paths.push(validated_ws_path(&state, p)?);
  }
  if paths.is_empty() {
    return Err("没有可操作的条目".to_string());
  }
  tauri::async_runtime::spawn_blocking(move || crate::adapters::shell_menu::show(&paths))
    .await
    .map_err(|e| format!("系统菜单任务异常：{e}"))?
}

/// 系统右键菜单 - 目录背景版（空白处右键「系统菜单」；经典版 新建 / 排序方式 / 查看 等在其中：
/// CreateViewObject 取目录背景 IContextMenu；「新建」落盘经 watcher 热刷新回视图）
#[tauri::command]
pub async fn ws_shell_menu_background(state: State<'_, AppState>, abs_path: String) -> Result<(), String> {
  let target = validated_ws_path(&state, &abs_path)?;
  if !target.is_dir() {
    return Err("目标不是文件夹".to_string());
  }
  tauri::async_runtime::spawn_blocking(move || crate::adapters::shell_menu::show_background(&target))
    .await
    .map_err(|e| format!("系统菜单任务异常：{e}"))?
}

/// 发送至 Animate（工作空间按路径入口，8.3 移植）：仅 fla；流程与库素材共用 cep::send_fla
#[tauri::command]
pub async fn send_path_to_animate(state: State<'_, AppState>, abs_path: String) -> Result<String, String> {
  let target = validated_ws_path(&state, &abs_path)?;
  if target.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() != Some("fla") {
    return Err("仅 fla 素材支持发送至 Animate".to_string());
  }
  if !target.is_file() {
    return Err("文件已不存在（可能刚被移动或删除）".to_string());
  }
  let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| "路径不合法".to_string())?;
  crate::commands::cep::send_fla(name, target).await
}

/// 请求缩略图（2026-09-26 性能优化 A/B 重写）：立即返回，任务进进程级 worker 池
/// （services::ws_thumb_queue：固定并发 = 可用并行度一半钳 1..=4、跨请求去重、失败记忆——
/// 解码失败键本会话不再重试，失败条目以格式图标形态随事件落回）；priority=true 整批插队
/// 队首（前端视口内缺失项），否则排尾部。完成批次仍 emit workspace-thumbs-ready
/// （负载 Vec<ThumbReady>，2026-09-26 起条目新增 thumbKind 字段）。
/// 入队前的逐条 stat / 新鲜度判定是同步 IO，挪 spawn_blocking；未开空间 / 目录缺失不动
#[tauri::command]
pub async fn request_workspace_thumbs(state: State<'_, AppState>, abs_paths: Vec<String>, priority: Option<bool>) -> Result<(), String> {
  let Some(space) = state.space.lock().map_err(|e| e.to_string())?.clone() else { return Ok(()) };
  let root = space.root.join(WORKSPACE_DIR);
  if !root.is_dir() {
    return Ok(());
  }
  let paths: Vec<std::path::PathBuf> = abs_paths.into_iter().map(std::path::PathBuf::from).collect();
  let priority = priority.unwrap_or(false);
  tauri::async_runtime::spawn_blocking(move || {
    crate::services::ws_thumb_queue::global().submit(root, paths, priority);
  })
  .await
  .map_err(|e| format!("工作空间缩略图入队异常：{e}"))
}

// ---------- 步骤 2 基础整理写操作（命令返回即更新 UI，watcher 回声由服务消重） ----------

/// 未开空间给统一中文错；开了取 workspace 根（写操作专属，读路径的空返回不适用）
fn ws_root(state: &State<AppState>) -> Result<std::path::PathBuf, String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  Ok(space.root.join(WORKSPACE_DIR))
}

/// 链接拓扑可能变化后重建工作空间监听（junction 方向 b：每条「链接在且目标在」各补一条
/// target 监听——增删 / 重指 / 改名 / 移动「门」都会改变这个集合）。重建 = stop + spawn
/// 轻量监听，代价可忽略，故不挑「是否真的动了链接」恒重跑（简单口径，2026-10-02 定）；
/// start_watch 内 reconcile 顺带剔出已消失链接的登记，自洽
fn restart_watch(app: &AppHandle, state: &State<AppState>) {
  if let Ok(guard) = state.space.lock() {
    if let Some(space) = guard.as_ref() {
      workspace::start_watch(app, &space.root);
    }
  }
}

/// 新建文件夹：rel_dir 下创建 name；重名 / 非法名 / .anikit 内给中文错
#[tauri::command]
pub fn ws_create_folder(state: State<AppState>, rel_dir: String, name: String) -> Result<(), String> {
  let root = ws_root(&state)?;
  if !root.is_dir() {
    return Err("工作空间目录不存在".to_string());
  }
  workspace::create_folder(&root, &rel_dir, &name)
}

/// 重命名：rel_path 最后一段改为 new_name（文件与文件夹同径）；目标重名给中文错
#[tauri::command]
pub fn ws_rename(app: AppHandle, state: State<AppState>, rel_path: String, new_name: String) -> Result<(), String> {
  let root = ws_root(&state)?;
  workspace::rename_entry(&root, &rel_path, &new_name)?;
  restart_watch(&app, &state); // 改的可能就是链接「门」——监听恒重建（简单口径见 restart_watch）
  Ok(())
}

/// 删除到系统回收站（多路径一次调用；PRD 06 不进库内回收站）
#[tauri::command]
pub fn ws_delete(app: AppHandle, state: State<AppState>, rel_paths: Vec<String>) -> Result<(), String> {
  let root = ws_root(&state)?;
  workspace::delete_entries(&root, &rel_paths)?;
  restart_watch(&app, &state); // 删的可能含链接「门」或其祖先——监听恒重建
  Ok(())
}

/// 移动（拖拽 / 剪切粘贴）：rel_paths 移入 dest_rel；重名自动 _n 规避（n 从 2 递增）
#[tauri::command]
pub fn ws_move(app: AppHandle, state: State<AppState>, rel_paths: Vec<String>, dest_rel: String) -> Result<(), String> {
  let root = ws_root(&state)?;
  workspace::move_entries(&root, &rel_paths, &dest_rel)?;
  restart_watch(&app, &state); // 移的可能含链接「门」——监听恒重建
  Ok(())
}

/// 粘贴：rel_paths 复制（cut=false）或移动（cut=true）进 dest_rel；剪切成功后前端清剪贴板
#[tauri::command]
pub fn ws_paste(app: AppHandle, state: State<AppState>, rel_paths: Vec<String>, dest_rel: String, cut: bool) -> Result<(), String> {
  let root = ws_root(&state)?;
  if cut {
    workspace::move_entries(&root, &rel_paths, &dest_rel)?;
    restart_watch(&app, &state); // 剪切 = 移动，可能动链接「门」——监听恒重建
    Ok(())
  } else {
    workspace::copy_entries(&root, &rel_paths, &dest_rel) // 复制不改链接拓扑，不重建
  }
}

/// 创建副本（库「创建副本」Ctrl+D 移植）：rel_paths 逐个复制进自身所在目录，重名自动 _n 规避
#[tauri::command]
pub fn ws_duplicate(state: State<AppState>, rel_paths: Vec<String>) -> Result<(), String> {
  let root = ws_root(&state)?;
  workspace::duplicate_entries(&root, &rel_paths)
}

// ---------- 链接（junction 方向 b：外部文件夹挂进 WS 子目录，PRD 02） ----------

/// 挂载外部文件夹：target_path（外部目录绝对路径）以 junction 挂进 dest_rel 下，
/// 挂载名 = target 目录名（重名自动 _n 规避）；返回挂载点 rel。
/// 拒绝：target 非目录 / 在工作空间内（含等于）/ 是工作空间祖先（防循环）——中文报错
#[tauri::command]
pub fn ws_add_link(app: AppHandle, state: State<AppState>, dest_rel: String, target_path: String) -> Result<String, String> {
  let root = ws_root(&state)?;
  let rel = workspace::add_link(&root, &dest_rel, std::path::Path::new(&target_path))?;
  restart_watch(&app, &state); // 新链接 = 新补一条 target 监听——重建
  Ok(rel)
}

/// 解除挂载：链接本体进系统回收站（只删「门」不动外部目标），剔出登记清单；
/// 断链链接同样可移除。rel 不是链接给中文错
#[tauri::command]
pub fn ws_remove_link(app: AppHandle, state: State<AppState>, rel_path: String) -> Result<(), String> {
  let root = ws_root(&state)?;
  workspace::remove_link(&root, &rel_path)?;
  restart_watch(&app, &state); // 链接没了 = 补监听少一条——重建
  Ok(())
}

/// 改挂目标：rel 链接重指向 new_target（外部目录绝对路径）；校验同挂载（目录 /
/// 不在工作空间内 / 非祖先）。断链链接走本径「送修」
#[tauri::command]
pub fn ws_retarget_link(app: AppHandle, state: State<AppState>, rel_path: String, new_target: String) -> Result<(), String> {
  let root = ws_root(&state)?;
  workspace::retarget_link(&root, &rel_path, std::path::Path::new(&new_target))?;
  restart_watch(&app, &state); // 目标变了 = 补监听换 target——重建
  Ok(())
}

// ---------- ws ↔ Explorer 剪贴板 / 拖入互通（2026-10-01，PRD 02-13） ----------
// 剪贴板读写属 COM/IO，一律 async（规范 §2）；实际读写与位图解码挪 spawn_blocking 不占 tokio worker

/// 系统剪贴板文件清单（ws_clipboard_files 返回 / 菜单「粘贴」可见性查询用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsClipboardFiles {
  /// CF_HDROP 文件绝对路径集（无文件 = 空）
  pub paths: Vec<String>,
  /// Preferred DropEffect=MOVE（外部剪切 = 移动语义）
  pub cut: bool,
}

/// ws_paste_system 返回：had_files = 系统剪贴板含文件列表（false = 无文件属常态，前端静默）；
/// pasted = 实际落入的目标 rel 集（源已不存在项跳过，全失效 = 空，前端 toast）；moved = 外部剪切移动入
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsPasteSystemResult {
  pub pasted: Vec<String>,
  pub moved: bool,
  pub had_files: bool,
}

/// 写系统剪贴板（PRD 02-13：ws 复制 / 剪切同步写系统剪贴板，Explorer 可直接粘贴接力）：
/// CF_HDROP 文件列表 + 单张静态图 ≤20MB 附位图（PNG + CF_DIBV5，库 2026-09-17 同口径，
/// services::asset_menu::clipboard_bitmap 复用）+ 剪切附 Preferred DropEffect=MOVE
///（资源管理器粘贴即移动，源由系统自删）。rel 逐条校验（越界 / 不存在跳过 + 去重保序），空集报错
#[tauri::command]
pub async fn ws_write_clipboard(state: State<'_, AppState>, rel_paths: Vec<String>, cut: bool) -> Result<(), String> {
  let mut seen = std::collections::HashSet::new();
  let paths: Vec<std::path::PathBuf> = rel_paths
    .iter()
    .filter_map(|rel| validated_ws_rel(&state, rel).ok())
    .filter(|p| seen.insert(p.clone()))
    .collect();
  if paths.is_empty() {
    return Err("没有可复制的文件".to_string());
  }
  tauri::async_runtime::spawn_blocking(move || {
    let bitmap = crate::services::asset_menu::clipboard_bitmap(&paths);
    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
    crate::adapters::clipboard::write_files_clipboard(&refs, bitmap.as_ref(), cut)
  })
  .await
  .map_err(|e| format!("系统剪贴板写入任务异常：{e}"))?
}

/// 读系统剪贴板文件清单（右键菜单「粘贴」出现条件 = 内部剪贴板非空 || 系统剪贴板含文件，
/// 前端开 ws 菜单前查询）；仲裁读取在 ws_paste_system 内，不经本命令
#[tauri::command]
pub async fn ws_clipboard_files() -> Result<WsClipboardFiles, String> {
  tauri::async_runtime::spawn_blocking(|| {
    let (paths, cut) = crate::adapters::clipboard::read_files_clipboard();
    Ok(WsClipboardFiles {
      paths: paths.into_iter().map(|p| p.to_string_lossy().into_owned()).collect(),
      cut,
    })
  })
  .await
  .map_err(|e| format!("系统剪贴板读取任务异常：{e}"))?
}

/// 系统剪贴板粘贴进 ws（PRD 02-13 粘贴仲裁的外部分支）：读 CF_HDROP → 源已不存在项跳过 →
/// **复制来源恒复制 / 剪切来源（DropEffect=MOVE）= 移动**（同盘 rename，异盘复制后删源）；
/// 目录递归、重名 名_n 规避；图片 / 文本等无文件内容不导入（库导入专属）
#[tauri::command]
pub async fn ws_paste_system(state: State<'_, AppState>, dest_rel: String) -> Result<WsPasteSystemResult, String> {
  let root = ws_root(&state)?;
  tauri::async_runtime::spawn_blocking(move || {
    let (paths, cut) = crate::adapters::clipboard::read_files_clipboard();
    if paths.is_empty() {
      return Ok(WsPasteSystemResult { pasted: Vec::new(), moved: false, had_files: false });
    }
    let sources: Vec<std::path::PathBuf> = paths.into_iter().filter(|p| p.exists()).collect();
    let pasted = workspace::import_external_entries(&root, &sources, &dest_rel, cut)?;
    Ok(WsPasteSystemResult { pasted, moved: cut, had_files: true })
  })
  .await
  .map_err(|e| format!("系统剪贴板粘贴任务异常：{e}"))?
}

/// Explorer 拖入（PRD 02-13：拖入一律复制，非破坏口径与剪贴板导入同源）：源 = 外部绝对路径
///（不做 root 约束，不存在项跳过），dest 走 ws 校验（越界 / 非目录 / .anikit 内中文报错）；
/// 目录递归、重名 名_n 规避；返回 created rel 集（前端局部重扫锚）
#[tauri::command]
pub async fn ws_import_paths(state: State<'_, AppState>, abs_paths: Vec<String>, dest_rel: String) -> Result<Vec<String>, String> {
  let root = ws_root(&state)?;
  let sources: Vec<std::path::PathBuf> = abs_paths.into_iter().map(std::path::PathBuf::from).filter(|p| p.exists()).collect();
  tauri::async_runtime::spawn_blocking(move || workspace::import_external_entries(&root, &sources, &dest_rel, false))
    .await
    .map_err(|e| format!("工作空间导入任务异常：{e}"))?
}

// ---------- 步骤 3 常用固定（.anikit/workspace.json，PRD 02） ----------

/// 常用固定清单（读取时静默剔除失效条目并回写；未开空间返回空）
#[tauri::command]
pub fn ws_list_pinned(state: State<AppState>) -> Vec<String> {
  let Some(space) = state.space.lock().ok().and_then(|g| g.clone()) else { return Vec::new() };
  let root = space.root.join(WORKSPACE_DIR);
  if !root.is_dir() {
    return Vec::new();
  }
  workspace::list_pinned(&root)
}

/// 固定文件夹到常用；返回最新清单
#[tauri::command]
pub fn ws_pin(state: State<AppState>, rel_path: String) -> Result<Vec<String>, String> {
  let root = ws_root(&state)?;
  workspace::pin(&root, &rel_path)
}

/// 取消固定；返回最新清单
#[tauri::command]
pub fn ws_unpin(state: State<AppState>, rel_path: String) -> Result<Vec<String>, String> {
  let root = ws_root(&state)?;
  Ok(workspace::unpin(&root, &rel_path))
}

/// 打开系统回收站（工作空间侧栏回收站行点击，2026-10-01；ws 删除走系统回收站，PRD 06）
#[tauri::command]
pub fn open_system_recycle_bin() -> Result<(), String> {
  shell_open::open_recycle_bin()
}
