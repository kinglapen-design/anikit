//! anikit 引用解析（2026-10-08 S5，PRD 02-09 双形态分工）：`anikit://item|folder/{id}`（内部标识符）
//! 与 `http(s)://127.0.0.1:{port}/(open/)item|folder/{id}`（对外可点击形态）归一化为纯 id——
//! MCP 工具参数直接吃用户粘贴的链接（AI 整段透传免字符串处理）；前端 src/lib/anikitLink.ts 同逻辑。
//! 纯函数不依赖 tauri（MCP 模块 #[cfg(not(test))] 编不进测试，故落 core 可单测）

/// anikit 内部形态解析：`anikit://item|folder/{id}` → Some((kind, id))；其余 None。
/// deep-link 命令行参数 / 内部跳转入口用（对外 http 形态归一化走 normalize_ref_id）
pub fn parse_ref(s: &str) -> Option<(String, String)> {
  let t = s.trim();
  for (prefix, kind) in [("anikit://item/", "item"), ("anikit://folder/", "folder")] {
    if let Some(rest) = t.strip_prefix(prefix) {
      let id = rest.trim_end_matches('/');
      if !id.is_empty() {
        return Some((kind.to_string(), id.to_string()));
      }
    }
  }
  None
}

/// anikit 引用归一化：双形态链接 → 纯 id；其余输入原样返回（trim 后）
pub fn normalize_ref_id(s: &str) -> String {
  let t = s.trim();
  if let Some((_kind, id)) = parse_ref(t) {
    return id;
  }
  let Some(rest) = t.strip_prefix("http://").or_else(|| t.strip_prefix("https://")) else {
    return t.to_string();
  };
  // 仅认本机回环（127.0.0.1 / localhost，端口任意）：剥 host:port 后按 (open/)item|folder/{id} 匹配
  let Some((host, path)) = rest.split_once('/') else {
    return t.to_string();
  };
  let host_name = host.split(':').next().unwrap_or("");
  if host_name != "127.0.0.1" && host_name != "localhost" {
    return t.to_string();
  }
  let path = path.strip_prefix("open/").unwrap_or(path);
  for prefix in ["item/", "folder/"] {
    if let Some(id) = path.strip_prefix(prefix) {
      let id = id.trim_end_matches('/');
      if !id.is_empty() {
        return id.to_string();
      }
    }
  }
  t.to_string()
}

/// 点击跳转 URI（http open 形态，server/media.rs 端点）：AI 回复 / 外部文档可点击回 AniKit
pub fn open_uri(port: u16, kind: &str, id: &str) -> String {
  format!("http://127.0.0.1:{port}/open/{kind}/{id}")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn normalize_internal_forms() {
    assert_eq!(normalize_ref_id("anikit://item/AN-20260901-ABCDEF"), "AN-20260901-ABCDEF");
    assert_eq!(normalize_ref_id("anikit://folder/F-1"), "F-1");
    assert_eq!(normalize_ref_id("  anikit://item/AN-1  "), "AN-1"); // trim
    assert_eq!(normalize_ref_id("anikit://item/AN-1/"), "AN-1"); // 尾斜杠容忍
  }

  #[test]
  fn normalize_http_forms() {
    assert_eq!(normalize_ref_id("http://127.0.0.1:17701/item/AN-1"), "AN-1");
    assert_eq!(normalize_ref_id("http://127.0.0.1:17701/open/item/AN-1"), "AN-1");
    assert_eq!(normalize_ref_id("http://127.0.0.1:50423/open/folder/F-2"), "F-2"); // 动态端口任意
    assert_eq!(normalize_ref_id("https://localhost:17701/item/AN-3"), "AN-3");
  }

  #[test]
  fn passthrough_plain_and_foreign() {
    assert_eq!(normalize_ref_id("AN-20260901-ABCDEF"), "AN-20260901-ABCDEF"); // 纯 id 原样
    assert_eq!(normalize_ref_id("root"), "root"); // 特殊文件夹 id 原样
    assert_eq!(normalize_ref_id("https://example.com/item/AN-1"), "https://example.com/item/AN-1"); // 非回环不认
    assert_eq!(normalize_ref_id("anikit://item/"), "anikit://item/"); // 空 id 原样（交后端报不存在）
    assert_eq!(normalize_ref_id(""), "");
  }

  #[test]
  fn parse_internal_ref() {
    assert_eq!(parse_ref("anikit://item/AN-1"), Some(("item".to_string(), "AN-1".to_string())));
    assert_eq!(parse_ref("anikit://folder/F-2"), Some(("folder".to_string(), "F-2".to_string())));
    assert_eq!(parse_ref("http://127.0.0.1:17701/item/AN-1"), None, "http 形态不归 parse_ref 管");
    assert_eq!(parse_ref("anikit://item/"), None);
  }

  #[test]
  fn open_uri_form() {
    assert_eq!(open_uri(17701, "item", "AN-1"), "http://127.0.0.1:17701/open/item/AN-1");
    assert_eq!(open_uri(50423, "folder", "F-2"), "http://127.0.0.1:50423/open/folder/F-2");
  }
}
