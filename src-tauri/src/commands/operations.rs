//! 命令 API：素材基础操作（6.1 选择态 + 基础操作，PRD 02-10）。
//! 命令层只做锁与撤销三步接入（锁序 library → undo，与 commands::undo 同约）：
//! begin(标签) → 写点前 record_file → 业务（services::operations）→ 成功 commit / 失败 cancel；
//! 磁盘副作用（副本 info 目录、originName 改名）成对 record_disk_op。
//! 业务失败一律 cancel + 内存从盘重载（盘上是事实来源，防半截内存态）。
//! 全部 async：涉及磁盘 IO 一律走线程池（规范第 2 条）。

use std::path::PathBuf;

use tauri::State;

use crate::commands::library::AppState;
use crate::core::library::{self, Library, ARCHIVES_FILE, FOLDERS_FILE, RECYCLE_FILE};
use crate::core::models::Folder;
use crate::core::paths;
use crate::core::undo::{DiskOp, UndoManager};
use crate::services::import::ROOT_FOLDER_ID;
use crate::services::operations::{self, ApplyBlockReport, BatchRenameFailed, BlockInput, PlacementRef, RenameItem, SyncLinkAdd, SyncLinkRemove, SyncLinkRename};

/// metadata.json 路径（由 ID 推导，PRD 01-2.3）
fn meta_path(lib: &Library, asset_id: &str) -> PathBuf {
  paths::metadata_file(&lib.assets_dir, asset_id)
}

/// 业务失败收尾：丢弃撤销事务 + 内存从盘重载（防半截内存态），返回错误文案
/// pub(crate)：commands::recycle 还原命令同约复用
pub(crate) fn fail(lib: &mut Library, undo: &mut UndoManager, e: String) -> String {
  undo.cancel();
  if let Err(re) = library::reload(lib) {
    log::error!("操作失败后内存重载失败：{re}");
  }
  e
}

/// 新建文件夹：parentId = null / "root" 为顶层；afterFolderId 给了就插到该兄弟之后，否则末尾；
/// blockId 仅顶层新建生效（写入 Folder.block 并继承区块默认图标，2026-08-14 区块子系统）
#[tauri::command]
pub async fn create_folder(
  state: State<'_, AppState>,
  parent_id: Option<String>,
  name: String,
  after_folder_id: Option<String>,
  block_id: Option<String>,
) -> Result<Folder, String> {
  create_folder_op(&state, parent_id.as_deref(), &name, after_folder_id.as_deref(), block_id.as_deref())
}

/// 新建文件夹整段（锁 library → 锁 undo → 单撤销步，锁序与撤销三步同命令层各操作）；
/// pub(crate)：server/mcp（AI 外接通道二批写工具 create_folder）同用
pub(crate) fn create_folder_op(
  state: &AppState,
  parent_id: Option<&str>,
  name: &str,
  after_folder_id: Option<&str>,
  block_id: Option<&str>,
) -> Result<Folder, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建文件夹");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::create_folder(lib, parent_id, name, after_folder_id, block_id) {
    Ok(folder) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(folder)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 重命名文件夹（2026-07-29 复验三轮）：仅改数据不动磁盘；名称同库名 Windows 规则
/// （非法 / 空名后端拒绝，业务失败 cancel + 内存重载）
#[tauri::command]
pub async fn rename_folder(
  state: State<'_, AppState>,
  folder_id: String,
  new_name: String,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("重命名文件夹为 {}", new_name.trim()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::rename_folder(lib, &folder_id, &new_name) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 更改文件夹图标（2026-08-18，3.3 / 05 稿 §3）：icon = null 恢复默认；
/// 同步语义（用户定，与颜色同口径、无复选框）：链路一致的后代跟随调整，不一致的不动
#[tauri::command]
pub async fn set_folder_icon(
  state: State<'_, AppState>,
  folder_id: String,
  icon: Option<String>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("更改文件夹图标");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::set_folder_icon(lib, &folder_id, icon.as_deref()) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 更改文件夹颜色（2026-08-18，3.3 / 05 稿 §4）：color = null 清为跟随态（无值 = 视觉继承最近
/// 祖先色）；前端「默认」色块传 "default" 哨兵 = 显式默认断链（2026-09-01 用户定）；
/// 同步语义（用户定）：链路一致的后代跟随调整（无色后代不写数据、视觉自动跟随），不一致的不动
#[tauri::command]
pub async fn set_folder_color(
  state: State<'_, AppState>,
  folder_id: String,
  color: Option<String>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("更改文件夹颜色");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::set_folder_color(lib, &folder_id, color.as_deref()) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 更改文件夹颜色（批量，2026-09-01 左侧边栏走查：多选树 item 换色无法批量）：
/// 多选域内触发 → 整个选中集，逐 id 同语义（各自链路一致后代跟随），单撤销步；
/// 陈旧 id 跳过不计，返回实际生效个数
#[tauri::command]
pub async fn set_folder_colors(
  state: State<'_, AppState>,
  folder_ids: Vec<String>,
  color: Option<String>,
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("更改文件夹颜色");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::set_folder_colors(lib, &folder_ids, color.as_deref()) {
    Ok(n) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(n)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 设置文件夹分组（2026-08-19，3.5 二轮 / PRD 02-04）：草稿式一次提交单撤销步；
/// 校验失败（完全相同 / 空规则）前端弹 23 稿错误窗
#[tauri::command]
pub async fn set_folder_groups(
  state: State<'_, AppState>,
  folder_id: String,
  groups: Vec<crate::core::models::GroupDef>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("设置分组");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::set_folder_groups(lib, &folder_id, groups) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 自由组成员归组（2026-09-17，PRD 02-04 无规则空组）：移动至分组 / 移出分组——单撤销步
#[tauri::command]
pub async fn assign_group(
  state: State<'_, AppState>,
  folder_id: String,
  group_id: Option<String>,
  asset_ids: Vec<String>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(if group_id.is_some() { "移动至分组" } else { "移出分组" });
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::assign_group(lib, &folder_id, group_id.as_deref(), &asset_ids) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 移动 Placement 到目标文件夹，返回实际移动条数（去重 / 原地 / 陈旧项跳过不计）；
/// 移出 root 时清 metadata.importedFrom（摘标：移动即转正）
#[tauri::command]
pub async fn move_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
) -> Result<usize, String> {
  move_placements_op(&state, &items, &target_folder)
}

/// 移动整段（锁 library → 锁 undo → 单撤销步；root → 非 root 摘标的 metadata 写点前快照）；
/// pub(crate)：server/mcp（AI 外接通道二批写工具 move_placements）同用
pub(crate) fn move_placements_op(state: &AppState, items: &[PlacementRef], target_folder: &str) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("移动 {} 个项目", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  // 摘标候选（root → 非 root）的 metadata 写点前快照
  if target_folder != ROOT_FOLDER_ID {
    for it in items {
      if it.folder_id == ROOT_FOLDER_ID {
        undo.record_file(&meta_path(lib, &it.asset_id)).map_err(|e| e.to_string())?;
      }
    }
  }
  match operations::move_placements(lib, items, target_folder) {
    Ok(moved) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(moved)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 段内排序：items（须同属一个文件夹）移到 targetFolder 段内 beforeItem 之前（null = 段尾）；
/// 跨文件夹移出 root 时清 metadata.importedFrom（摘标口径与 move_placements 一致）
#[tauri::command]
pub async fn reorder_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
  before_item: Option<PlacementRef>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("调整顺序");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  // 摘标候选（root → 非 root）的 metadata 写点前快照（与 move_placements 同口径）
  if target_folder != ROOT_FOLDER_ID {
    for it in &items {
      if it.folder_id == ROOT_FOLDER_ID {
        undo.record_file(&meta_path(lib, &it.asset_id)).map_err(|e| e.to_string())?;
      }
    }
  }
  match operations::reorder_placements(lib, &items, &target_folder, before_item.as_ref()) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 文件夹同级排序（混排护栏：Folder 与 Placement 各自独立排序，前端分开调）
#[tauri::command]
pub async fn reorder_folders(
  state: State<'_, AppState>,
  folder_ids: Vec<String>,
  parent_id: Option<String>,
  before_folder_id: Option<String>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("调整文件夹顺序");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::reorder_folders(lib, &folder_ids, parent_id.as_deref(), before_folder_id.as_deref()) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 文件夹重设父级（仅改数据不动磁盘）：targetParent = null / "root" 为顶层；
/// beforeFolderId 给了 = 插到该兄弟（targetParent 直接子级）之前（跨父级上/下 1/3 落点定位，
/// 2026-07-29 复验：嵌套文件夹拖出），null = 同级末尾；
/// targetBlock（2026-08-14 区块子系统）：null = 保持原区块；落根级时给值 = 写入 Folder.block
/// （移回「目录」传 "dir"）；落进文件夹内时必须为 null；
/// 循环（移入自身或子孙）整条报错，同父 / 陈旧 ID 跳过，返回实际移动数
#[tauri::command]
pub async fn move_folders(
  state: State<'_, AppState>,
  folder_ids: Vec<String>,
  target_parent: Option<String>,
  before_folder_id: Option<String>,
  target_block: Option<String>,
) -> Result<usize, String> {
  move_folders_op(&state, &folder_ids, target_parent.as_deref(), before_folder_id.as_deref(), target_block.as_deref())
}

/// 文件夹重设父级整段（锁 library → 锁 undo → 单撤销步；循环移入自身或子孙整条报错、
/// 同父 / 陈旧 ID 跳过）；pub(crate)：server/mcp（AI 外接通道二批写工具 move_folders）同用
pub(crate) fn move_folders_op(
  state: &AppState,
  folder_ids: &[String],
  target_parent: Option<&str>,
  before_folder_id: Option<&str>,
  target_block: Option<&str>,
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("移动 {} 个文件夹", folder_ids.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::move_folders(lib, folder_ids, target_parent, before_folder_id, target_block) {
    Ok(moved) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(moved)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 关联：目标文件夹逐条建链接 Placement，返回关联条数；
/// names[i] 非空 ? 自定义显示名 : 源 Placement 名
#[tauri::command]
pub async fn link_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
  names: Vec<Option<String>>,
) -> Result<usize, String> {
  link_placements_op(&state, &items, &target_folder, &names)
}

/// 链接汇总整段（锁 library → 锁 undo → 单撤销步）；pub(crate)：server/mcp
///（AI 外接通道二批写工具 link_placements）同用——资产本体留原地，目标文件夹只建链接
pub(crate) fn link_placements_op(
  state: &AppState,
  items: &[PlacementRef],
  target_folder: &str,
  names: &[Option<String>],
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("关联 {} 个项目", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::link_placements(lib, items, target_folder, names) {
    Ok(linked) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(linked)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 链接管理（17 稿，6.2 三轮）：一个资产的位置增 / 删 / 改一次提交（单撤销步，标签「链接管理」）；
/// origin 改名含磁盘段（登记 DiskOp::Rename + metadata.json 快照）；返回实际变更条数
#[tauri::command]
pub async fn sync_asset_links(
  state: State<'_, AppState>,
  asset_id: String,
  adds: Vec<SyncLinkAdd>,
  removes: Vec<SyncLinkRemove>,
  renames: Vec<SyncLinkRename>,
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("链接管理");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  if renames.iter().any(|r| r.origin) {
    undo.record_file(&meta_path(lib, &asset_id)).map_err(|e| e.to_string())?;
  }
  match operations::sync_asset_links(lib, &asset_id, &adds, &removes, &renames) {
    Ok(disk_renames) => {
      if adds.is_empty() && removes.is_empty() && renames.is_empty() {
        undo.cancel();
        return Ok(0);
      }
      for (old_path, new_path) in &disk_renames {
        undo.record_disk_op(
          DiskOp::Rename { from: new_path.clone(), to: old_path.clone() },
          DiskOp::Rename { from: old_path.clone(), to: new_path.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(adds.len() + removes.len() + renames.len())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 将此文件设为原始文件（6.2 四轮走查③，PRD 01 边界规则 8 手动 origin 转移）：
/// 目标 Placement 置 origin，原 origin 行同时摘标（库内 origin 恒 ≤ 1）；仅标记不动磁盘；
/// name 可选 = 同文件夹重复链接精确到行；已是 origin = 无操作不进撤销栈。撤销标签"设为原始文件"
#[tauri::command]
pub async fn set_placement_origin(
  state: State<'_, AppState>,
  asset_id: String,
  folder_id: String,
  name: Option<String>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("设为原始文件");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::set_placement_origin(lib, &asset_id, &folder_id, name.as_deref()) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 创建副本：info 目录整体复制 + 新资源实体，返回新 assetId 清单；
/// 磁盘副作用成对登记（undo = 删除新 info 目录 / redo = 复制还原）
#[tauri::command]
pub async fn duplicate_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
  names: Vec<Option<String>>,
) -> Result<Vec<String>, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("创建 {} 个副本", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::duplicate_placements(lib, &items, &target_folder, &names) {
    Ok(out) => {
      for (old_info, new_info) in &out.copies {
        undo.record_disk_op(
          DiskOp::Delete(new_info.clone()),
          DiskOp::Copy { from: old_info.clone(), to: new_info.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(out.new_ids)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 克隆文件夹报告（3.4）：新根 id 清单 + 节点/资产计数（前端 toast 计数用）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneReport {
  pub folder_ids: Vec<String>,
  pub folder_count: usize,
  pub asset_count: usize,
}

/// 克隆文件夹（3.4，PRD 02-13 复制克隆；批量克隆 = folder_ids 多棵，2026-08-18 用户定）：
/// 递归复制子树（结构 / 资产物理副本 / 档案记录），单撤销步；
/// 撤销磁盘段 = 新 info 目录 Delete（重做 = 从源再 Copy，源不动可重放——同 duplicate 口径）
#[tauri::command]
pub async fn clone_folders(
  state: State<'_, AppState>,
  folder_ids: Vec<String>,
) -> Result<CloneReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  if folder_ids.len() == 1 {
    let name = lib.index.folders.get(folder_ids[0].as_str()).map(|f| f.name.clone()).unwrap_or_else(|| folder_ids[0].clone());
    undo.begin(format!("克隆文件夹 {name}"));
  } else {
    undo.begin(format!("克隆 {} 个文件夹", folder_ids.len()));
  }
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match operations::clone_folders(lib, &folder_ids) {
    Ok(out) => {
      for (old_info, new_info) in &out.copies {
        undo.record_disk_op(
          DiskOp::Delete(new_info.clone()),
          DiskOp::Copy { from: old_info.clone(), to: new_info.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(CloneReport { folder_ids: out.folder_ids, folder_count: out.folder_count, asset_count: out.asset_count })
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 新建文档入库（3.1 二轮，07 新增菜单"新建文档"组固定七类）：kind = md/txt/fla/psd/ai/docx/xlsx；
/// 撤销 = info 目录移入 cache/tmp 备份（DiskOp::Rename），重做 = 移回（Delete 会丢内容无法重做）
#[tauri::command]
pub async fn create_document(
  state: State<'_, AppState>,
  kind: String,
  target_folder: Option<String>,
) -> Result<crate::services::views::PlacementView, String> {
  create_document_op(&state, &kind, target_folder.as_deref())
}

/// 新建文档整段（锁 library → 锁 undo → begin/record_file/服务调用/record_disk_op/commit，
/// 锁序与撤销三步同命令层各操作）；pub(crate)：server/mcp（AI 外接通道二期写工具）同用
pub(crate) fn create_document_op(
  state: &AppState,
  kind: &str,
  target_folder: Option<&str>,
) -> Result<crate::services::views::PlacementView, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建文档".to_string());
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::create_document(lib, kind, target_folder, None) {
    Ok((view, info_dir, backup)) => {
      undo.record_disk_op(
        DiskOp::Rename { from: info_dir.clone(), to: backup.clone() },
        DiskOp::Rename { from: backup, to: info_dir },
      );
      undo.commit().map_err(|e| e.to_string())?;
      Ok(view)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 重命名 Placement：显示名即时改；原始 Placement 追加磁盘段（originName + 词干缩略图
/// 同步改名，成对登记 DiskOp::Rename——注意 record_disk_op(undo_op, redo_op) 参数顺序）；
/// 链接 Placement 仅改显示名
#[tauri::command]
pub async fn rename_placement(
  state: State<'_, AppState>,
  asset_id: String,
  folder_id: String,
  origin: bool,
  new_name: String,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("重命名为 {}", new_name.trim()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  if origin {
    undo.record_file(&meta_path(lib, &asset_id)).map_err(|e| e.to_string())?;
  }
  match operations::rename_placement(lib, &asset_id, &folder_id, origin, &new_name) {
    Ok(renames) => {
      for (old_path, new_path) in &renames {
        undo.record_disk_op(
          DiskOp::Rename { from: new_path.clone(), to: old_path.clone() },
          DiskOp::Rename { from: old_path.clone(), to: new_path.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 批量重命名报告（3.4，37 稿）：成功计数 + 失败清单（新名 + 原因）——失败项跳过不整体回滚（PRD 02-14）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRenameReport {
  pub renamed: usize,
  pub failed: Vec<BatchRenameFailed>,
}

/// 批量重命名（3.4）：逐条改名（撞名不唯一化——显示名放开 / 失败跳过汇总），单撤销步
///（folders.json + 各 origin 项 metadata.json 快照 + 磁盘改名对成对登记，同单条 rename 口径）
#[tauri::command]
pub async fn rename_placements(state: State<'_, AppState>, items: Vec<RenameItem>) -> Result<BatchRenameReport, String> {
  rename_placements_op(&state, &items)
}

/// 批量重命名整段（锁 library → 锁 undo → 单撤销步）；pub(crate)：server/mcp
///（AI 外接通道二期写工具）同用——失败项跳过不整体回滚语义对 MCP 同样生效
pub(crate) fn rename_placements_op(state: &AppState, items: &[RenameItem]) -> Result<BatchRenameReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("批量重命名 {} 个项目", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  for it in items {
    // origin 经 resolve_origin 消歧（2026-09-20）：MCP 传 None 时锁内按唯一记录判定，
    // 漏判会漏 record metadata 快照（撤销后 originName 不回滚）
    if operations::resolve_origin(lib, it) == Some(true) {
      undo.record_file(&meta_path(lib, &it.asset_id)).map_err(|e| e.to_string())?;
    }
  }
  match operations::rename_placements(lib, items) {
    Ok(out) => {
      for (old_path, new_path) in &out.renames {
        undo.record_disk_op(
          DiskOp::Rename { from: new_path.clone(), to: old_path.clone() },
          DiskOp::Rename { from: old_path.clone(), to: new_path.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(BatchRenameReport { renamed: out.renamed, failed: out.failed })
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 删除 Placement（3.4 八轮语义简化，2026-08-19 用户定"护栏拆干净"）：**位置级**——
/// "uncategorized" = 从文件夹中移除（挪未分类，不删记录）/ "recycle" = 丢到回收站
/// （recycle.json 按资产聚合记录，info 原地不动）；origin / 链接同路径，无"最后一条"两段式、
/// 无 origin 自动转移（手动转移归阶段六「设为原始文件」）。撤销标签按 action 分流
#[tauri::command]
pub async fn delete_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  last_action: Option<String>,
) -> Result<(), String> {
  delete_placements_op(&state, &items, last_action.as_deref())
}

/// 删除 Placement 整段（锁 library → 锁 undo → 单撤销步）；pub(crate)：server/mcp
///（AI 外接通道三批 trash_placements）同用——MCP 只传 Some("recycle")，
/// "uncategorized" 仅 APP 内
pub(crate) fn delete_placements_op(state: &AppState, items: &[PlacementRef], last_action: Option<&str>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  // 撤销标签分流（3.4 八轮，2026-08-18）：从文件夹中移除（去未分类）/ 丢到回收站
  let label = match last_action {
    Some("uncategorized") => format!("从文件夹中移除 {} 个项目", items.len()),
    _ => format!("丢到回收站 {} 个项目", items.len()),
  };
  undo.begin(label);
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  if last_action == Some("recycle") {
    undo.record_file(&lib.root.join(RECYCLE_FILE)).map_err(|e| e.to_string())?;
  }
  match operations::delete_placements(lib, items, last_action) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 区块管理草稿一次提交（2026-08-14 区块子系统，30 稿确认/取消）：blocks = 保留列表
/// （数组序 = 显示顺序，id = null 为新建），deleted = 待删区块 id；
/// deleted 含非空区块且 confirmed = false 时返回 needConfirm 不动数据（询问段不进撤销栈）
#[tauri::command]
pub async fn apply_block_changes(
  state: State<'_, AppState>,
  blocks: Vec<BlockInput>,
  deleted: Vec<String>,
  confirmed: bool,
) -> Result<ApplyBlockReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("区块管理");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::apply_block_changes(lib, blocks, deleted, confirmed) {
    // 询问段未动数据：丢弃事务，不进撤销栈
    Ok(report) if report.need_confirm => {
      undo.cancel();
      Ok(report)
    }
    Ok(report) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(report)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 区块拖拽排序即时提交（2026-08-14 区块子系统）：ids 须恰好覆盖现有全部区块
#[tauri::command]
pub async fn reorder_blocks(state: State<'_, AppState>, ids: Vec<String>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("排序区块");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::reorder_blocks(lib, ids) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}
