//! 工具：分词搜索（services::search::search，commands/search.rs 同服务；面向 Agent 简化范围参数：
//! 不传 folderId = 全库搜（含档案命中，回收站除外），传 folderId = 限定该文件夹，recursive 缺省 true）

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, schemars, tool, tool_router};

use crate::server::mcp::AnikitMcp;
use crate::services::search::{self, SearchScope};

/// search_assets 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
  /// 搜索关键字：空格分词 AND、不区分大小写；命中资产名 / Placement 显示名 / 扩展名 / 链接 / 路径链 / 档案名等
  pub query: String,
  /// 限定文件夹 id（取自 list_folder_tree 的节点 id；anikit://folder/{id} 链接形态同吃）；缺省 = 全库搜索
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub folder_id: Option<String>,
  /// 限定文件夹时是否含全部子文件夹；缺省 true（未限定文件夹时无效）
  pub recursive: Option<bool>,
}

#[tool_router(router = search_router, vis = "pub")]
impl AnikitMcp {
  /// 分词搜索素材库。返回命中 Placement 键集（assetId+folderId + uri 点击跳转链接，2026-10-08 S5；
  /// 可交给 get_asset_detail / get_asset_origin_path 逐个取详情与磁盘路径）+ 命中档案 id 集（可交给 list_archives 对照）
  #[tool(name = "search_assets", annotations(read_only_hint = true))]
  fn search_assets(&self, Parameters(p): Parameters<SearchParams>) -> Result<CallToolResult, ErrorData> {
    let scope = SearchScope {
      global: p.folder_id.is_none(),
      view_key: p.folder_id.map(|id| format!("folder:{id}")).unwrap_or_else(|| "quick:all".to_string()),
      recursive: p.recursive.unwrap_or(true),
      filters: crate::services::search::SearchFilters::default(), // MCP 搜索不暴露筛选（2026-10-09）
    };
    self.with_lib(|lib| {
      let hits = search::search(lib, &p.query, &scope);
      // placements 每项附 uri 字段（http open 形态，AI 回复可点击回 AniKit；新增字段不破坏既有解析）
      let mut v = serde_json::to_value(&hits).map_err(|e| e.to_string())?;
      let port = crate::server::mcp::status().1.unwrap_or(17701);
      if let Some(arr) = v.get_mut("placements").and_then(|x| x.as_array_mut()) {
        for hit in arr {
          if let Some(id) = hit.get("assetId").and_then(|x| x.as_str()).map(|s| s.to_string()) {
            hit["uri"] = serde_json::Value::String(crate::core::anikit_ref::open_uri(port, "item", &id));
          }
        }
      }
      Ok(v)
    })
  }
}
