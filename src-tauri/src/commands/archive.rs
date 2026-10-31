//! 命令 API：档案文件夹（2026-08-15 档案子系统一轮骨架，PRD 02_文件整理 03 / 03_数据链接）。
//! 写命令走撤销三步（锁序 library → undo，与 commands::operations 同约）；只读命令同步直出。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library::{Library, ARCHIVES_FILE, FOLDERS_FILE};
use crate::core::models::{ArchiveData, AssetMeta, Badge, Avatar, DescItem, Folder, Placement, PromptBinding, ARCHIVE_KIND, DEFAULT_BLOCK_ID, ROOT_FOLDER_ID};
use crate::core::undo::DiskOp;
use crate::services::archive::{self, ConvertOutcome};
use crate::services::custom_data;

/// 新建档案：parentId = null / "root" 为顶层；afterFolderId / blockId 口径同 create_folder
#[tauri::command]
pub async fn create_archive(
  state: State<'_, AppState>,
  parent_id: Option<String>,
  name: String,
  after_folder_id: Option<String>,
  block_id: Option<String>,
) -> Result<Folder, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建档案");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::create_archive(lib, parent_id.as_deref(), &name, after_folder_id.as_deref(), block_id.as_deref()) {
    Ok(folder) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(folder)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 文件夹转换为档案（两段式）：有子文件夹且 confirmed = false 时返回 "need_confirm"
/// 不动数据（询问段丢弃事务，不进撤销栈）；否则剔除层级并置 kind，返回 "done"；已是档案幂等 "done"
#[tauri::command]
pub async fn convert_to_archive(
  state: State<'_, AppState>,
  folder_id: String,
  confirmed: bool,
) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("转换为档案");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::convert_to_archive(lib, &folder_id, confirmed) {
    // 询问段未动数据：丢弃事务，不进撤销栈
    Ok(ConvertOutcome::NeedConfirm) => {
      undo.cancel();
      Ok("need_confirm".to_string())
    }
    Ok(ConvertOutcome::Done) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok("done".to_string())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 档案解散为普通文件夹：摘 kind 标记 + 清 archives.json 中该档案的记录
#[tauri::command]
pub async fn convert_to_folder(state: State<'_, AppState>, folder_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("解散档案为文件夹");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::convert_to_folder(lib, &folder_id) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 档案清单项（list_archives 输出）：ArchiveData 全字段 + 容量 + 封面图绝对路径 + 版本戳
///（前端 convertFileSrc + ?v= 直用；无自定义封面 / 文件缺失 = null，前端回退子项首图 / 默认封面图）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveListItem {
  pub folder_id: String,
  pub badge: Option<Badge>,
  pub avatar: Option<Avatar>,
  pub props: Vec<String>,
  pub descriptions: Vec<DescItem>,
  pub links: Vec<String>,
  pub created_at: String,
  /// 档案容量（字节，2026-08-30 用户定案）：档案内 Placement 引用资产 size 去重合计（同详情口径）
  pub total_size: u64,
  pub cover_path: Option<String>,
  /// 封面版本戳（2026-09-08 覆盖式烘培）：文件 mtime 纳秒——同路径重写后前端 ?v= 驱动 img 重载
  pub cover_ver: Option<String>,
  /// 提示词绑定（2026-09-26 提示词轮）：ArchiveData 加了字段但本 DTO 另组，须显式带出；未指定 = null
  pub prompt: Option<PromptBinding>,
}

/// 档案内容统计：fileCount = 档案内 Placement 数（逐条计）；
/// totalSize = 引用资产 size 合计（同一资产多链接只计一次——占用空间语义）
fn archive_stats(lib: &Library, folder_id: &str) -> (u32, u64) {
  let mut file_count: u32 = 0;
  let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
  let mut total_size: u64 = 0;
  for p in lib.folders.items.iter().filter(|p| p.folder == folder_id) {
    file_count += 1;
    if seen.insert(p.asset.as_str()) {
      if let Some(meta) = lib.index.assets.get(&p.asset) {
        total_size += meta.size;
      }
    }
  }
  (file_count, total_size)
}

/// 全部档案数据 + 容量 + 封面路径
#[tauri::command]
pub fn list_archives(state: State<AppState>) -> Result<Vec<ArchiveListItem>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(archive_list_items(lib))
}

/// 档案清单组装（ArchiveData + 容量 + 封面路径）；pub(crate)：server/mcp（AI 外接通道）同用
pub(crate) fn archive_list_items(lib: &Library) -> Vec<ArchiveListItem> {
  archive::list_archives(lib)
    .into_iter()
    .map(|a| {
      let (cover_path, cover_ver) = archive::cover_file_info(lib, &a.folder_id).unzip();
      ArchiveListItem {
        cover_path,
        cover_ver,
        total_size: archive_stats(lib, &a.folder_id).1,
        folder_id: a.folder_id,
        badge: a.badge,
        avatar: a.avatar,
        props: a.props,
        descriptions: a.descriptions,
        links: a.links,
        created_at: a.created_at,
        prompt: a.prompt,
      }
    })
    .collect()
}

/// 档案详情视图（2026-08-15 任务 A 三项）：ArchiveData 全字段 + 统计（fileCount / totalSize）；
/// fileCount = 档案内 Placement 数（档案无子文件夹，直接匹配即全量）；
/// totalSize = 档案内 Placement 引用资产 size 合计（同一资产多链接只计一次——占用空间语义）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveDetailView {
  pub folder_id: String,
  pub badge: Option<Badge>,
  pub avatar: Option<Avatar>,
  pub props: Vec<String>,
  pub descriptions: Vec<DescItem>,
  pub links: Vec<String>,
  pub created_at: String,
  pub file_count: u32,
  pub total_size: u64,
  /// 封面图绝对路径（6.1；无自定义封面 / 文件缺失 = null，前端回退子项首图 / 默认封面图）
  pub cover_path: Option<String>,
  /// 封面版本戳（2026-09-08 覆盖式烘培）：文件 mtime 纳秒，前端 ?v= 驱动 img 重载
  pub cover_ver: Option<String>,
  /// 提示词绑定（2026-09-26 提示词轮，PRD 03_数据链接 02_文件，档案侧同构）；未指定 = null
  pub prompt: Option<PromptBinding>,
}

/// 单个档案的详细数据 + 统计；无记录返回默认空记录（档案刚转换、尚未配置数据）
#[tauri::command]
pub fn get_archive_data(state: State<AppState>, folder_id: String) -> Result<ArchiveDetailView, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let rec = lib.archives.archives.iter().find(|a| a.folder_id == folder_id).cloned().unwrap_or_else(|| ArchiveData::new(&folder_id));
  let (file_count, total_size) = archive_stats(lib, &folder_id);
  let (cover_path, cover_ver) = archive::cover_file_info(lib, &folder_id).unzip();
  Ok(ArchiveDetailView {
    folder_id: rec.folder_id,
    badge: rec.badge,
    avatar: rec.avatar,
    props: rec.props,
    descriptions: rec.descriptions,
    links: rec.links,
    created_at: rec.created_at,
    file_count,
    total_size,
    cover_path,
    cover_ver,
    prompt: rec.prompt,
  })
}

/// 编辑档案属性（6.1，archives.json 单写点）
#[tauri::command]
pub async fn set_archive_props(state: State<'_, AppState>, folder_id: String, props: Vec<String>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("编辑属性");
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match custom_data::set_archive_props(lib, &folder_id, &props) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 编辑档案描述（6.1）
#[tauri::command]
pub async fn set_archive_descriptions(state: State<'_, AppState>, folder_id: String, descriptions: Vec<DescItem>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("编辑描述");
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match custom_data::set_archive_descriptions(lib, &folder_id, &descriptions) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 档案封面设置（6.1，43 稿确认）：徽标三角（null/省略 = 不动……serde Option<Option<Badge>>：
/// 缺省 = 不动、null = 清除、对象 = 设置）+ 封面（缺省 = 不动、给 = 选图按取景参数烘培）。
/// 封面图独立存 cache/covers/（可再生缓存，2026-09-08 用户定），**覆盖式**文件名恒 {folder_id}
/// 一档案一文件；撤销依托 = 旧封面改名暂存 cache/covers-trash/（本命令登记的 DiskOp::Rename
/// 在 undo/redo 时执行搬运，同 sequence 删帧 seq-trash 口径；暂存开库清空 = 会话结束真弃）
#[tauri::command]
pub async fn set_archive_cover_settings(
  state: State<'_, AppState>,
  folder_id: String,
  badge: Option<Option<Badge>>,
  cover: Option<archive::CoverInput>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("设置档案封面");
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::set_archive_cover_settings(lib, &folder_id, badge, cover) {
    Ok(out) => {
      if !out.changed {
        undo.cancel();
        return Ok(());
      }
      // 封面覆盖烘培的撤销磁盘操作（apply_disk 逆序执行——登记序 = 执行序的反向）：
      // undo 执行序 = [新封面挪入暂存 a, 旧封面自暂存 b 归位]；redo 执行序 = [旧封面再挪 b, 新封面自 a 归位]
      if let Some(b) = out.bake {
        let trash_a = crate::core::library::covers_trash_dir(&lib.root).join(format!("{}-a-{}", b.stamp, b.file));
        if let Some((old_name, staged)) = b.staged_old {
          let cover_old = crate::core::library::covers_dir(&lib.root).join(&old_name);
          undo.record_disk_op(
            DiskOp::Rename { from: staged.clone(), to: cover_old.clone() }, // undo ②旧封面归位
            DiskOp::Rename { from: trash_a.clone(), to: b.new_path.clone() }, // redo ②新封面归位
          );
          undo.record_disk_op(
            DiskOp::Rename { from: b.new_path.clone(), to: trash_a.clone() }, // undo ①新封面暂存
            DiskOp::Rename { from: cover_old, to: staged },                 // redo ①旧封面再暂存
          );
        } else {
          // 首次设置（无旧封面）：undo = 新封面挪入暂存（留内容供 redo），redo = 挪回
          undo.record_disk_op(
            DiskOp::Rename { from: b.new_path.clone(), to: trash_a.clone() },
            DiskOp::Rename { from: trash_a, to: b.new_path },
          );
        }
      }
      undo.commit().map_err(|e| e.to_string())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 资产详情（PRD 03_数据链接 02）：基础信息 + 属性 / 链接 / 注释 + 链接区块（Placement 反查）+ 归属档案
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDetail {
  pub asset_id: String,
  pub origin_name: String,
  pub ext: String,
  pub size: u64,
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub duration: Option<f64>,
  pub imported_at: String,
  pub modified_at: String,
  /// 属性词（复用原 tags 字段，2026-08-04 标签体系取消后语义转为属性，PRD 03_数据链接 06）
  pub tags: Vec<String>,
  /// 描述（标题 + 内容，6.1）
  pub descriptions: Vec<DescItem>,
  /// 主色（6.1 颜色解析）：metadata.colors，#RRGGBB 按占比降序（最多 10，bar 显示前 7）
  pub colors: Vec<String>,
  pub links: Vec<String>,
  pub note: String,
  /// 链接区块：该资产全部 Placement 反查
  pub placements: Vec<PlacementLink>,
  /// 归属档案：Placement 所在 folder 链上 kind = archive 者去重
  pub archives: Vec<ArchiveRef>,
  /// 缩略图区块（8.1 伴生缩略图，2026-09-23 接业务）·调用侧：本资产 thumb_asset_id 指向的
  /// hidden 伴生资产（users = 它被引用的总数）；无绑定 / 目标非 hidden（还原途中）= null
  pub thumb_asset: Option<ThumbLinkItem>,
  /// 缩略图区块·被调用侧：本资产是 hidden 伴生缩略图时，引用它的资产清单（按名称排序，users 恒 null）
  pub thumb_used_by: Vec<ThumbLinkItem>,
  /// 提示词绑定（2026-09-26 提示词轮，PRD 03_数据链接 02_文件）：引用 + 内容快照；未指定 = null
  pub prompt: Option<PromptBinding>,
  #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
  pub r#type: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub start: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub end: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub start_frame: Option<u32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub end_frame: Option<u32>,
}

/// 缩略图区块一条（8.1，2026-09-23）：assetId + 显示名（首条 Placement 名，无则 originName）
/// + 引用数（仅调用侧卡片显示——稿「雪牧_自动.webp + 2」）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbLinkItem {
  pub asset_id: String,
  pub name: String,
  pub users: Option<u32>,
}

/// 链接区块一条：哪个区块的哪个文件夹、显示什么名称
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementLink {
  /// 所在文件夹 ID；未分类（root Placement）= null
  pub folder_id: Option<String>,
  /// 所在区块名（未分类 / 无区块归属 = 默认区块「目录」）
  pub block_name: String,
  /// 路径段，最多三段：根目录 / 父级 / 所在目录（超出三段取首段 + 末两段）；未分类 = ["未分类"]
  pub path: Vec<String>,
  pub name: String,
  pub origin: bool,
}

/// 归属档案一条
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveRef {
  pub folder_id: String,
  pub name: String,
}

/// 资产详情；资产不存在报中文错。
/// 6.1 颜色解析：返回前对存量资产懒回填主色（ensure_asset_colors：colors 空 + 有缩略图才触发，
/// 被动补数不刷 modifiedAt、不进撤销，失败静默下回重试）——读命令带维护性写副作用先例；
/// 2026-10-08 origin 外部编辑自愈（哈希漂移治理第 2 层）同例挂上：size/mtime 漂移 →
/// 回写 size/hash/宽高（+ 非音视频重生成缩略图）后广播 asset-origin-healed 同步前端。
/// 自愈涉文件 IO，改 async 走线程池（规范第 2 条；原同步壳只剩内存索引读）
#[tauri::command]
pub async fn get_asset_detail(app: tauri::AppHandle, state: State<'_, AppState>, asset_id: String) -> Result<AssetDetail, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  crate::services::color::ensure_asset_colors(lib, &asset_id);
  if let Some((size, modified_at)) = crate::services::origin_sync::heal_if_drifted(lib, &asset_id) {
    crate::services::origin_sync::emit_healed(&app, &asset_id, size, &modified_at);
  }
  asset_detail(lib, &asset_id)
}

/// 组装资产详情（命令层薄壳，便于单测）
/// pub(crate)：server/mcp（AI 外接通道）同用——MCP 走纯读，不调 ensure_asset_colors 懒回填
pub(crate) fn asset_detail(lib: &Library, asset_id: &str) -> Result<AssetDetail, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?;
  let mut placements = Vec::new();
  let mut archives: Vec<ArchiveRef> = Vec::new();
  for p in lib.index.placements_of(asset_id) {
    let Some(link) = placement_link(lib, p) else { continue };
    // 归属档案反查：沿 folder 父链上溯，kind = archive 者去重收集
    let mut cur = link.folder_id.clone();
    while let Some(id) = cur {
      let Some(f) = lib.index.folders.get(&id) else { break };
      if f.kind.as_deref() == Some(ARCHIVE_KIND) && !archives.iter().any(|a| a.folder_id == id) {
        archives.push(ArchiveRef { folder_id: id.clone(), name: f.name.clone() });
      }
      cur = f.parent.clone();
    }
    placements.push(link);
  }
  // 回收站资产（无存活 Placement）：按 PRD 02-15 链接区块显示「回收站（原位置：区块 / 路径）」——
  // recycle.json 记录删除前全部 Placement，取首条原位置合成兜底条目
  //（folderId = null 标识伪条目，前端不做档案后缀判定；2026-08-16 用户拍板）
  if placements.is_empty() {
    if let Some(item) = lib.recycle.items.iter().find(|i| i.asset == asset_id) {
      if let Some(p) = item.placements.first() {
        if let Some(link) = placement_link(lib, p) {
          let orig = format!("{} / {}", link.block_name, link.path.join(" / "));
          placements.push(PlacementLink {
            folder_id: None,
            block_name: "回收站".to_string(),
            path: vec![orig],
            name: link.name,
            origin: link.origin,
          });
        }
      }
    }
  }
  Ok(AssetDetail {
    asset_id: meta.id.clone(),
    origin_name: meta.origin_name.clone(),
    ext: meta.ext.clone(),
    size: meta.size,
    width: meta.width,
    height: meta.height,
    duration: meta.duration,
    imported_at: meta.imported_at.clone(),
    modified_at: meta.modified_at.clone(),
    tags: meta.tags.clone(),
    descriptions: meta.descriptions.clone(),
    colors: meta.colors.clone(),
    links: meta.links.clone(),
    note: meta.note.clone(),
    placements,
    archives,
    thumb_asset: thumb_link_of(lib, meta),
    thumb_used_by: thumb_users_of(lib, meta),
    prompt: meta.prompt.clone(),
    r#type: meta.r#type.clone(),
    source: meta.source.clone(),
    start: meta.start,
    end: meta.end,
    start_frame: meta.start_frame,
    end_frame: meta.end_frame,
  })
}

// —— 缩略图区块（8.1 伴生缩略图，2026-09-23 接业务）——

/// 资产显示名：首条 Placement 名（回收站等无存活 Placement = originName 兜底）
fn thumb_display_name(lib: &Library, asset_id: &str, origin_name: &str) -> String {
  lib
    .index
    .placements_of(asset_id)
    .first()
    .map(|p| p.name.clone())
    .unwrap_or_else(|| origin_name.to_string())
}

/// 引用计数：多少资产的 thumb_asset_id 指向它（调用侧卡片的「users」）
fn thumb_user_count(lib: &Library, thumb_id: &str) -> u32 {
  lib.index.assets.values().filter(|m| m.thumb_asset_id.as_deref() == Some(thumb_id)).count() as u32
}

/// 调用侧：本资产 thumb_asset_id → hidden 伴生资产（非 hidden 不代理，同 views::placement_view 口径）
fn thumb_link_of(lib: &Library, meta: &AssetMeta) -> Option<ThumbLinkItem> {
  let t = meta
    .thumb_asset_id
    .as_deref()
    .and_then(|id| lib.index.assets.get(id))
    .filter(|t| t.hidden == Some(true))?;
  Some(ThumbLinkItem {
    asset_id: t.id.clone(),
    name: thumb_display_name(lib, &t.id, &t.origin_name),
    users: Some(thumb_user_count(lib, &t.id)),
  })
}

/// 被调用侧：本资产是 hidden 伴生缩略图时，引用它的资产清单（按显示名排序保序稳定）
fn thumb_users_of(lib: &Library, meta: &AssetMeta) -> Vec<ThumbLinkItem> {
  if meta.hidden != Some(true) {
    return Vec::new();
  }
  let mut users: Vec<ThumbLinkItem> = lib
    .index
    .assets
    .values()
    .filter(|m| m.thumb_asset_id.as_deref() == Some(meta.id.as_str()))
    .map(|m| ThumbLinkItem {
      asset_id: m.id.clone(),
      name: thumb_display_name(lib, &m.id, &m.origin_name),
      users: None,
    })
    .collect();
  users.sort_by(|a, b| a.name.cmp(&b.name));
  users
}

/// Placement → 链接区块一条；folder 链断裂（脏数据）返回 None 整条跳过
/// pub(crate)：server/mcp list_links（AI 外接通道只读补批，2026-10-12）同用
pub(crate) fn placement_link(lib: &Library, p: &Placement) -> Option<PlacementLink> {
  if p.folder == ROOT_FOLDER_ID {
    return Some(PlacementLink {
      folder_id: None,
      block_name: block_name(lib, None),
      path: vec!["未分类".to_string()],
      name: p.name.clone(),
      origin: p.origin,
    });
  }
  // folder 链（所在目录 → 根目录逐级上溯）
  let mut chain: Vec<&Folder> = Vec::new();
  let mut cur = Some(p.folder.clone());
  while let Some(id) = cur {
    let f = lib.index.folders.get(&id)?;
    chain.push(f);
    cur = f.parent.clone();
  }
  chain.reverse();
  let top_block = chain[0].block.clone();
  let mut path: Vec<String> = chain.iter().map(|f| f.name.clone()).collect();
  // 最多三段（PRD 03_数据链接 02：区块 / 根目录父级 / 所在目录；区块由 blockName 单列）
  if path.len() > 3 {
    path = vec![path[0].clone(), path[path.len() - 2].clone(), path[path.len() - 1].clone()];
  }
  Some(PlacementLink {
    folder_id: Some(p.folder.clone()),
    block_name: block_name(lib, top_block.as_deref()),
    path,
    name: p.name.clone(),
    origin: p.origin,
  })
}

/// 区块名：None / 失效引用 = 默认区块「文件夹」（2026-09-17 目录语义改文件夹）
fn block_name(lib: &Library, block: Option<&str>) -> String {
  let id = block.unwrap_or(DEFAULT_BLOCK_ID);
  lib.folders.blocks.iter().find(|b| b.id == id).map(|b| b.name.clone()).unwrap_or_else(|| "文件夹".to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::core::models::{AssetMeta, Block};

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str, parent: Option<&str>, block: Option<&str>, order: i32) {
    let folder = Folder {
      id: id.into(),
      name: name.into(),
      parent: parent.map(|s| s.into()),
      block: block.map(|s| s.into()),
      icon: None,
      kind: None,
      color: None,
      groups: None,
      order,
    };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_asset(lib: &mut Library, id: &str) {
    let meta: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": "人物", "originName": "人物.fla", "ext": "fla", "size": 100,
      "width": 1920, "height": 1080, "importedAt": "2026-08-15T00:00:00+08:00",
      "modifiedAt": "2026-08-15T00:00:00+08:00", "tags": ["主角"], "links": ["https://x"], "note": "注释"
    }))
    .unwrap();
    lib.index.assets.insert(id.into(), meta);
  }

  fn add_placement(lib: &mut Library, asset: &str, folder: &str, name: &str, origin: bool) {
    lib.folders.items.push(Placement { asset: asset.into(), folder: folder.into(), name: name.into(), order: 0, origin });
    lib.index.rebuild_placements(&lib.folders.items);
  }

  /// 缩略图区块（8.1 伴生缩略图，2026-09-23 接业务）：调用侧 = thumb_asset（显示名 = 首条
  /// Placement 名 + 引用总数）；被调用侧 = hidden 资产反查使用方清单（按名排序，users 恒 None）；
  /// 目标非 hidden 不代理（同 views::placement_view 口径）
  #[test]
  fn asset_detail_thumb_links() {
    let mut h = make_lib();
    let webp: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-T", "name": "雪牧_自动", "originName": "雪牧_自动.webp", "ext": "webp",
      "size": 100, "importedAt": "2026-08-15T00:00:00+08:00", "modifiedAt": "2026-08-15T00:00:00+08:00",
      "hidden": true
    }))
    .unwrap();
    let mut fla: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-F", "name": "工程", "originName": "工程.fla", "ext": "fla", "size": 100,
      "importedAt": "2026-08-15T00:00:00+08:00", "modifiedAt": "2026-08-15T00:00:00+08:00"
    }))
    .unwrap();
    fla.thumb_asset_id = Some("AN-T".into());
    let mut fla2 = fla.clone();
    fla2.id = "AN-F2".into();
    let mut fla3 = fla.clone();
    fla3.id = "AN-F3".into();
    fla3.thumb_asset_id = Some("AN-N".into()); // 指向非 hidden 资产 → 不代理
    let plain: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-N", "name": "普通图", "originName": "普通图.png", "ext": "png",
      "size": 100, "importedAt": "2026-08-15T00:00:00+08:00", "modifiedAt": "2026-08-15T00:00:00+08:00"
    }))
    .unwrap();
    for m in [webp, fla, fla2, fla3, plain] {
      h.lib.index.assets.insert(m.id.clone(), m);
    }
    add_placement(&mut h.lib, "AN-T", "root", "雪牧_自动.webp", true);
    add_placement(&mut h.lib, "AN-F", "root", "工程.fla", true);
    add_placement(&mut h.lib, "AN-F2", "root", "工程2.fla", true);

    // 调用侧：fla → 伴生 webp（名称取首条 Placement 名，users = 两工程共绑 = 2）
    let d = asset_detail(&h.lib, "AN-F").unwrap();
    let ta = d.thumb_asset.expect("fla 应带出调用侧伴生缩略图");
    assert_eq!(ta.asset_id, "AN-T");
    assert_eq!(ta.name, "雪牧_自动.webp");
    assert_eq!(ta.users, Some(2));
    assert!(d.thumb_used_by.is_empty(), "fla 非 hidden，无被调用侧");

    // 被调用侧：hidden webp 反查两个工程（按显示名排序），users 恒 None
    let d2 = asset_detail(&h.lib, "AN-T").unwrap();
    assert!(d2.thumb_asset.is_none(), "hidden 资产自身无调用侧");
    let names: Vec<&str> = d2.thumb_used_by.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, vec!["工程.fla", "工程2.fla"], "被调用侧反查按名排序");
    assert!(d2.thumb_used_by.iter().all(|u| u.users.is_none()));

    // 非 hidden 目标不代理（还原途中脏引用不显示）
    let d3 = asset_detail(&h.lib, "AN-F3").unwrap();
    assert!(d3.thumb_asset.is_none(), "目标非 hidden 不代理");
  }

  /// 链接区块反查：普通文件夹链（区块名 + 三段路径）与未分类（目录 / ["未分类"]）
  #[test]
  fn asset_detail_placement_links() {
    let mut h = make_lib();
    h.lib.folders.blocks.push(Block { id: "B01".into(), name: "角色".into(), icon: None, order: 1 });
    add_folder(&mut h.lib, "F-TOP", "顶级", None, Some("B01"), 0);
    add_folder(&mut h.lib, "F-MID", "父级", Some("F-TOP"), None, 0);
    add_folder(&mut h.lib, "F-LEAF", "所在", Some("F-MID"), None, 0);
    add_asset(&mut h.lib, "AN-A");
    add_placement(&mut h.lib, "AN-A", "F-LEAF", "男人.fla", true);
    add_placement(&mut h.lib, "AN-A", "root", "人物.fla", false);

    let d = asset_detail(&h.lib, "AN-A").unwrap();
    assert_eq!(d.asset_id, "AN-A");
    assert_eq!((d.width, d.height), (Some(1920), Some(1080)));
    assert_eq!(d.tags, vec!["主角"]);
    assert_eq!(d.placements.len(), 2);
    let dir_link = &d.placements[0];
    assert_eq!(dir_link.folder_id.as_deref(), Some("F-LEAF"));
    assert_eq!(dir_link.block_name, "角色", "顶级祖先的区块");
    assert_eq!(dir_link.path, vec!["顶级", "父级", "所在"], "三段：根目录 / 父级 / 所在目录");
    assert!(dir_link.origin);
    let root_link = &d.placements[1];
    assert_eq!(root_link.folder_id, None, "未分类 folderId = null");
    assert_eq!(root_link.block_name, "文件夹");
    assert_eq!(root_link.path, vec!["未分类"]);
    assert!(!root_link.origin);
    // 资产不存在报中文错
    assert!(asset_detail(&h.lib, "AN-不存在").is_err());
  }

  /// 超过三段的链：取首段 + 末两段
  #[test]
  fn asset_detail_path_truncates_to_three_segments() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F1", "一", None, None, 0);
    add_folder(&mut h.lib, "F2", "二", Some("F1"), None, 0);
    add_folder(&mut h.lib, "F3", "三", Some("F2"), None, 0);
    add_folder(&mut h.lib, "F4", "四", Some("F3"), None, 0);
    add_asset(&mut h.lib, "AN-A");
    add_placement(&mut h.lib, "AN-A", "F4", "x.fla", true);

    let d = asset_detail(&h.lib, "AN-A").unwrap();
    assert_eq!(d.placements[0].path, vec!["一", "三", "四"], "首段（根目录）+ 末两段（父级 / 所在目录）");
    assert_eq!(d.placements[0].block_name, "文件夹", "无区块归属 = 默认区块");
  }

  /// 归属档案反查：folder 链上 kind = archive 者去重（含嵌套档案、跨 Placement 去重）
  #[test]
  fn asset_detail_archive_refs_deduped() {    let mut h = make_lib();
    add_folder(&mut h.lib, "F-A", "档案甲", None, None, 0);
    add_folder(&mut h.lib, "F-B", "档案乙", None, None, 1);
    add_folder(&mut h.lib, "F-C", "普通", Some("F-A"), None, 0);
    for id in ["F-A", "F-B"] {
      h.lib.folders.folders.iter_mut().find(|f| f.id == id).unwrap().kind = Some(ARCHIVE_KIND.into());
      h.lib.index.folders.get_mut(id).unwrap().kind = Some(ARCHIVE_KIND.into());
    }
    add_asset(&mut h.lib, "AN-A");
    add_placement(&mut h.lib, "AN-A", "F-C", "a.fla", true); // 链上：F-A
    add_placement(&mut h.lib, "AN-A", "F-A", "b.fla", false); // 档案内直接：F-A（去重）
    add_placement(&mut h.lib, "AN-A", "F-B", "c.fla", false); // F-B

    let d = asset_detail(&h.lib, "AN-A").unwrap();
    assert_eq!(
      d.archives.iter().map(|a| (a.folder_id.as_str(), a.name.as_str())).collect::<Vec<_>>(),
      vec![("F-A", "档案甲"), ("F-B", "档案乙")],
      "去重且按首次出现序"
    );
  }

  /// ArchiveDetailView：fileCount = 档案内 Placement 数；totalSize = 资产去重合计；
  /// links / createdAt 透传；camelCase 键名成对（踩坑 031 防线）
  #[test]
  fn archive_detail_view_counts_and_camel_case() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-ARC", "设定集", None, None, 0);
    let mut rec = ArchiveData::new("F-ARC");
    rec.links = vec!["https://ref".into()];
    rec.created_at = "2026-08-15T10:00:00+08:00".into();
    h.lib.archives.archives.push(rec);
    add_asset(&mut h.lib, "AN-A"); // size 100
    add_asset(&mut h.lib, "AN-B"); // size 100
    add_placement(&mut h.lib, "AN-A", "F-ARC", "a.fla", true);
    add_placement(&mut h.lib, "AN-A", "F-ARC", "a-链接.fla", false); // 同资产第二条：计数 +1，体积不重复
    add_placement(&mut h.lib, "AN-B", "F-ARC", "b.fla", true);

    let view = {
      let rec = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-ARC").cloned().unwrap();
      let (file_count, total_size) = archive_stats(&h.lib, "F-ARC");
      ArchiveDetailView {
        folder_id: rec.folder_id,
        badge: rec.badge,
        avatar: rec.avatar,
        props: rec.props,
        descriptions: rec.descriptions,
        links: rec.links,
        created_at: rec.created_at,
        file_count,
        total_size,
        cover_path: None,
        cover_ver: None,
        prompt: None,
      }
    };
    assert_eq!(view.file_count, 3, "Placement 逐条计数");
    assert_eq!(view.total_size, 200, "同资产去重合计");
    assert_eq!(view.links, vec!["https://ref"]);
    assert_eq!(view.created_at, "2026-08-15T10:00:00+08:00");
    // camelCase 键名防线（踩坑 031）
    let v = serde_json::to_value(&view).unwrap();
    let obj = v.as_object().unwrap();
    for key in ["folderId", "createdAt", "fileCount", "totalSize", "props", "descriptions", "links"] {
      assert!(obj.contains_key(key), "缺 camelCase 键 {key}");
    }
    for key in obj.keys() {
      assert!(!key.contains('_'), "出现 snake_case 键 {key}");
    }
  }

  /// ArchiveListItem（list_archives 输出）带 totalSize（2026-08-30 用户定案，同详情口径去重合计）；
  /// camelCase 键名防线（踩坑 031 同类）
  #[test]
  fn archive_list_item_carries_total_size_camel_case() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-ARC", "设定集", None, None, 0);
    h.lib.archives.archives.push(ArchiveData::new("F-ARC"));
    add_asset(&mut h.lib, "AN-A"); // size 100
    add_placement(&mut h.lib, "AN-A", "F-ARC", "a.fla", true);
    add_placement(&mut h.lib, "AN-A", "F-ARC", "a-链接.fla", false); // 同资产第二条：体积不重复计

    let item = archive::list_archives(&h.lib)
      .into_iter()
      .find(|a| a.folder_id == "F-ARC")
      .map(|a| {
        let (cover_path, cover_ver) = archive::cover_file_info(&h.lib, &a.folder_id).unzip();
        ArchiveListItem {
          cover_path,
          cover_ver,
          total_size: archive_stats(&h.lib, &a.folder_id).1,
          folder_id: a.folder_id,
          badge: a.badge,
          avatar: a.avatar,
          props: a.props,
          descriptions: a.descriptions,
          links: a.links,
          created_at: a.created_at,
          prompt: a.prompt,
        }
      })
      .unwrap();
    assert_eq!(item.total_size, 100, "同资产去重合计");
    let v = serde_json::to_value(&item).unwrap();
    let obj = v.as_object().unwrap();
    assert!(obj.contains_key("totalSize"), "缺 camelCase 键 totalSize");
    for key in obj.keys() {
      assert!(!key.contains('_'), "出现 snake_case 键 {key}");
    }
  }

  /// 回收站资产：无存活 Placement 时链接区块兜底「回收站（原位置：区块 / 路径）」（PRD 02-15）
  #[test]
  fn asset_detail_recycle_fallback_entry() {
    let mut h = make_lib();
    h.lib.folders.blocks.push(Block { id: "B01".into(), name: "角色".into(), icon: None, order: 1 });
    add_folder(&mut h.lib, "F-TOP", "顶级", None, Some("B01"), 0);
    add_folder(&mut h.lib, "F-LEAF", "所在", Some("F-TOP"), None, 0);
    add_asset(&mut h.lib, "AN-R");
    // 无存活 Placement，recycle.json 记录删除前位置（F-LEAF 一条 + root 一条取首条）
    h.lib.recycle.items.push(crate::core::models::RecycleItem {
      asset: "AN-R".into(),
      placements: vec![
        Placement { asset: "AN-R".into(), folder: "F-LEAF".into(), name: "男人.fla".into(), order: 0, origin: true },
        Placement { asset: "AN-R".into(), folder: "root".into(), name: "人物.fla".into(), order: 1, origin: false },
      ],
      deleted_at: "2026-08-16T00:00:00+08:00".into(),
    });

    let d = asset_detail(&h.lib, "AN-R").unwrap();
    assert_eq!(d.placements.len(), 1, "兜底一条回收站条目");
    let link = &d.placements[0];
    assert_eq!(link.block_name, "回收站");
    assert_eq!(link.path, vec!["角色 / 顶级 / 所在"], "原位置 = 区块 / 路径");
    assert_eq!(link.folder_id, None, "伪条目不做档案后缀判定");
    assert!(link.origin);
    assert!(d.archives.is_empty(), "回收站资产不做归属档案反查");
  }
}
