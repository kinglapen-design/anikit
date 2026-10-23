//! anikit 引用解析（2026-10-08 S5，PRD 02-09 双形态分工）：`anikit://item|folder/{id}`（内部标识符）
//! 与 `http(s)://127.0.0.1:{port}/(open/)item|folder/{id}`（对外可点击形态）归一化为纯 id——
//! MCP 工具参数直接吃用户粘贴的链接（AI 整段透传免字符串处理）；前端 src/lib/anikitLink.ts 同逻辑。
//! 2026-10-23 编辑器贴图直链扩 by-name 形态：`anikit://item/by-name/{文件名}`（粘贴写链接时资产
//! 可能尚未入库拿不到 ID，按 originName 解析，见 core::index::latest_by_origin_name）。
//! 纯函数不依赖 tauri（MCP 模块 #[cfg(not(test))] 编不进测试，故落 core 可单测）

/// anikit 内部形态解析：`anikit://item|folder/{id}` → Some((kind, id))；
/// `anikit://item/by-name/{文件名}` → Some(("item-by-name", percent 解码后的文件名)）；其余 None。
/// **by-name 必须先于 item/ 判**——否则 "by-name/xx" 被 "item/" 前缀命中当普通 id 吞掉
pub fn parse_ref(s: &str) -> Option<(String, String)> {
  let t = s.trim();
  if let Some(rest) = t.strip_prefix("anikit://item/by-name/") {
    let name = percent_decode(rest.trim_end_matches('/'));
    // 空名 = 无效引用直接 None，不回退 item/ 解析（"by-name" 不是合法 id）
    return if name.is_empty() { None } else { Some(("item-by-name".to_string(), name)) };
  }
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

/// anikit 引用归一化：双形态链接 → 纯 id；其余输入原样返回（trim 后）。
/// by-name 形态（item-by-name）不是 id 引用（写链接时资产可能尚未入库），原样透传交解析方，
/// 不退化成文件名当 id
pub fn normalize_ref_id(s: &str) -> String {
  let t = s.trim();
  if let Some((kind, id)) = parse_ref(t) {
    return if kind == "item-by-name" { t.to_string() } else { id };
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
  // by-name 媒体形态同上：非 id 引用，原样透传
  if path.starts_with("item/by-name/") {
    return t.to_string();
  }
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

/// 极简 percent-encode（URL path 段语义）：RFC3986 unreserved（A-Z a-z 0-9 - _ . ~）原样保留，
/// 其余字节一律 %XX 大写——非 ASCII / 控制字符全覆盖，斜杠兜底也编（文件名本不含斜杠）。
/// anikit-paste 拼 /item/by-name/{文件名} 链接用
pub fn percent_encode(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for &b in s.as_bytes() {
    match b {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
      _ => out.push_str(&format!("%{b:02X}")),
    }
  }
  out
}

/// 极简 percent-decode（URL path 语义）：%XX → 字节（中文等多字节 UTF-8 序列随解码还原）；
/// **不做 + 号转空格**；残缺 / 非十六进制序列原样保留。core 自实现：不引外部 crate、不依赖 services 层
///（pub：services/md_bundle 的 by-name 导出解析同用，2026-10-23）
pub fn percent_decode(s: &str) -> String {
  let b = s.as_bytes();
  let mut out = Vec::with_capacity(b.len());
  let mut i = 0;
  while i < b.len() {
    if b[i] == b'%' && i + 2 < b.len() {
      if let (Some(h), Some(l)) = (hex_val(b[i + 1]), hex_val(b[i + 2])) {
        out.push(h * 16 + l);
        i += 3;
        continue;
      }
    }
    out.push(b[i]);
    i += 1;
  }
  // 正常路径恒为合法 UTF-8（写入方按 URL 编码 UTF-8 字节流）；损坏序列按 lossy 兜底不炸
  String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// 十六进制字符值（大小写通吃）；非十六进制字符 None
fn hex_val(c: u8) -> Option<u8> {
  match c {
    b'0'..=b'9' => Some(c - b'0'),
    b'a'..=b'f' => Some(c - b'a' + 10),
    b'A'..=b'F' => Some(c - b'A' + 10),
    _ => None,
  }
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

  /// by-name 深链形态（2026-10-23 编辑器贴图直链）：先于 item/ 判，percent 解码文件名
  #[test]
  fn parse_by_name_ref() {
    assert_eq!(
      parse_ref("anikit://item/by-name/贴图-a.png"),
      Some(("item-by-name".to_string(), "贴图-a.png".to_string()))
    );
    // 中文 percent 编码（贴 = E8 B4 B4 / 图 = E5 9B BE）
    assert_eq!(
      parse_ref("anikit://item/by-name/%E8%B4%B4%E5%9B%BE-20261023-143052-789.png"),
      Some(("item-by-name".to_string(), "贴图-20261023-143052-789.png".to_string()))
    );
    assert_eq!(parse_ref("anikit://item/by-name/"), None, "空名 = 无效引用");
    // 普通 item 不受影响
    assert_eq!(parse_ref("anikit://item/AN-1"), Some(("item".to_string(), "AN-1".to_string())));
  }

  /// by-name 非 id 引用：normalize 原样透传（不退化成文件名 / by-name 前缀残段当 id）
  #[test]
  fn normalize_by_name_passthrough() {
    let deep = "anikit://item/by-name/%E8%B4%B4%E5%9B%BE-a.png";
    assert_eq!(normalize_ref_id(deep), deep);
    let http = "http://127.0.0.1:17701/item/by-name/x.png";
    assert_eq!(normalize_ref_id(http), http);
    let http_open = "http://127.0.0.1:17701/open/item/by-name/x.png";
    assert_eq!(normalize_ref_id(http_open), http_open);
  }

  #[test]
  fn percent_codec() {
    // encode：unreserved 原样，其余 %XX 大写
    assert_eq!(percent_encode("贴图-20261023-143052-789.png"), "%E8%B4%B4%E5%9B%BE-20261023-143052-789.png");
    assert_eq!(percent_encode("a b#c.png"), "a%20b%23c.png");
    assert_eq!(percent_encode("a/b.png"), "a%2Fb.png", "斜杠兜底编（文件名本不含）");
    // decode：大小写十六进制通吃；+ 不转空格（URL path 语义）；残缺序列原样保留
    assert_eq!(percent_decode("a%20b.png"), "a b.png");
    assert_eq!(percent_decode("%e8%b4%b4.png"), "贴.png");
    assert_eq!(percent_decode("a+b.png"), "a+b.png");
    assert_eq!(percent_decode("100%.png"), "100%.png");
    assert_eq!(percent_decode("%zz.png"), "%zz.png");
  }

  #[test]
  fn open_uri_form() {
    assert_eq!(open_uri(17701, "item", "AN-1"), "http://127.0.0.1:17701/open/item/AN-1");
    assert_eq!(open_uri(50423, "folder", "F-2"), "http://127.0.0.1:50423/open/folder/F-2");
  }
}
