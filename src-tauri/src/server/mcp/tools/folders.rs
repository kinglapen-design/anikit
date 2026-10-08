//! 工具：文件夹树与资产清单（commands/import.rs folder_tree / placement_views 同函数，薄壳转发）

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, schemars, tool, tool_router};

use crate::server::mcp::AnikitMcp;

/// list_placements 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPlacementsParams {
  /// 文件夹 id（取自 list_folder_tree 节点 id；"root" = 未分类；anikit://folder/{id} 链接形态同吃）；缺省 = 全部 Placement
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub folder_id: Option<String>,
}

#[tool_router(router = folders_router, vis = "pub")]
impl AnikitMcp {
  /// 文件夹树（区块表 blocks + 树 nodes：id / name / count / totalSize / children…）。
  /// 浏览素材库结构的第一步——文件夹 id 供 search_assets / list_placements 限定范围
  #[tool(name = "list_folder_tree", annotations(read_only_hint = true))]
  fn list_folder_tree(&self) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| Ok(crate::services::folder_tree::folder_tree(lib)))
  }

  /// 文件夹内资产清单（PlacementView 数组：assetId / folderId / 显示名 / 宽高 / 扩展名 /
  /// thumbPath 缩略图绝对路径（看图直读该 webp 文件）/ 大小 / 时长 / 标签等）。
  /// 不传 folderId 返回全库清单，量大时注意体积
  #[tool(name = "list_placements", annotations(read_only_hint = true))]
  fn list_placements(&self, Parameters(p): Parameters<ListPlacementsParams>) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| Ok(crate::commands::import::placement_views(lib, p.folder_id.as_deref())))
  }
}
