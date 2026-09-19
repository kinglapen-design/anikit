//! 工具：档案列表（commands/archive.rs archive_list_items 同函数，薄壳转发）

use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};

use crate::server::mcp::AnikitMcp;

#[tool_router(router = archives_router, vis = "pub")]
impl AnikitMcp {
  /// 档案列表（folderId / 徽标 badge / 头像 avatar / 属性 props / 描述 descriptions / 链接 links /
  /// 容量 totalSize / 封面图绝对路径 coverPath 等）。folderId 可交给 list_placements 列档案内容
  #[tool(name = "list_archives", annotations(read_only_hint = true))]
  fn list_archives(&self) -> Result<CallToolResult, ErrorData> {
    self.with_lib(|lib| Ok(crate::commands::archive::archive_list_items(lib)))
  }
}
