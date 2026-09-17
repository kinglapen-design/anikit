//! 文件夹操作（6.1）：新建（支持插到指定兄弟之后）、同级排序与重设父级。
//! Folder 展示按 order 排序（commands::import::folder_tree），排序只重写同级 order 为连续序号。

use std::collections::HashSet;

use crate::core::library::{self, Library};
use crate::core::models::{Folder, GroupDef, GroupRule, ARCHIVE_KIND};
use crate::services::import as ops;
use crate::services::import::ROOT_FOLDER_ID;

use super::{check_folder, parent_key};

/// 新建文件夹：after_folder_id 给了就插到该兄弟之后（同级后续 order 顺移 +1），否则末尾。
/// 名称走库名同一套 Windows 命名规则（前后端同规则，命令接口统一把关）。
/// block_id 仅顶级新建（parent = None / "root"）生效（2026-08-14 区块子系统）：写入 Folder.block，
/// 且所在区块有默认图标时继承到 Folder.icon；非顶级新建忽略该参数
pub fn create_folder(
  lib: &mut Library,
  parent: Option<&str>,
  name: &str,
  after_folder_id: Option<&str>,
  block_id: Option<&str>,
) -> Result<Folder, String> {
  let name = name.trim();
  library::validate_library_name(name).map_err(|e| e.to_string())?;
  let parent_key = parent_key(parent);
  if let Some(p) = &parent_key {
    match lib.index.folders.get(p) {
      None => return Err("父文件夹不存在，可能已被删除".to_string()),
      // 档案层级约束（2026-08-15 档案子系统，PRD 02_文件整理 03）：档案内不允许子级文件夹
      Some(f) if f.kind.as_deref() == Some(ARCHIVE_KIND) => return Err("档案下不允许出现子级文件夹".to_string()),
      _ => {}
    }
  }
  // 区块归属与默认图标继承（仅顶级；block_id 指向不存在区块报错）
  let (block, icon) = match (&parent_key, block_id) {
    (None, Some(b)) => {
      let blk = lib
        .folders
        .blocks
        .iter()
        .find(|x| x.id == b)
        .ok_or_else(|| "目标区块不存在，可能已被删除".to_string())?;
      (Some(b.to_string()), blk.icon.clone())
    }
    _ => (None, None),
  };
  let mut folder = match after_folder_id {
    None => {
      let id = ops::create_folder(lib, parent.unwrap_or(ROOT_FOLDER_ID), name);
      lib.index.folders[&id].clone()
    }
    Some(after_id) => {
      let anchor = lib
        .folders
        .folders
        .iter()
        .find(|f| f.id == after_id && f.parent == parent_key)
        .ok_or_else(|| "插入位置无效：目标兄弟文件夹不存在".to_string())?;
      let insert_order = anchor.order + 1;
      for f in &mut lib.folders.folders {
        if f.parent == parent_key && f.order >= insert_order {
          f.order += 1;
        }
      }
      let folder =
        Folder { id: ops::new_folder_id(), name: name.to_string(), parent: parent_key, block: None, icon: None, kind: None, color: None, groups: None, order: insert_order };
      lib.folders.folders.push(folder.clone());
      lib.index.folders.insert(folder.id.clone(), folder.clone());
      folder
    }
  };
  // 区块归属写回（两条创建路径统一在此落 block/icon；均为 None 时无操作）
  if block.is_some() || icon.is_some() {
    folder.block = block;
    folder.icon = icon;
    if let Some(f) = lib.folders.folders.iter_mut().find(|f| f.id == folder.id) {
      f.block = folder.block.clone();
      f.icon = folder.icon.clone();
    }
    lib.index.folders.insert(folder.id.clone(), folder.clone());
  }
  sync_folder_index(lib); // 顺移过 order 的兄弟同步进索引
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("新建文件夹：{}（{:?} 下）", folder.name, folder.parent);
  Ok(folder)
}

/// 同级排序：folder_ids（须同属 parent 层级）整体移到 before_folder_id 之前（None = 末尾），
/// 随后同级 order 重写为连续 0..n。混排护栏：Folder 与 Placement 各自独立排序（前端分开调）
pub fn reorder_folders(
  lib: &mut Library,
  folder_ids: &[String],
  parent: Option<&str>,
  before_folder_id: Option<&str>,
) -> Result<(), String> {
  if folder_ids.is_empty() {
    return Ok(());
  }
  let parent_key = parent_key(parent);
  // 同级兄弟的 vec 下标，按 order 排序（当前显示顺序）
  let mut siblings: Vec<usize> = lib
    .folders
    .folders
    .iter()
    .enumerate()
    .filter(|(_, f)| f.parent == parent_key)
    .map(|(i, _)| i)
    .collect();
  siblings.sort_by_key(|&i| lib.folders.folders[i].order);

  let mut moving: Vec<usize> = Vec::with_capacity(folder_ids.len());
  let mut seen = HashSet::new();
  for id in folder_ids {
    if !seen.insert(id) {
      return Err("调整列表存在重复文件夹".to_string());
    }
    let pos = siblings
      .iter()
      .position(|&i| lib.folders.folders[i].id == *id)
      .ok_or_else(|| "文件夹不在该层级，可能已被移动".to_string())?;
    moving.push(siblings[pos]);
  }
  let moving_set: HashSet<usize> = moving.iter().copied().collect();
  let remaining: Vec<usize> = siblings.iter().copied().filter(|i| !moving_set.contains(i)).collect();

  let insert_at = match before_folder_id {
    None => remaining.len(),
    Some(before) => remaining
      .iter()
      .position(|&i| lib.folders.folders[i].id == *before)
      .ok_or_else(|| "插入位置无效：目标兄弟文件夹不存在".to_string())?,
  };
  let mut sequence = remaining[..insert_at].to_vec();
  sequence.extend(moving);
  sequence.extend(&remaining[insert_at..]);
  for (n, &i) in sequence.iter().enumerate() {
    lib.folders.folders[i].order = n as i32;
  }
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())
}

/// 文件夹重命名（2026-07-29 复验三轮）：仅改数据不动磁盘（Folder 是数据层分组，无磁盘实体）。
/// 名称走库名同一套 Windows 命名规则（validate_library_name）；同名 = 幂等无操作
pub fn rename_folder(lib: &mut Library, folder_id: &str, new_name: &str) -> Result<(), String> {
  let name = new_name.trim();
  library::validate_library_name(name).map_err(|e| e.to_string())?;
  let folder = lib
    .folders
    .folders
    .iter_mut()
    .find(|f| f.id == folder_id)
    .ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
  if folder.name == name {
    return Ok(());
  }
  folder.name = name.to_string();
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("重命名文件夹 {folder_id} 为 {name}");
  Ok(())
}

/// Folder 数据改动后索引同步（folders.folders 为事实来源，全量重建映射）
fn sync_folder_index(lib: &mut Library) {
  lib.index.folders = lib.folders.folders.iter().map(|f| (f.id.clone(), f.clone())).collect();
}

/// 更改文件夹图标（2026-08-18，3.3 / 05 稿 §3）：icon None = 恢复默认文件夹图标。
/// 同步语义（2026-08-18 用户定，与颜色同口径——默认影响子级，无「应用至子级」复选框）：
/// 后代图标与本节点当前图标一致的跟随调整（含默认图标 None == None——未区分子级首次统一即靠此）；
/// 不一致的（用户单独区分过）不动且不再下钻（其子树链路已被它接管）；
/// 档案后代恒跳过（图标强制 archive，PRD 02-54）
pub fn set_folder_icon(lib: &mut Library, folder_id: &str, icon: Option<&str>) -> Result<(), String> {
  let old_chain = lib
    .index
    .folders
    .get(folder_id)
    .ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?
    .icon
    .clone();
  let value = icon.map(|s| s.to_string());
  // 图标无视觉跟随（无图标 = 默认图标，与有色父级即"不一样"）：None 不穿透；档案跳过
  let targets = sync_follow_targets(lib, folder_id, &old_chain, |f| &f.icon, false, true);
  for f in &mut lib.folders.folders {
    if targets.contains(&f.id) {
      f.icon = value.clone();
    }
  }
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("更改文件夹图标 {folder_id} → {icon:?}（链路一致后代跟随）");
  Ok(())
}

/// 更改文件夹颜色（2026-08-18，3.3 / 05 稿 §4）：color None = 清为跟随态（无值 = 视觉继承最近祖先色）。
/// 「默认」色块 = 显式默认——前端传 "default" 哨兵（2026-09-01 用户定：子级选默认要能断开父级
/// 链路退回常态灰，直接清 None 会被父级色盖回）；哨兵在同步语义上按普通链路值对待。
/// 同步语义（2026-08-18 用户定）：默认影响子级——后代颜色与本节点旧链路色一致的跟随调整
///（用户：子级改回与父级一致后要能重新跟随），不一致的不动且不再下钻；
/// 无色后代不写数据（视觉自动跟随底色版本 --custom-{色}-bg）但链路穿透继续下钻
pub fn set_folder_color(lib: &mut Library, folder_id: &str, color: Option<&str>) -> Result<(), String> {
  apply_folder_color(lib, folder_id, color)?;
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("更改文件夹颜色 {folder_id} → {color:?}（链路一致后代跟随）");
  Ok(())
}

/// 更改文件夹颜色（批量，2026-09-01 左侧边栏走查：多选树 item 换色无法批量）：
/// 多选域内触发 → 整个选中集逐 id 走单节点同语义（各自链路一致后代跟随），单次落盘单撤销步；
/// 陈旧 id 跳过不计，全部不存在才报错
pub fn set_folder_colors(lib: &mut Library, folder_ids: &[String], color: Option<&str>) -> Result<usize, String> {
  let mut n = 0;
  for id in folder_ids {
    if !lib.index.folders.contains_key(id.as_str()) {
      continue; // 陈旧 id（选中后又被删等）跳过
    }
    apply_folder_color(lib, id, color)?;
    n += 1;
  }
  if n == 0 {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("批量更改文件夹颜色 {} 个 → {color:?}", n);
  Ok(n)
}

/// set_folder_color 的不落盘内层（批量复用）：目标 + 链路一致跟随集改写 + 索引同步
fn apply_folder_color(lib: &mut Library, folder_id: &str, color: Option<&str>) -> Result<(), String> {
  let folder = lib
    .index
    .folders
    .get(folder_id)
    .ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
  // 旧链路色 = 本节点色 ?? 祖先链最近色（本节点无色时显示的是祖先色）
  let mut old_chain = folder.color.clone();
  let mut cur = folder.parent.clone();
  while old_chain.is_none() {
    match cur.and_then(|id| lib.index.folders.get(&id).cloned()) {
      Some(f) => {
        old_chain = f.color.clone();
        cur = f.parent.clone();
      }
      None => break,
    }
  }
  let value = color.map(|s| s.to_string());
  let targets = sync_follow_targets(lib, folder_id, &old_chain, |f| &f.color, true, false);
  for f in &mut lib.folders.folders {
    if targets.contains(&f.id) {
      f.color = value.clone();
    }
  }
  sync_folder_index(lib);
  Ok(())
}

/// 设置文件夹分组（2026-08-19，3.5 二轮 / PRD 02-04，33 稿管理 / 34 稿新建 / 12 稿菜单统一写点）：
/// 草稿式一次提交；空数组 = 不分组（存 None 不写字段）。校验：**规则组**规则范围非空 +
/// 规则集合完全相同的两组禁止保存（23 稿"无法保存，存在完全相同的分组"；名称不查重——2026-08-19 用户定）；
/// **rules 为空 = 自由组（2026-09-17，PRD 02-04 无规则空组）放开**——无规则可比，查重只管规则组；
/// 单规则组互斥转移由前端草稿内完成（确认前取消全恢复），后端不再处理
pub fn set_folder_groups(lib: &mut Library, folder_id: &str, groups: Vec<GroupDef>) -> Result<(), String> {
  if !lib.index.folders.contains_key(folder_id) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  for g in &groups {
    for r in &g.rules {
      if r.values.is_empty() {
        return Err(format!("分组「{}」存在未选择范围的规则", g.name));
      }
    }
  }
  // 完全相同判定：仅规则组之间（规则集合 = kind + 排序后 values，组序 / 规则序无关；名称不查重；
  // 自由组 rules 为空直接跳过——2026-09-17 自由组无规则可比）
  let norm = |rules: &[GroupRule]| {
    let mut v: Vec<(String, Vec<String>)> = rules
      .iter()
      .map(|r| {
        let mut vals = r.values.clone();
        vals.sort();
        (r.kind.clone(), vals)
      })
      .collect();
    v.sort();
    v
  };
  for i in 0..groups.len() {
    if groups[i].rules.is_empty() {
      continue;
    }
    for j in (i + 1)..groups.len() {
      if groups[j].rules.is_empty() {
        continue;
      }
      if norm(&groups[i].rules) == norm(&groups[j].rules) {
        return Err("无法保存，存在完全相同的分组".to_string());
      }
    }
  }
  let count = groups.len();
  let value = if groups.is_empty() { None } else { Some(groups) };
  for f in &mut lib.folders.folders {
    if f.id == folder_id {
      f.groups = value;
      break;
    }
  }
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("设置文件夹分组 {folder_id}（{count} 组）");
  Ok(())
}

/// 自由组成员归组（2026-09-17，PRD 02-04 无规则空组）：把 assetIds **移入**目标自由组
///（移动语义 = 先从本文件夹全部自由组摘除再加入，成员身份唯一）；group_id = None = 移出
///（摘除成员身份，回未分组或按规则重估）。目标组必须存在且为自由组（rules 为空）；
/// 资产存在性不校验（显示层成员身份，陈旧 id 随编辑/开库自然清）
pub fn assign_group(lib: &mut Library, folder_id: &str, group_id: Option<&str>, asset_ids: &[String]) -> Result<(), String> {
  if asset_ids.is_empty() {
    return Ok(());
  }
  let folder = lib
    .folders
    .folders
    .iter_mut()
    .find(|f| f.id == folder_id)
    .ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
  let groups = folder.groups.get_or_insert_with(Vec::new);
  if let Some(gid) = group_id {
    let g = groups.iter().find(|g| g.id == gid).ok_or_else(|| "目标分组不存在".to_string())?;
    if !g.rules.is_empty() {
      return Err("只能移入自由组（无规则空组）".to_string());
    }
  }
  let want: std::collections::HashSet<&String> = asset_ids.iter().collect();
  for g in groups.iter_mut() {
    if let Some(m) = &mut g.members {
      m.retain(|a| !want.contains(a));
    }
  }
  if let Some(gid) = group_id {
    let g = groups.iter_mut().find(|g| g.id == gid).expect("前面已校验存在");
    let m = g.members.get_or_insert_with(Vec::new);
    for a in asset_ids {
      if !m.contains(a) {
        m.push(a.clone());
      }
    }
  }
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("自由组归组 {folder_id} → {group_id:?}（{} 项）", asset_ids.len());
  Ok(())
}

/// 链路一致跟随目标集（含本节点）：DFS——子级字段值 == old_chain 则跟随（入集）并继续下钻；
/// != 则整支跳过（用户单独区分过，链路被它接管）。
/// penetrate_none = true 时无色（None）后代**不入集**（不写数据，视觉自动跟随）但继续下钻
///（颜色专用——图标无视觉跟随，None = 默认图标与有色父级即"不一样"，无穿透）；
/// skip_archive = true 时档案后代整支跳过（图标专用——档案图标强制 archive，写入无意义）
fn sync_follow_targets(
  lib: &Library,
  folder_id: &str,
  old_chain: &Option<String>,
  field: for<'a> fn(&'a Folder) -> &'a Option<String>,
  penetrate_none: bool,
  skip_archive: bool,
) -> HashSet<String> {
  let mut set = HashSet::from([folder_id.to_string()]);
  let mut stack = vec![folder_id.to_string()];
  while let Some(pid) = stack.pop() {
    for f in lib.folders.folders.iter().filter(|f| f.parent.as_deref() == Some(pid.as_str())) {
      if skip_archive && f.kind.as_deref() == Some(ARCHIVE_KIND) {
        continue;
      }
      let v = field(f);
      if penetrate_none && v.is_none() {
        stack.push(f.id.clone()); // 无色穿透：不写数据（视觉自动跟随），链路继续下钻
      } else if *v == *old_chain {
        set.insert(f.id.clone());
        stack.push(f.id.clone());
      }
    }
  }
  set
}

/// 文件夹重设父级（仅改数据不动磁盘）：target_parent None / "root" = 顶层。
/// 护栏：目标是自身或子孙 → 循环整条报错；同父 / 陈旧 ID 跳过不计数。
/// 落点：before_folder_id 给了 = 插到该兄弟（须为 target_parent 直接子级）之前——
///       同级 order >= 锚点的兄弟顺移 +移动数，移动组从锚点 order 起按传入顺序连续分配
///       （跨父级的上/下 1/3 落点定位，2026-07-29 复验：嵌套文件夹拖出）；
///       None = 排到目标父级现有子文件夹末尾（同级 max+1 起连续分配）
/// target_block（2026-08-14 区块子系统）三态：None = 保持原区块不动；Some(id) = 落根级时
/// 写入 Folder.block（移回「目录」传 Some("dir")）；落进文件夹内（target_parent 有值）时必须为 None
pub fn move_folders(
  lib: &mut Library,
  folder_ids: &[String],
  target_parent: Option<&str>,
  before_folder_id: Option<&str>,
  target_block: Option<&str>,
) -> Result<usize, String> {
  let target_key = parent_key(target_parent);
  if let Some(t) = &target_key {
    check_folder(lib, t)?;
    // 档案层级约束（2026-08-15 档案子系统）：档案不作为父级并入；档案自身当移动 / 排序对象不受限
    if lib.index.folders.get(t).map(|f| f.kind.as_deref()) == Some(Some(ARCHIVE_KIND)) {
      return Err("档案下不允许出现子级文件夹".to_string());
    }
    if target_block.is_some() {
      return Err("移入文件夹内不能同时指定目标区块".to_string());
    }
  }
  if let Some(b) = target_block {
    if !lib.folders.blocks.iter().any(|x| x.id == b) {
      return Err("目标区块不存在，可能已被删除".to_string());
    }
  }
  // 锚点校验先于落点（先全量验证后落点）：须为 target_parent 的直接子级
  let anchor_order = match before_folder_id {
    None => None,
    Some(before) => Some(
      lib
        .index
        .folders
        .get(before)
        .filter(|f| f.parent == target_key)
        .map(|f| f.order)
        .ok_or_else(|| "插入位置无效：目标兄弟文件夹不存在".to_string())?,
    ),
  };
  let mut seen = HashSet::new();
  let mut moving: Vec<String> = Vec::new();
  for id in folder_ids {
    if !seen.insert(id) {
      continue;
    }
    let folder = match lib.index.folders.get(id) {
      Some(f) => f,
      None => continue, // 陈旧 ID 跳过
    };
    if folder.parent == target_key {
      // 原地移动跳过；但落根级且指定了不同目标区块时不算原地——拖到区块标题 =
      // 移入该区块根级末尾（2026-08-14 区块子系统），照常走落点改区块归属
      let block_change = target_key.is_none() && target_block.is_some() && folder.block.as_deref() != target_block;
      if !block_change {
        continue;
      }
    }
    if is_self_or_descendant(lib, id, target_key.as_deref()) {
      return Err("不能移动到自身或其子文件夹内".to_string());
    }
    moving.push(id.clone());
  }
  if moving.is_empty() {
    return Ok(0);
  }
  place_moved(lib, &moving, &target_key, anchor_order);
  // 落根级且指定目标区块 → 写入 Folder.block（2026-08-14 区块子系统）
  if target_key.is_none() {
    if let Some(b) = target_block {
      for f in &mut lib.folders.folders {
        if moving.contains(&f.id) {
          f.block = Some(b.to_string());
        }
      }
    }
  }
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("移动 {} 个文件夹到 {:?}", moving.len(), target_key);
  Ok(moving.len())
}

/// 落点分配：anchor_order Some = 插到锚点前（同级 order >= 锚点的兄弟顺移 +移动数，
/// 移动组从锚点 order 起连续分配）；None = 末尾（同级 max+1 起，按传入顺序连续分配）
fn place_moved(lib: &mut Library, moving: &[String], target_key: &Option<String>, anchor_order: Option<i32>) {
  let base = match anchor_order {
    Some(base) => {
      let shift = moving.len() as i32;
      for f in &mut lib.folders.folders {
        if f.parent == *target_key && f.order >= base {
          f.order += shift;
        }
      }
      base
    }
    None => {
      lib.folders.folders.iter().filter(|f| f.parent == *target_key).map(|f| f.order).max().unwrap_or(-1) + 1
    }
  };
  for (i, id) in moving.iter().enumerate() {
    if let Some(f) = lib.folders.folders.iter_mut().find(|f| &f.id == id) {
      f.parent = target_key.clone();
      f.order = base + i as i32;
    }
  }
}

/// target 是否为 folder_id 自身或其子孙（沿 target 父链上溯，命中即循环）
fn is_self_or_descendant(lib: &Library, folder_id: &str, target: Option<&str>) -> bool {
  let mut cur = target.map(|s| s.to_string());
  while let Some(id) = cur {
    if id == folder_id {
      return true;
    }
    cur = lib.index.folders.get(&id).and_then(|f| f.parent.clone());
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{FOLDERS_FILE};
  use crate::core::models::{Block, FoldersFile, DEFAULT_BLOCK_ID};
  use crate::core::undo::UndoManager;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str, parent: Option<&str>, order: i32) {
    let folder = Folder { id: id.into(), name: name.into(), parent: parent.map(|s| s.into()), block: None, icon: None, kind: None, color: None, groups: None, order };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn top_orders(lib: &Library) -> Vec<(&str, i32)> {
    let mut v: Vec<(&str, i32)> = lib
      .folders
      .folders
      .iter()
      .filter(|f| f.parent.is_none())
      .map(|f| (f.id.as_str(), f.order))
      .collect();
    v.sort_by_key(|(_, o)| *o);
    v
  }

  #[test]
  fn create_folder_appends_at_end() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);
    let f = create_folder(&mut h.lib, None, "动画", None, None).unwrap();
    assert!(f.id.starts_with('F'));
    assert_eq!((f.parent, f.order), (None, 1), "末尾 order = 同级 max+1");
    assert!(h.lib.index.folders.contains_key(&f.id), "索引同步");
    // "root" 视为顶层；落盘验证
    let f2 = create_folder(&mut h.lib, Some("root"), "参考", None, None).unwrap();
    assert_eq!((f2.parent, f2.order), (None, 2));
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.len(), 3);
  }

  #[test]
  fn create_folder_inserts_after_sibling() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F03", "丙", None, 2);
    let f = create_folder(&mut h.lib, None, "新", Some("F01"), None).unwrap();
    assert_eq!(f.order, 1, "插到 F01 之后");
    assert_eq!(top_orders(&h.lib), vec![("F01", 0), (f.id.as_str(), 1), ("F02", 2), ("F03", 3)], "后续兄弟顺移 +1");
    // 子层级互不影响
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    let child = create_folder(&mut h.lib, Some("F01"), "子2", Some("F01-1"), None).unwrap();
    assert_eq!(child.parent.as_deref(), Some("F01"));
    // 兄弟不存在 / 父不存在 / 非法名称
    assert!(create_folder(&mut h.lib, None, "x", Some("F-不存在"), None).is_err());
    assert!(create_folder(&mut h.lib, Some("F-不存在"), "x", None, None).is_err());
    assert!(create_folder(&mut h.lib, None, "坏:名", None, None).is_err());
    assert!(create_folder(&mut h.lib, None, "   ", None, None).is_err());
  }

  #[test]
  fn reorder_folders_moves_before_and_compacts_orders() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F03", "丙", None, 2);

    // F03 移到 F01 之前
    reorder_folders(&mut h.lib, &["F03".into()], None, Some("F01")).unwrap();
    assert_eq!(top_orders(&h.lib), vec![("F03", 0), ("F01", 1), ("F02", 2)]);
    assert_eq!(h.lib.index.folders["F03"].order, 0, "索引同步");

    // 多选移到末尾（before = None）
    reorder_folders(&mut h.lib, &["F03".into(), "F01".into()], Some("root"), None).unwrap();
    assert_eq!(top_orders(&h.lib), vec![("F02", 0), ("F03", 1), ("F01", 2)]);

    // 护栏：不属于该层级 / 插入点无效 / 重复
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    assert!(reorder_folders(&mut h.lib, &["F01-1".into()], None, None).is_err());
    assert!(reorder_folders(&mut h.lib, &["F01".into()], None, Some("F-不存在")).is_err());
    assert!(reorder_folders(&mut h.lib, &["F01".into(), "F01".into()], None, None).is_err());
  }

  #[test]
  fn move_folders_basic_moves() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    // 顶层 → 子级
    let n = move_folders(&mut h.lib, &["F02".into()], Some("F01"), None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F02"].parent.as_deref(), Some("F01"), "索引同步");
    assert_eq!(h.lib.index.folders["F02"].order, 1, "排到目标子级末尾");

    // 子级 → 顶层（None = 顶层）
    let n = move_folders(&mut h.lib, &["F02".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!((h.lib.index.folders["F02"].parent.clone(), h.lib.index.folders["F02"].order), (None, 1));

    // 跨父级 + "root" 视为顶层
    let n = move_folders(&mut h.lib, &["F01-1".into()], Some("root"), None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].parent, None);
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    let f = disk.folders.iter().find(|f| f.id == "F01-1").unwrap();
    assert_eq!(f.parent, None);
  }

  #[test]
  fn move_folders_appends_continuous_orders_in_input_order() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-T", "目标", None, 0);
    add_folder(&mut h.lib, "F-A", "甲", None, 1);
    add_folder(&mut h.lib, "F-B", "乙", None, 2);
    add_folder(&mut h.lib, "F-C1", "子1", Some("F-T"), 0);
    add_folder(&mut h.lib, "F-C2", "子2", Some("F-T"), 1);

    let n = move_folders(&mut h.lib, &["F-B".into(), "F-A".into()], Some("F-T"), None, None).unwrap();
    assert_eq!(n, 2);
    assert_eq!(h.lib.index.folders["F-B"].order, 2, "同级 max+1 起");
    assert_eq!(h.lib.index.folders["F-A"].order, 3, "按传入顺序连续分配");
    assert_eq!(h.lib.index.folders["F-C2"].order, 1, "原有子级不动");
  }

  #[test]
  fn move_folders_inserts_before_anchor_and_shifts_siblings() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-T", "目标", None, 0);
    add_folder(&mut h.lib, "F-X", "甲", None, 1);
    add_folder(&mut h.lib, "F-Y", "乙", None, 2);
    add_folder(&mut h.lib, "F-C1", "子1", Some("F-T"), 0);
    add_folder(&mut h.lib, "F-C2", "子2", Some("F-T"), 1);
    add_folder(&mut h.lib, "F-C3", "子3", Some("F-T"), 2);

    // F-X、F-Y 跨父级插到 F-C2 之前：锚点及后续兄弟顺移 +移动数，移动组从锚点 order 起连续分配
    let n = move_folders(&mut h.lib, &["F-X".into(), "F-Y".into()], Some("F-T"), Some("F-C2"), None).unwrap();
    assert_eq!(n, 2);
    assert_eq!(h.lib.index.folders["F-X"].order, 1, "从锚点 order 起");
    assert_eq!(h.lib.index.folders["F-Y"].order, 2, "按传入顺序连续分配");
    assert_eq!(h.lib.index.folders["F-C1"].order, 0, "锚点前的兄弟不动");
    assert_eq!(h.lib.index.folders["F-C2"].order, 3, "锚点及后续顺移 +移动数");
    assert_eq!(h.lib.index.folders["F-C3"].order, 4);
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F-X").unwrap().order, 1);
  }

  #[test]
  fn move_folders_anchor_must_be_direct_child_of_target() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);

    // 锚点不是 target_parent 直接子级：报错且不动数据
    let e = move_folders(&mut h.lib, &["F02".into()], None, Some("F01-1"), None).unwrap_err();
    assert!(e.contains("插入位置无效"), "{e}");
    assert_eq!(h.lib.index.folders["F02"].order, 1, "数据未被半截改动");
    // 锚点自身不在目标父级（F02 是顶层）/ 锚点不存在
    assert!(move_folders(&mut h.lib, &["F02".into()], Some("F01"), Some("F02"), None).is_err());
    assert!(move_folders(&mut h.lib, &["F02".into()], Some("F01"), Some("F-不存在"), None).is_err());
  }

  /// 2026-07-29 复验：嵌套文件夹拖出——跨父级落顶层某文件夹之前
  #[test]
  fn move_folders_pulls_nested_folder_out_to_top_before_anchor() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    let n = move_folders(&mut h.lib, &["F01-1".into()], None, Some("F02"), None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].parent, None, "拖回顶层");
    assert_eq!(top_orders(&h.lib), vec![("F01", 0), ("F01-1", 1), ("F02", 2)], "插到 F02 之前、F02 顺移");
  }

  #[test]
  fn move_folders_rejects_cycles() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_folder(&mut h.lib, "F01-1-1", "孙", Some("F01-1"), 0);

    // 移入自身 / 子孙：整条报错且不动数据
    let e = move_folders(&mut h.lib, &["F01".into()], Some("F01"), None, None).unwrap_err();
    assert!(e.contains("不能移动到自身或其子文件夹内"), "{e}");
    let e = move_folders(&mut h.lib, &["F01".into()], Some("F01-1-1"), None, None).unwrap_err();
    assert!(e.contains("不能移动到自身或其子文件夹内"), "{e}");
    // 批量中一条循环 → 整条失败，先验证后落点（F02 不被半截移动）
    assert!(move_folders(&mut h.lib, &["F02".into(), "F01".into()], Some("F01-1"), None, None).is_err());
    assert_eq!(h.lib.index.folders["F02"].parent, None);
    assert_eq!(h.lib.index.folders["F01"].parent, None);
  }

  #[test]
  fn move_folders_skips_same_parent_and_stale_ids() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    // 同父跳过不计数，数据不变
    let n = move_folders(&mut h.lib, &["F01-1".into()], Some("F01"), None, None).unwrap();
    assert_eq!(n, 0);
    assert_eq!((h.lib.index.folders["F01-1"].parent.clone(), h.lib.index.folders["F01-1"].order), (Some("F01".into()), 0));
    // 陈旧 ID 跳过；重复 ID 只计一次
    let n = move_folders(&mut h.lib, &["F-不存在".into(), "F01-1".into(), "F01-1".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].parent, None);
    // 目标父级不存在 → 报错
    assert!(move_folders(&mut h.lib, &["F01".into()], Some("F-不存在"), None, None).is_err());
  }

  /// 区块参数（2026-08-14 区块子系统）：顶级新建带 block_id 写入 Folder.block 并继承区块默认图标；
  /// 区块无默认图标则 icon 不写；block_id 指向不存在区块报错；非顶级新建忽略该参数
  #[test]
  fn create_folder_with_block_inherits_icon() {
    let mut h = make_lib();
    h.lib.folders.blocks.push(Block { id: "B01".into(), name: "角色".into(), icon: Some("icon-role".into()), order: 1 });
    h.lib.folders.blocks.push(Block { id: "B02".into(), name: "场景".into(), icon: None, order: 2 });
    add_folder(&mut h.lib, "F01", "甲", None, 0);

    let f = create_folder(&mut h.lib, None, "乙", None, Some("B01")).unwrap();
    assert_eq!(f.block.as_deref(), Some("B01"));
    assert_eq!(f.icon.as_deref(), Some("icon-role"), "继承区块默认图标");
    assert_eq!(h.lib.index.folders[&f.id].block.as_deref(), Some("B01"), "索引同步");
    // 区块无默认图标 → Folder.icon 保持 None
    let f2 = create_folder(&mut h.lib, None, "丙", None, Some("B02")).unwrap();
    assert_eq!((f2.block.as_deref(), f2.icon.clone()), (Some("B02"), None));
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|x| x.id == f.id).unwrap().icon.as_deref(), Some("icon-role"));
    // 不存在区块报错；非顶级忽略 block_id（即使 id 无效也不报）
    assert!(create_folder(&mut h.lib, None, "x", None, Some("B-不存在")).is_err());
    let child = create_folder(&mut h.lib, Some("F01"), "子", None, Some("B-不存在")).unwrap();
    assert_eq!((child.block.clone(), child.icon.clone()), (None, None), "非顶级忽略区块参数");
  }

  /// target_block 三态（2026-08-14 区块子系统）：落根级 Some(id) 写入 Folder.block；Some("dir") 移回
  /// 「目录」；None 保持原区块不动；落进文件夹内传 target_block 报错；指向不存在区块报错
  #[test]
  fn move_folders_target_block() {
    let mut h = make_lib();
    h.lib.folders.blocks.push(Block { id: "B01".into(), name: "角色".into(), icon: None, order: 1 });
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    // 落根级设置区块（子级拖出到区块根级）
    let n = move_folders(&mut h.lib, &["F01-1".into()], None, None, Some("B01")).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].block.as_deref(), Some("B01"), "索引同步");
    // None = 不动原区块
    move_folders(&mut h.lib, &["F01-1".into()], Some("F02"), None, None).unwrap();
    let n = move_folders(&mut h.lib, &["F01-1".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].block.as_deref(), Some("B01"), "不传 target_block 保持原区块");
    // Some("dir") = 移回「目录」
    move_folders(&mut h.lib, &["F01-1".into()], None, None, Some(DEFAULT_BLOCK_ID)).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].block.as_deref(), Some(DEFAULT_BLOCK_ID));
    // 落进文件夹内传 target_block → 报错；不存在区块 → 报错
    assert!(move_folders(&mut h.lib, &["F02".into()], Some("F01"), None, Some("B01")).is_err());
    assert!(move_folders(&mut h.lib, &["F02".into()], None, None, Some("B-不存在")).is_err());
  }

  /// 档案层级约束（2026-08-15 档案子系统，PRD 02_文件整理 03）：档案不作为父级——
  /// create_folder / move_folders 目标父级是 archive 报错；档案自身当移动 / 排序对象不受限
  #[test]
  fn archive_rejects_child_folders() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-A", "档案", None, 0);
    add_folder(&mut h.lib, "F-B", "普通", None, 1);
    // 置 F-A 为档案形态
    h.lib.folders.folders.iter_mut().find(|f| f.id == "F-A").unwrap().kind = Some(ARCHIVE_KIND.into());
    h.lib.index.folders.get_mut("F-A").unwrap().kind = Some(ARCHIVE_KIND.into());

    let e = create_folder(&mut h.lib, Some("F-A"), "子", None, None).unwrap_err();
    assert!(e.contains("档案下不允许出现子级文件夹"), "{e}");
    let e = move_folders(&mut h.lib, &["F-B".into()], Some("F-A"), None, None).unwrap_err();
    assert!(e.contains("档案下不允许出现子级文件夹"), "{e}");
    assert_eq!(h.lib.index.folders["F-B"].parent, None, "拒绝后数据不动");

    // 档案自身当移动对象不受限（移入普通文件夹 / 拖回顶层 / 同级排序）
    let n = move_folders(&mut h.lib, &["F-A".into()], Some("F-B"), None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F-A"].parent.as_deref(), Some("F-B"));
    let n = move_folders(&mut h.lib, &["F-A".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    reorder_folders(&mut h.lib, &["F-A".into()], None, Some("F-B")).unwrap();
    assert_eq!(top_orders(&h.lib), vec![("F-A", 0), ("F-B", 1)]);
  }

  /// 撤销接入验证：begin → record_file → 业务 → commit 后 undo 栈有一条，undo 回滚 folders.json
  #[test]
  fn undo_rolls_back_move_folders() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    library::save_folders(&h.lib).unwrap(); // 先落盘，撤销快照才含测试数据
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("移动 1 个文件夹");
    undo.record_file(&path).unwrap();
    move_folders(&mut h.lib, &["F02".into()], Some("F01"), None, None).unwrap();
    undo.commit().unwrap();
    assert!(undo.state().can_undo);
    assert_eq!(undo.state().undo_label.as_deref(), Some("移动 1 个文件夹"));

    undo.undo().unwrap();
    crate::core::library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.folders["F02"].parent, None, "undo 后回到顶层");
    assert_eq!(h.lib.index.folders["F02"].order, 1);
  }

  /// 带锚点定位的移动同样一步撤销（移动组与顺移的兄弟一并回滚）
  #[test]
  fn undo_rolls_back_move_folders_before_anchor() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    library::save_folders(&h.lib).unwrap();
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("移动 1 个文件夹");
    undo.record_file(&path).unwrap();
    move_folders(&mut h.lib, &["F01-1".into()], None, Some("F02"), None).unwrap();
    undo.commit().unwrap();

    undo.undo().unwrap();
    crate::core::library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].parent.as_deref(), Some("F01"), "undo 后回到原父级");
    assert_eq!(h.lib.index.folders["F01-1"].order, 0);
    assert_eq!(top_orders(&h.lib), vec![("F01", 0), ("F02", 1)], "顺移过的兄弟一并回滚");
  }

  /// 重命名（2026-07-29 复验三轮）：基本改名 + 落盘 + 索引同步；同名幂等
  #[test]
  fn rename_folder_basic_and_idempotent() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    rename_folder(&mut h.lib, "F01", "角色").unwrap();
    assert_eq!(h.lib.index.folders["F01"].name, "角色", "索引同步");
    assert_eq!(h.lib.index.folders["F01"].order, 0, "改名不动 order / 父级");
    assert_eq!(h.lib.index.folders["F01"].parent, None);
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F01").unwrap().name, "角色");
    // 子层级互不影响；前后空白 trim；同名幂等无操作
    assert_eq!(h.lib.index.folders["F01-1"].name, "子");
    rename_folder(&mut h.lib, "F01-1", "  站姿  ").unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].name, "站姿");
    rename_folder(&mut h.lib, "F01", "角色").unwrap();
    assert_eq!(h.lib.index.folders["F01"].name, "角色");
  }

  /// 非法名 / 空名 / 不存在目标拒绝（数据不动）
  #[test]
  fn rename_folder_rejects_invalid_names() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);

    // 注意："结尾 " trim 后为合法名（与 create_folder 同口径，先 trim 再校验）
    for bad in ["", "   ", "坏:名", "坏/名", "结尾.", "con"] {
      assert!(rename_folder(&mut h.lib, "F01", bad).is_err(), "应拒绝：{bad:?}");
    }
    assert!(rename_folder(&mut h.lib, "F-不存在", "新名").is_err());
    assert_eq!(h.lib.index.folders["F01"].name, "人物", "拒绝后数据不动");
  }

  /// 撤销接入验证（2026-07-29 复验三轮）：begin → record_file → 业务 → commit，undo 回滚 folders.json
  #[test]
  fn undo_rolls_back_rename_folder() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);
    library::save_folders(&h.lib).unwrap(); // 先落盘，撤销快照才含测试数据
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("重命名文件夹为 角色");
    undo.record_file(&path).unwrap();
    rename_folder(&mut h.lib, "F01", "角色").unwrap();
    undo.commit().unwrap();
    assert!(undo.state().can_undo);
    assert_eq!(undo.state().undo_label.as_deref(), Some("重命名文件夹为 角色"));

    undo.undo().unwrap();
    crate::core::library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.folders["F01"].name, "人物", "undo 后恢复原名");
  }

  /// 更改图标（2026-08-18，3.3，同步语义）：默认影响子级——后代图标与本节点当前图标一致的
  /// 跟随调整（None == None 首次统一靠此）；不一致的不动且不再下钻；档案后代恒跳过
  #[test]
  fn set_folder_icon_syncs_matching_descendants() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "顶", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_folder(&mut h.lib, "F01-1-1", "孙", Some("F01-1"), 0);
    add_folder(&mut h.lib, "F01-A", "档案子", Some("F01"), 1);
    h.lib.folders.folders.iter_mut().find(|f| f.id == "F01-A").unwrap().kind = Some(ARCHIVE_KIND.into());
    h.lib.index.folders.get_mut("F01-A").unwrap().kind = Some(ARCHIVE_KIND.into());

    // 首次统一（全默认 None == None）：整支跟随；档案跳过
    set_folder_icon(&mut h.lib, "F01", Some("animal")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].icon.as_deref(), Some("animal"), "子级跟随");
    assert_eq!(h.lib.index.folders["F01-1-1"].icon.as_deref(), Some("animal"), "多层后代一并跟随");
    assert_eq!(h.lib.index.folders["F01-A"].icon, None, "档案后代跳过");
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F01-1-1").unwrap().icon.as_deref(), Some("animal"));

    // 子级单独区分（F01-1 → scene；其旧值 animal 与孙一致 → 孙跟随 scene）
    set_folder_icon(&mut h.lib, "F01-1", Some("scene")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1-1"].icon.as_deref(), Some("scene"), "孙与子级旧链路一致，跟随");

    // 父级再改（F01 → weapon）：F01-1(scene != animal 旧链) 不动、不再下钻（孙保持 scene）
    set_folder_icon(&mut h.lib, "F01", Some("weapon")).unwrap();
    assert_eq!(h.lib.index.folders["F01"].icon.as_deref(), Some("weapon"));
    assert_eq!(h.lib.index.folders["F01-1"].icon.as_deref(), Some("scene"), "已区分子级不跟随");
    assert_eq!(h.lib.index.folders["F01-1-1"].icon.as_deref(), Some("scene"), "阻断分支不下钻");

    // 恢复默认（F01 → None）：F01-1 分支仍阻断（scene != weapon）
    set_folder_icon(&mut h.lib, "F01", None).unwrap();
    assert_eq!(h.lib.index.folders["F01"].icon, None);
    assert_eq!(h.lib.index.folders["F01-1"].icon.as_deref(), Some("scene"));
    // 不存在目标报错
    assert!(set_folder_icon(&mut h.lib, "F-不存在", Some("animal")).is_err());
  }

  /// 图标同步（2026-08-18 用户场景）：子级改回与父级一致后，父级再改子级重新跟随
  #[test]
  fn set_folder_icon_follows_after_realigned() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "顶", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    set_folder_icon(&mut h.lib, "F01", Some("scene")).unwrap(); // 全跟随 scene
    set_folder_icon(&mut h.lib, "F01-1", Some("animal")).unwrap(); // 子级单独区分
    set_folder_icon(&mut h.lib, "F01", Some("weapon")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].icon.as_deref(), Some("animal"), "已区分不跟随");
    // 子级改回与父级一致（weapon）→ 父级再改，子级重新跟随
    set_folder_icon(&mut h.lib, "F01-1", Some("weapon")).unwrap();
    set_folder_icon(&mut h.lib, "F01", Some("game")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].icon.as_deref(), Some("game"), "改回一致后重新跟随");
  }

  /// 更改颜色（2026-08-18，3.3，同步语义）：默认影响子级——显式与旧链路色一致的跟随调整；
  /// 无色后代不写数据（视觉自动跟随）但链路穿透；不一致的不动且不再下钻
  #[test]
  fn set_folder_color_syncs_matching_keeps_divergent() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "顶", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_folder(&mut h.lib, "F01-1-1", "孙", Some("F01-1"), 0);

    // P 设红：子级无色不写数据（视觉自动跟随，穿透）
    set_folder_color(&mut h.lib, "F01", Some("red")).unwrap();
    assert_eq!(h.lib.index.folders["F01"].color.as_deref(), Some("red"), "索引同步");
    assert_eq!(h.lib.index.folders["F01-1"].color, None, "无色子级不写数据");
    // 孙显式设红（与链路一致）→ P 改绿：孙跟随绿，子级仍无色
    set_folder_color(&mut h.lib, "F01-1-1", Some("red")).unwrap();
    set_folder_color(&mut h.lib, "F01", Some("green")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1-1"].color.as_deref(), Some("green"), "与旧链路一致的显式后代跟随");
    assert_eq!(h.lib.index.folders["F01-1-1"].color.as_deref(), Some("green"));
    assert_eq!(h.lib.index.folders["F01-1"].color, None, "无色穿透不写");

    // 子级单独区分（F01-1 → yellow-green）：其旧链路 = P 的绿 → 孙（绿）跟随变黄绿
    set_folder_color(&mut h.lib, "F01-1", Some("yellow-green")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1-1"].color.as_deref(), Some("yellow-green"), "孙与子级旧链路一致，跟随");

    // P 改蓝：F01-1(yellow-green != green 旧链) 不动、不下钻（孙保持黄绿）
    set_folder_color(&mut h.lib, "F01", Some("blue")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].color.as_deref(), Some("yellow-green"), "已区分子级不跟随");
    assert_eq!(h.lib.index.folders["F01-1-1"].color.as_deref(), Some("yellow-green"), "阻断分支不下钻");

    // 子级改回与父级一致（blue）→ P 改红：子级重新跟随（用户场景 2 修复）
    set_folder_color(&mut h.lib, "F01-1", Some("blue")).unwrap();
    set_folder_color(&mut h.lib, "F01", Some("red")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].color.as_deref(), Some("red"), "改回一致后重新跟随");

    // P 恢复默认：跟随支清 None
    set_folder_color(&mut h.lib, "F01", None).unwrap();
    assert_eq!(h.lib.index.folders["F01"].color, None);
    assert_eq!(h.lib.index.folders["F01-1"].color, None, "跟随的后代一并恢复默认");
    // 不存在目标报错
    assert!(set_folder_color(&mut h.lib, "F-不存在", Some("red")).is_err());
  }

  /// 颜色同步的旧链路上溯（2026-08-18）：本节点无色时旧链路色 = 祖先链最近色——
  /// 后代显式与祖先链路一致的同样跟随
  #[test]
  fn set_folder_color_old_chain_falls_back_to_ancestor() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "祖", None, 0);
    add_folder(&mut h.lib, "F01-1", "父", Some("F01"), 0);
    add_folder(&mut h.lib, "F01-1-1", "子", Some("F01-1"), 0);

    set_folder_color(&mut h.lib, "F01", Some("red")).unwrap(); // 祖设红，父无色视觉跟随
    set_folder_color(&mut h.lib, "F01-1-1", Some("red")).unwrap(); // 子显式红（与祖链路一致）
    // 父（无色，旧链路 = 祖的红）设绿：子显式红 == 旧链路 → 跟随绿
    set_folder_color(&mut h.lib, "F01-1", Some("green")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1-1"].color.as_deref(), Some("green"), "与祖先链路一致的后代跟随");
    assert_eq!(h.lib.index.folders["F01"].color.as_deref(), Some("red"), "祖不受影响");
  }

  /// 显式默认哨兵（2026-09-01 用户定）：子级选「默认」= 存 "default" 断开父级链路退回常态灰——
  /// 父级后续改色不再影响（写 None 会被父级色视觉盖回，永退不回）；改回与父级同色后恢复跟随
  ///（哨兵按普通链路值参与同步语义）
  #[test]
  fn set_folder_color_default_sentinel_breaks_chain() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "父", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_folder(&mut h.lib, "F01-1-1", "孙", Some("F01-1"), 0);

    set_folder_color(&mut h.lib, "F01", Some("red")).unwrap(); // 父红，子孙无色视觉跟随
    // 子选「默认」→ 显式哨兵断链
    set_folder_color(&mut h.lib, "F01-1", Some("default")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].color.as_deref(), Some("default"), "显式默认存哨兵");
    assert_eq!(h.lib.index.folders["F01-1-1"].color, None, "孙无色穿透不写（视觉跟随子的默认灰）");
    // 父改蓝：子哨兵 != 旧链路红 → 整支不动不下钻
    set_folder_color(&mut h.lib, "F01", Some("blue")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].color.as_deref(), Some("default"), "断链子级不跟随");
    assert_eq!(h.lib.index.folders["F01-1-1"].color, None, "阻断分支不下钻");
    // 子改回与父一致（blue）→ 父再改绿：子重新跟随（哨兵与九色同规则）
    set_folder_color(&mut h.lib, "F01-1", Some("blue")).unwrap();
    set_folder_color(&mut h.lib, "F01", Some("green")).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].color.as_deref(), Some("green"), "改回一致后重新跟随");
  }

  /// 批量更改颜色（2026-09-01，多选树 item 批量换色）：选中集逐 id 同语义（各自链路一致后代
  /// 跟随）；陈旧 id 跳过不计，全陈旧报错
  #[test]
  fn set_folder_colors_batch_applies_skips_stale() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F02-1", "乙子", Some("F02"), 0);

    let n = set_folder_colors(&mut h.lib, &["F01".to_string(), "F02".to_string(), "F-陈旧".to_string()], Some("green")).unwrap();
    assert_eq!(n, 2, "陈旧 id 跳过不计");
    assert_eq!(h.lib.index.folders["F01"].color.as_deref(), Some("green"));
    assert_eq!(h.lib.index.folders["F02"].color.as_deref(), Some("green"));
    assert_eq!(h.lib.index.folders["F02-1"].color, None, "无色后代不写数据（视觉跟随）");
    // 批量显式默认同样写哨兵
    set_folder_colors(&mut h.lib, &["F01".to_string(), "F02".to_string()], Some("default")).unwrap();
    assert_eq!(h.lib.index.folders["F01"].color.as_deref(), Some("default"));
    assert_eq!(h.lib.index.folders["F02"].color.as_deref(), Some("default"));
    // 全陈旧 → 报错
    assert!(set_folder_colors(&mut h.lib, &["F-不存在".to_string()], Some("red")).is_err());
  }

  // ---------- 分组（2026-08-19，3.5 二轮 / PRD 02-04）----------

  fn group(id: &str, name: &str, rules: &[(&str, &[&str])]) -> GroupDef {
    GroupDef {
      id: id.into(),
      name: name.into(),
      rules: rules
        .iter()
        .map(|(k, vs)| GroupRule { kind: k.to_string(), values: vs.iter().map(|v| v.to_string()).collect() })
        .collect(),
      members: None,
    }
  }

  /// 设置 + 读回 + 清空（folders.json 持久化；空数组存 None 不写字段）
  #[test]
  fn set_folder_groups_roundtrip_and_clear() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "角色", None, 0);
    let groups = vec![
      group("g1", "动画", &[("ext", &["fla", "gif"])]),
      group("g2", "图", &[("type", &["image"]), ("name", &["主角"])]),
    ];
    set_folder_groups(&mut h.lib, "F01", groups.clone()).unwrap();
    assert_eq!(h.lib.index.folders["F01"].groups.as_ref().unwrap().len(), 2);
    // 持久化读回
    let back: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    let f = back.folders.iter().find(|f| f.id == "F01").unwrap();
    assert_eq!(f.groups.as_ref().unwrap()[1].rules.len(), 2, "多规则组读回");
    // 名称不查重：同名不同规则允许
    set_folder_groups(&mut h.lib, "F01", vec![
      group("g1", "同名", &[("ext", &["fla"])]),
      group("g2", "同名", &[("ext", &["png"])]),
    ])
    .unwrap();
    // 清空 = None（不写字段）
    set_folder_groups(&mut h.lib, "F01", vec![]).unwrap();
    assert_eq!(h.lib.index.folders["F01"].groups, None);
    let back: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    let f = back.folders.iter().find(|f| f.id == "F01").unwrap();
    assert!(f.groups.is_none());
    // 目标不存在报错
    assert!(set_folder_groups(&mut h.lib, "F-X", vec![]).is_err());
  }

  /// 完全相同判定（23 稿）：规则集合相同（组序 / 规则序 / 值序无关）禁止保存；名称不参与
  #[test]
  fn set_folder_groups_rejects_identical_rules() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "角色", None, 0);
    // 完全相同（值序不同也算相同）
    let dup = set_folder_groups(&mut h.lib, "F01", vec![
      group("g1", "A", &[("ext", &["fla", "gif"])]),
      group("g2", "B", &[("ext", &["gif", "fla"])]),
    ]);
    assert!(dup.is_err(), "规则集合相同应报错");
    assert!(dup.unwrap_err().contains("完全相同"));
    // 规则数不同不算相同
    set_folder_groups(&mut h.lib, "F01", vec![
      group("g1", "A", &[("ext", &["fla"])]),
      group("g2", "B", &[("ext", &["fla"]), ("type", &["image"])]),
    ])
    .unwrap();
    // 空规则 = 自由组（2026-09-17 无规则空组放开）；空范围仍报错
    assert!(set_folder_groups(&mut h.lib, "F01", vec![group("g1", "A", &[])]).is_ok(), "空规则 = 自由组放开");
    assert!(set_folder_groups(&mut h.lib, "F01", vec![group("g1", "A", &[("ext", &[])])]).is_err());
  }

  /// 自由组（2026-09-17，PRD 02-04 无规则空组）：assign_group 移动语义唯一归属 / 移出 /
  /// 非自由组与不存在组拦截；自由组与规则组「完全相同」查重不混
  #[test]
  fn assign_group_moves_members_uniquely() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "角色", None, 0);
    let mut free_a = group("ga", "空组A", &[]);
    free_a.members = Some(vec![]);
    let mut free_b = group("gb", "空组B", &[]);
    free_b.members = Some(vec!["AN-1".into()]);
    set_folder_groups(&mut h.lib, "F01", vec![free_a, free_b, group("g1", "规则组", &[("ext", &["fla"])])]).unwrap();
    // 自由组与规则组查重不混（另一文件夹：两个空规则自由组不算「完全相同」）
    add_folder(&mut h.lib, "F02", "特效", None, 1);
    set_folder_groups(&mut h.lib, "F02", vec![group("ga", "空A", &[]), group("gb", "空B", &[])]).unwrap();

    let members_of = |lib: &Library, idx: usize| {
      lib.index.folders["F01"].groups.as_ref().unwrap()[idx]
        .members
        .clone()
        .unwrap_or_default()
    };
    // 移入 A：唯一归属
    assign_group(&mut h.lib, "F01", Some("ga"), &["AN-1".into(), "AN-2".into()]).unwrap();
    assert_eq!(members_of(&h.lib, 0), vec!["AN-1", "AN-2"], "移入目标组");
    assert_eq!(members_of(&h.lib, 1), Vec::<String>::new(), "从其他自由组摘除（移动语义）");
    // 再移 B：只动 AN-1（A 留 AN-2）、B 收留（幂等去重）
    assign_group(&mut h.lib, "F01", Some("gb"), &["AN-1".into()]).unwrap();
    assert_eq!(members_of(&h.lib, 0), vec!["AN-2"], "未涉及的成员不动");
    assert_eq!(members_of(&h.lib, 1), vec!["AN-1"], "幂等去重");
    // 移出（None）：全部摘除
    assign_group(&mut h.lib, "F01", None, &["AN-1".into()]).unwrap();
    assert_eq!(members_of(&h.lib, 1), Vec::<String>::new());
    // 非自由组 / 不存在组拦截
    assert!(assign_group(&mut h.lib, "F01", Some("g1"), &["AN-1".into()]).is_err());
    assert!(assign_group(&mut h.lib, "F01", Some("g-x"), &["AN-1".into()]).is_err());
    assert!(assign_group(&mut h.lib, "F-X", None, &["AN-1".into()]).is_err());
  }
}
