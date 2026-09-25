//! 文件夹树视图组装（2026-09-25 T1 自 commands/import.rs 下迁——server/plugin_ws 与 server/mcp
//! 两处消费，server 层反向依赖 commands 层就此根治）：parent=None 为顶层递归子级；
//! count = 递归 Placement 数（hidden 伴生不计，PRD 02-15）；totalSize = 子树资产去重后
//! size 合计（跨层级同资产只计一次，2026-08-30 用户定案）；blocks 按 order 排序一并输出。
//! 消费方：list_folder_tree 命令（导入弹窗）/ plugin_ws tree（插件目标选择器）/ MCP list_folder_tree。

use serde::Serialize;

use crate::core::library::Library;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
  pub id: String,
  pub name: String,
  /// 该文件夹递归 Placement 总数（含全部子文件夹；2026-07-25 用户定：整洁层级下直接数会大面积为 0）
  pub count: u32,
  /// 递归容量（字节，2026-08-30 用户定案，与 count 同口径同趟算出）：子树全部 Placement 按 asset
  /// 去重后 size 合计——同一资产在父子文件夹各放一份只计一次（占用空间语义，同 archive.rs 档案容量）
  pub total_size: u64,
  /// 所属区块 ID（2026-08-14 区块子系统：仅顶级节点有值语义，None = 「目录」；子节点恒 None 不写字段）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub block: Option<String>,
  /// 自定义图标（2026-08-14 区块子系统起顶级继承区块默认图标；2026-08-18 3.3 起全层级可持，None 不写字段）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
  /// 节点形态（2026-08-15 档案子系统一轮：Some("archive") = 档案；所有层级都带；None 不写字段）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub kind: Option<String>,
  /// 自定义颜色（2026-08-18，3.3）：色档名，所有层级都带；子级视觉跟随由前端底色档承担，None 不写字段
  #[serde(skip_serializing_if = "Option::is_none")]
  pub color: Option<String>,
  /// 分组规则（2026-08-19，3.5 二轮）：所有层级都带（档案与普通文件夹通用，PRD 02-04）；
  /// 空 / None 不写字段——分组计算在前端（显示层）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub groups: Option<Vec<crate::core::models::GroupDef>>,
  pub children: Vec<FolderNode>,
}

/// 区块视图（侧栏分组依据），按 Block.order 排序输出
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockView {
  pub id: String,
  pub name: String,
  pub icon: Option<String>,
}

/// 文件夹树返回（2026-08-14 区块子系统）：blocks = 区块表（order 序）；nodes = 文件夹树
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderTreeView {
  pub blocks: Vec<BlockView>,
  pub nodes: Vec<FolderNode>,
}

/// 组装文件夹树：parent=None 为顶层，递归子级；区块表按 order 排序一并输出
pub fn folder_tree(lib: &Library) -> FolderTreeView {
  /// 返回 (节点, 子树资产 id 集)：集合供父级并集去重算 totalSize（跨子树同资产只计一次）
  fn children_of<'a>(lib: &'a Library, parent: Option<&'a str>) -> Vec<(FolderNode, std::collections::HashSet<&'a str>)> {
    let mut folders: Vec<&crate::core::models::Folder> =
      lib.folders.folders.iter().filter(|f| f.parent.as_deref() == parent).collect();
    folders.sort_by_key(|f| f.order);
    folders
      .into_iter()
      .map(|f| {
        let pairs = children_of(lib, Some(&f.id));
        let (children, child_sets): (Vec<FolderNode>, Vec<std::collections::HashSet<&str>>) = pairs.into_iter().unzip();
        let direct_all = lib.index.placements_by_folder.get(&f.id).map(|v| v.as_slice()).unwrap_or(&[]);
        // hidden 伴生缩略图资产不计数（8.1，PRD 02-15：视图统一过滤，计数/容量同口径——
        // 否则"文件数 2 只见 1 卡"割裂，2026-09-01 用户实机报）
        let direct_ps: Vec<_> = direct_all
          .iter()
          .filter(|p| lib.index.assets.get(p.asset.as_str()).map(|m| m.hidden != Some(true)).unwrap_or(false))
          .collect();
        let direct = direct_ps.len() as u32;
        let recursive = direct + children.iter().map(|c| c.count).sum::<u32>();
        // 子树资产集 = 本级 placements ∪ 各子树集；totalSize = 集内资产 size 合计（缺失索引的脏引用跳过）
        let mut subtree: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for set in child_sets {
          subtree.extend(set);
        }
        subtree.extend(direct_ps.iter().map(|p| p.asset.as_str()));
        let total_size = subtree.iter().filter_map(|id| lib.index.assets.get(*id)).map(|m| m.size).sum::<u64>();
        // block 仅顶级节点输出（子节点不持区块语义）；icon / kind / color 全层级如实输出
        //（icon 2026-08-14 区块默认图标继承 → 3.3 起子级也可持自定义图标）
        let block = if parent.is_none() { f.block.clone() } else { None };
        let node = FolderNode {
          id: f.id.clone(),
          name: f.name.clone(),
          count: recursive,
          total_size,
          block,
          icon: f.icon.clone(),
          kind: f.kind.clone(),
          color: f.color.clone(),
          groups: f.groups.clone().filter(|g| !g.is_empty()), // 空组不输出（写回统一 None 口径）
          children,
        };
        (node, subtree)
      })
      .collect()
  }
  let mut blocks: Vec<&crate::core::models::Block> = lib.folders.blocks.iter().collect();
  blocks.sort_by_key(|b| b.order);
  FolderTreeView {
    blocks: blocks.into_iter().map(|b| BlockView { id: b.id.clone(), name: b.name.clone(), icon: b.icon.clone() }).collect(),
    nodes: children_of(lib, None).into_iter().map(|(n, _)| n).collect(),
  }
}
