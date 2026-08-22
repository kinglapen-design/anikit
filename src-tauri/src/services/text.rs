//! 文本资产读写（5.6）：txt / md 预览读取 + APP 内编辑保存。
//! 编码口径：读 = UTF-8（含 BOM 剥离）优先、失败 GBK 回退（Windows 中文用户 txt 常态 ANSI）；
//! 写 = 恒 UTF-8 无 BOM。内容上限 2MB（提示词 / 笔记场景远超够用，防爆内存）。

use std::fs;
use std::path::Path;

/// 文本格式（预览 kind 分流共用判定）
const TEXT_EXTS: &[&str] = &["txt", "md"];

/// 文本内容上限（2MB）
pub const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

pub fn is_text(path: &Path) -> bool {
  let ext = path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
  TEXT_EXTS.contains(&ext.as_str())
}

/// 读取文本内容：UTF-8（BOM 剥离）优先，解码失败 GBK 回退；超上限中文报错
pub fn read_text(origin: &Path) -> Result<String, String> {
  let size = fs::metadata(origin).map_err(|e| format!("文件读取失败：{e}"))?.len();
  if size > MAX_TEXT_BYTES {
    return Err("文件超过 2MB，暂不支持预览编辑".to_string());
  }
  let bytes = fs::read(origin).map_err(|e| format!("文件读取失败：{e}"))?;
  Ok(decode_text(&bytes))
}

/// 解码（抽出来供单测）：UTF-8 BOM 剥离 → UTF-8 严格解码 → GBK 回退
fn decode_text(bytes: &[u8]) -> String {
  let stripped = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
  match String::from_utf8(stripped.to_vec()) {
    Ok(s) => s,
    Err(_) => {
      let (s, _, _) = encoding_rs::GBK.decode(stripped);
      s.into_owned()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ext_gate() {
    assert!(is_text(Path::new("笔记.txt")));
    assert!(is_text(Path::new("提示词.md")));
    assert!(!is_text(Path::new("字库.ttf")));
  }

  #[test]
  fn decode_utf8_with_and_without_bom() {
    assert_eq!(decode_text("你好".as_bytes()), "你好");
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice("提示词".as_bytes());
    assert_eq!(decode_text(&bom), "提示词");
  }

  #[test]
  fn decode_gbk_fallback() {
    // "你好" GBK 字节（非合法 UTF-8）
    let gbk = [0xC4u8, 0xE3, 0xBA, 0xC3];
    assert_eq!(decode_text(&gbk), "你好");
  }

  #[test]
  fn read_text_rejects_oversize() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("大.txt");
    fs::write(&f, vec![b'a'; (MAX_TEXT_BYTES + 1) as usize]).unwrap();
    let err = read_text(&f).unwrap_err();
    assert!(err.contains("2MB"), "{err}");
  }

  #[test]
  fn read_text_roundtrip_utf8() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("笔记.md");
    fs::write(&f, "# 标题\n正文 🎬").unwrap();
    assert_eq!(read_text(&f).unwrap(), "# 标题\n正文 🎬");
  }
}
