//! 工具：全库链接 Placement 清单（2026-10-12 用户定，PRD 08-06 ⑨ 只读补批）——「资产留大池、
//! 链接进场次」组织习惯的可见性配套：AI 经 link_placements 建链接后回查哪些文件是链接、链在哪、
//! 原件在哪。无参全库列举（链接体量小，不破省 token 口径；替代全量 list_placements 自行过滤
//! origin 的重路径）。link / origin 复用详情链接区块 PlacementLink 结构（commands::archive::placement_link）

use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};

use crate::commands::archive::{PlacementLink, placement_link};
use crate::server::mcp::AnikitMcp;

/// list_links 回包单项：链接位置 + 原始位置 + uri 点击跳转链接
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LinkView {
  asset_id: String,
  ext: String,
  /// 链接 Placement 位置（folderId 为 null = 未分类 root；blockName + path = 区块与路径链，最多三段）
  link: PlacementLink,
  /// 原始 Placement 位置（同 link 结构）；原始已进回收站 / 彻底删除 = null
  origin: Option<PlacementLink>,
  /// http open 点击跳转链接（2026-10-08 S5 同口径，AI 回复用户可点击回 AniKit 定位）
  uri: String,
}

#[tool_router(router = links_router, vis = "pub")]
impl AnikitMcp {
  /// 全库链接 Placement 清单（资产的原始 Placement 之外的放置记录均为链接）。逐条回 assetId / ext /
  /// 链接位置 / 原始位置 / uri。链接通常不多，无参全列即可——不要用全量 list_placements 自行过滤 origin
  #[tool(name = "list_links", annotations(read_only_hint = true))]
  fn list_links(&self) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| {
      let port = crate::server::mcp::status().1.unwrap_or(17701);
      let mut out = Vec::new();
      for p in &lib.folders.items {
        if p.origin {
          continue;
        }
        // 资源在索引中缺失的脏 Placement 跳过（同 placement_views 口径）；folder 链断裂同跳
        let Some(meta) = lib.index.assets.get(&p.asset) else { continue };
        let Some(link) = placement_link(lib, p) else { continue };
        let origin = lib
          .index
          .placements_of(&p.asset)
          .iter()
          .find(|q| q.origin)
          .and_then(|q| placement_link(lib, q));
        out.push(LinkView {
          asset_id: p.asset.clone(),
          ext: meta.ext.clone(),
          link,
          origin,
          uri: crate::core::anikit_ref::open_uri(port, "item", &p.asset),
        });
      }
      Ok(out)
    })
  }
}
