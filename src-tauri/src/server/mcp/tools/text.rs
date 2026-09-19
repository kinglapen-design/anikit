//! 工具：文本资产内容（commands/text.rs read_text_asset 同口径：UTF-8 优先 GBK 回退，2MB 上限）

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};

use crate::server::mcp::AnikitMcp;
use super::assets::AssetParams;
use crate::commands::preview::locate_asset;
use crate::services::text;

#[tool_router(router = text_router, vis = "pub")]
impl AnikitMcp {
  /// 读取文本资产内容（txt / md；提示词、剧本等）。非文本资产报错；超过 2MB 报错
  #[tool(name = "read_text_asset", annotations(read_only_hint = true))]
  fn read_text_asset(&self, Parameters(p): Parameters<AssetParams>) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| {
      let (_info_dir, origin, meta) = locate_asset(lib, &p.asset_id)?;
      if !text::is_text(&origin) {
        return Err(format!("「{}」不是文本资源", meta.name));
      }
      text::read_text(&origin).map(|content| serde_json::json!({ "content": content }))
    })
  }
}
