//! 工具：档案查询与管理（APP commands/archive.rs / archive_ops.rs 同一业务路径）

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use crate::commands::library::AppState;
use crate::core::models::DescItem;
use crate::server::mcp::{AnikitMcp, json_result, write_gate};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveParams {
  /// 档案 folderId（list_archives / list_folder_tree 回包；anikit://folder/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub folder_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateArchiveParams {
  /// 档案名（空名 / Windows 非法字符后端拒绝）
  pub name: String,
  /// 父文件夹 id；缺省 / "root" = 顶层（档案内不能再建子级）
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub parent_id: Option<String>,
  /// 可选同级插入锚点；缺省 = 末尾
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub after_folder_id: Option<String>,
  /// 可选顶层区块 id（list_folder_tree blocks 回包）
  #[serde(default)]
  pub block_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConvertArchiveParams {
  /// 待转换文件夹 id（anikit://folder/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub folder_id: String,
  /// 有子文件夹时是否确认剔除层级；缺省 false = 只返回 need_confirm、不动数据
  #[serde(default)]
  pub confirmed: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetArchivePropsParams {
  /// 档案 folderId（anikit://folder/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub folder_id: String,
  /// 属性词终态数组（整体替换；逐项 trim、去空、保序去重；空数组 = 清空）
  pub props: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DescriptionParams {
  pub title: String,
  pub content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetArchiveDescriptionsParams {
  /// 档案 folderId（anikit://folder/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub folder_id: String,
  /// 描述终态数组（整体替换；标题 / 内容 trim，双空项丢弃；空数组 = 清空）
  pub descriptions: Vec<DescriptionParams>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetArchiveLinksParams {
  /// 档案 folderId（anikit://folder/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub folder_id: String,
  /// 链接终态数组（整体替换；逐条 trim、去空、保序去重；空数组 = 清空）
  pub links: Vec<String>,
}

impl AnikitMcp {
  fn archive_write<T: serde::Serialize>(&self, what: &str, result: Result<T, String>) -> Result<CallToolResult, ErrorData> {
    match result {
      Ok(value) => {
        self.emit_library_changed(what);
        json_result(&value)
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}

#[tool_router(router = archives_router, vis = "pub")]
impl AnikitMcp {
  /// 档案列表（folderId / 徽标 badge / 头像 avatar / 属性 props / 描述 descriptions / 链接 links /
  /// 容量 totalSize / 封面图绝对路径 coverPath 等）。folderId 可交给 list_placements 列档案内容
  #[tool(name = "list_archives", annotations(read_only_hint = true))]
  fn list_archives(&self) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| Ok(crate::commands::archive::archive_list_items(lib)))
  }

  /// 单个档案详情：ArchiveData 全字段 + fileCount / totalSize / coverPath / coverVer / prompt。纯读
  #[tool(name = "get_archive_data", annotations(read_only_hint = true))]
  fn get_archive_data(&self, Parameters(p): Parameters<ArchiveParams>) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| Ok(crate::commands::archive::archive_detail(lib, &p.folder_id)))
  }

  /// 新建档案，返回 Folder（含新 folderId）。parentId 缺省 / "root" = 顶层。进撤销
  #[tool(name = "create_archive", annotations(read_only_hint = false))]
  fn create_archive(&self, Parameters(p): Parameters<CreateArchiveParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    let result = crate::commands::archive_ops::create_archive_op(
      &state,
      p.parent_id.as_deref(),
      &p.name,
      p.after_folder_id.as_deref(),
      p.block_id.as_deref(),
    );
    self.archive_write("create_archive", result)
  }

  /// 文件夹转换为档案。有子级且 confirmed=false 时返回 need_confirm 且不动数据；
  /// 确认后剔除层级、Placement 平移到档案。返回 done / need_confirm。进撤销
  #[tool(name = "convert_to_archive", annotations(read_only_hint = false))]
  fn convert_to_archive(&self, Parameters(p): Parameters<ConvertArchiveParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    let result = crate::commands::archive_ops::convert_to_archive_op(&state, &p.folder_id, p.confirmed);
    self.archive_write("convert_to_archive", result)
  }

  /// 档案解散为普通文件夹：摘档案形态并清档案数据记录，Placement 不动。进撤销
  #[tool(name = "convert_to_folder", annotations(read_only_hint = false))]
  fn convert_to_folder(&self, Parameters(p): Parameters<ArchiveParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    let result = crate::commands::archive_ops::convert_to_folder_op(&state, &p.folder_id).map(|_| serde_json::json!({}));
    self.archive_write("convert_to_folder", result)
  }

  /// 整体设置档案属性词（trim / 去空 / 保序去重；空数组 = 清空）。进撤销
  #[tool(name = "set_archive_props", annotations(read_only_hint = false))]
  fn set_archive_props(&self, Parameters(p): Parameters<SetArchivePropsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    let result = crate::commands::archive_ops::set_archive_props_op(&state, &p.folder_id, &p.props).map(|_| serde_json::json!({}));
    self.archive_write("set_archive_props", result)
  }

  /// 整体设置档案描述（标题 + 内容数组；双空项丢弃；空数组 = 清空）。进撤销
  #[tool(name = "set_archive_descriptions", annotations(read_only_hint = false))]
  fn set_archive_descriptions(&self, Parameters(p): Parameters<SetArchiveDescriptionsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let descriptions: Vec<DescItem> = p.descriptions.into_iter().map(|d| DescItem { title: d.title, content: d.content }).collect();
    let state = self.app.state::<AppState>();
    let result = crate::commands::archive_ops::set_archive_descriptions_op(&state, &p.folder_id, &descriptions).map(|_| serde_json::json!({}));
    self.archive_write("set_archive_descriptions", result)
  }

  /// 整体设置档案链接（trim / 去空 / 保序去重；空数组 = 清空）。进撤销
  #[tool(name = "set_archive_links", annotations(read_only_hint = false))]
  fn set_archive_links(&self, Parameters(p): Parameters<SetArchiveLinksParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    let result = crate::commands::archive_ops::set_archive_links_op(&state, &p.folder_id, &p.links).map(|_| serde_json::json!({}));
    self.archive_write("set_archive_links", result)
  }

  /// 删除档案节点并清档案数据记录；档案内全部 Placement 移入未分类，素材实体不动。
  /// 返回 DeleteFolderReport。进撤销；物理删除永不经 MCP
  #[tool(name = "delete_archive", annotations(read_only_hint = false))]
  fn delete_archive(&self, Parameters(p): Parameters<ArchiveParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    let result = crate::commands::archive_ops::delete_archive_op(&state, &p.folder_id);
    self.archive_write("delete_archive", result)
  }
}
