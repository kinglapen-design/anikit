//! 工具：资产详情与磁盘路径（commands/archive.rs asset_detail / services::asset_menu::real_origin_file
//! / services::views::thumb_path 同函数）。originName 随重命名同步改磁盘名——外部不得缓存路径、每次现推

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, schemars, tool, tool_router};

use crate::server::mcp::AnikitMcp;
use crate::services::{asset_menu, views};

/// 资产 id 入参（search_assets / list_placements 回包里的 assetId）
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetParams {
  /// 资产 id（如 "AN-20260901-XXXXXX"；anikit://item/{id} 与 http://127.0.0.1:{port}/(open/)item/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub asset_id: String,
}

/// get_asset_origin_path 回包：源文件与缩略图磁盘绝对路径（看图直读，不经协议传图）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct OriginPaths {
  /// 源文件真实绝对路径（序列帧 = frames/ 封面帧，与右键「复制文件路径」同口径）
  origin_path: String,
  /// 缩略图绝对路径（webp；含 thumbAssetId 伴生代理——fla 等工程资产取伴生 hidden 资产的图）；
  /// 无缩略图且无格式图标缓存 = null
  thumb_path: Option<String>,
}

#[tool_router(router = assets_router, vis = "pub")]
impl AnikitMcp {
  /// 资产详情：基础信息（名 / 扩展名 / 尺寸 / 时长 / 大小 / 时间戳）+ 属性词 tags +
  /// 描述 descriptions + 注释 note + 链接 links + 主色 colors + Placement 反查 placements +
  /// 归属档案 archives + uri（http open 形态点击跳转链接，2026-10-08 S5）。
  /// 纯读（不做 APP 侧的懒回填主色——存量资产 colors 可能为空）
  #[tool(name = "get_asset_detail", annotations(read_only_hint = true))]
  fn get_asset_detail(&self, Parameters(p): Parameters<AssetParams>) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| {
      let detail = crate::commands::archive::asset_detail(lib, &p.asset_id)?;
      // 顶层附 uri 字段（新增不破坏既有 Agent 解析）：AI 回复用户时可点击回 AniKit
      let mut v = serde_json::to_value(&detail).map_err(|e| e.to_string())?;
      let port = crate::server::mcp::status().1.unwrap_or(17701);
      v["uri"] = serde_json::Value::String(crate::core::anikit_ref::open_uri(port, "item", &p.asset_id));
      Ok(v)
    })
  }

  /// 资产源文件与缩略图的磁盘绝对路径。Agent 读图 = 直读 thumbPath（webp 缩略图）或
  /// originPath（源图）；originName 随重命名改磁盘名，路径每次现推、不要缓存复用
  #[tool(name = "get_asset_origin_path", annotations(read_only_hint = true))]
  fn get_asset_origin_path(&self, Parameters(p): Parameters<AssetParams>) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| {
      let meta = lib.index.assets.get(&p.asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?;
      let origin = asset_menu::real_origin_file(lib, &p.asset_id)?;
      // 缩略图走 thumbAssetId 伴生代理（同 services::views::placement_view 口径，不自己拼词干）
      let thumb_source =
        meta.thumb_asset_id.as_deref().and_then(|id| lib.index.assets.get(id)).filter(|t| t.hidden == Some(true)).unwrap_or(meta);
      Ok(OriginPaths {
        origin_path: origin.to_string_lossy().into_owned(),
        thumb_path: views::thumb_path(lib, thumb_source, &lib.format_icons),
      })
    })
  }
}
