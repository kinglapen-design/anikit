//! 新建文档软件检测（1.3）：前端每次打开 UI 调用，结果驱动 07 新增菜单"新建文档"组显隐与图标。

use crate::services::doc_apps::{detect_doc_apps as detect, DocAppInfo};

/// 检测各类文档对应软件是否安装（Windows 文件关联）并提取主程序图标
#[tauri::command]
pub async fn detect_doc_apps() -> Result<Vec<DocAppInfo>, String> {
  Ok(detect())
}
