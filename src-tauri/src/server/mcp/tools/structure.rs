//! 工具：二批结构性写工具四个（PRD 08-06 二批写工具，2026-09-19 用户定）——create_folder /
//! move_placements / link_placements / move_folders（同日用户补：结构重组留一手）。全部 annotations(read_only_hint = false) + write_gate 门禁；
//! 撤销口径：三个都进撤销（复用 commands::operations 命令整段抽出的 _op 函数，
//! 锁 library → 锁 undo → 单写点 + 单撤销步，与 APP 命令同一实现）。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use crate::commands::library::AppState;
use crate::server::mcp::{AnikitMcp, json_result, write_gate};
use crate::services::operations::PlacementRef;

/// create_folder 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderParams {
  /// 文件夹名（空名 / Windows 非法字符后端拒绝）
  pub name: String,
  /// 父文件夹 id（取自 list_folder_tree 节点 id）；缺省 / "root" = 顶层
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub parent_id: Option<String>,
}

/// move_placements / link_placements 单项（Placement 定位三元组，取自 list_placements 回包）
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlacementRefParams {
  /// 资产 id（anikit:// 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub asset_id: String,
  /// 所在文件夹 id（Placement 定位二元组之一）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub folder_id: String,
  /// 是否原始 Placement（**定位三元组之一**，照回包 origin 字段抄；缺省 false = 只命中链接件——
  /// 对 origin 记录漏传本字段会定位落空被跳过）
  #[serde(default)]
  pub origin: bool,
}

/// move_placements 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovePlacementsParams {
  /// 待移动 Placement 清单（单条 = 1 项数组）
  pub items: Vec<PlacementRefParams>,
  /// 目标文件夹 id（取自 list_folder_tree 节点 id；"root" = 未分类）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub target_folder_id: String,
}

/// move_folders 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoveFoldersParams {
  /// 待移动文件夹 id 清单（取自 list_folder_tree 节点 id；单条 = 1 项数组）
  #[serde(deserialize_with = "super::de_ref_id_vec")]
  pub folder_ids: Vec<String>,
  /// 目标父文件夹 id；缺省 / "root" = 顶层（文件夹区块）
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub target_parent_id: Option<String>,
}

/// link_placements 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkPlacementsParams {
  /// 待链接 Placement 清单（单条 = 1 项数组）
  pub items: Vec<PlacementRefParams>,
  /// 目标文件夹 id（链接 Placement 建在该文件夹，资产本体留原地）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub target_folder_id: String,
  /// 自定义显示名清单（与 items 等长对齐；null 项 = 用源 Placement 名；整参缺省 = 全部用源名）
  pub names: Option<Vec<Option<String>>>,
}

fn to_refs(items: Vec<PlacementRefParams>) -> Vec<PlacementRef> {
  items
    .into_iter()
    .map(|it| PlacementRef { asset_id: it.asset_id, folder_id: it.folder_id, origin: it.origin })
    .collect()
}

#[tool_router(router = structure_router, vis = "pub")]
impl AnikitMcp {
  /// 新建文件夹，返回 Folder（含新 folderId）。parentId 缺省 / "root" = 顶层。进撤销
  #[tool(name = "create_folder", annotations(read_only_hint = false))]
  fn create_folder(&self, Parameters(p): Parameters<CreateFolderParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    match crate::commands::operations::create_folder_op(&state, p.parent_id.as_deref(), &p.name, None, None) {
      Ok(folder) => {
        self.emit_library_changed("create_folder");
        json_result(&folder)
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }

  /// 把资产移入目标文件夹（Placement 级移动，去重 / 原地 / 陈旧项自动跳过不计），
  /// 返回 { moved 实际移动条数 }。进撤销
  #[tool(name = "move_placements", annotations(read_only_hint = false))]
  fn move_placements(&self, Parameters(p): Parameters<MovePlacementsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    if p.items.is_empty() {
      return Ok(CallToolResult::error(vec![ContentBlock::text("items 至少 1 项")]));
    }
    let items = to_refs(p.items);
    let state = self.app.state::<AppState>();
    match crate::commands::operations::move_placements_op(&state, &items, &p.target_folder_id) {
      Ok(moved) => {
        self.emit_library_changed("move_placements");
        json_result(&serde_json::json!({ "moved": moved }))
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }

  /// 移动文件夹重设父级（仅改数据不动磁盘；移入自身或子孙整条报错，同父 / 陈旧 id 跳过不计），
  /// 返回 { moved 实际移动数 }；目标 = 同级末尾，不给排序插入位。进撤销
  #[tool(name = "move_folders", annotations(read_only_hint = false))]
  fn move_folders(&self, Parameters(p): Parameters<MoveFoldersParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    if p.folder_ids.is_empty() {
      return Ok(CallToolResult::error(vec![ContentBlock::text("folderIds 至少 1 项")]));
    }
    // targetParentId 给了 = 落进该文件夹（targetBlock 必须为 None）；缺省 / "root" = 顶层，写 block="dir"
    //（命令层口径：落根级给值才写 Folder.block，移回「文件夹」区块传 "dir"，id 恒 dir）
    let (target_parent, target_block) = match p.target_parent_id.as_deref() {
      Some(id) if id != "root" => (Some(id), None),
      _ => (None, Some("dir")),
    };
    let state = self.app.state::<AppState>();
    match crate::commands::operations::move_folders_op(&state, &p.folder_ids, target_parent, None, target_block) {
      Ok(moved) => {
        self.emit_library_changed("move_folders");
        json_result(&serde_json::json!({ "moved": moved }))
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }

  /// 链接汇总：目标文件夹逐条建链接 Placement（资产本体留原地、不占双份磁盘），
  /// 返回 { linked 实际关联条数 }。进撤销
  #[tool(name = "link_placements", annotations(read_only_hint = false))]
  fn link_placements(&self, Parameters(p): Parameters<LinkPlacementsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    if p.items.is_empty() {
      return Ok(CallToolResult::error(vec![ContentBlock::text("items 至少 1 项")]));
    }
    let item_count = p.items.len();
    let names: Vec<Option<String>> = match p.names {
      Some(n) => {
        if n.len() != item_count {
          return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
            "names 与 items 须等长（{} ≠ {item_count}）",
            n.len()
          ))]));
        }
        n
      }
      None => vec![None; item_count],
    };
    let items = to_refs(p.items);
    let state = self.app.state::<AppState>();
    match crate::commands::operations::link_placements_op(&state, &items, &p.target_folder_id, &names) {
      Ok(linked) => {
        self.emit_library_changed("link_placements");
        json_result(&serde_json::json!({ "linked": linked }))
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}
