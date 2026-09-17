//! Shell API 路径规范化（踩坑 078）：canonicalize 产物是 `\\?\` 逐字路径（verbatim path），
//! Shell/Explorer 系 API（SHCreateItemFromParsingName、SHFileOperationW 等）解析不了，
//! 静默失败——凡喂给这类 API 的路径一律先经本模块还原普通路径
//! （普通形 `\\?\C:\...` → `C:\...`；UNC 形 `\\?\UNC\s\sh` → `\\s\sh`）。

use std::path::Path;

/// 剥 `\\?\` 逐字前缀，得到 Shell API 可解析的普通路径字符串；无前缀原样返回。
/// 不校验存在性——纯字符串层规范化，调用方管语义。
pub fn to_shell_string(path: &Path) -> String {
  let s = path.as_os_str().to_string_lossy();
  if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
    format!(r"\\{rest}")
  } else {
    s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or_else(|| s.into_owned())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strips_verbatim_prefixes() {
    assert_eq!(to_shell_string(Path::new(r"\\?\C:\a\b.txt")), r"C:\a\b.txt");
    assert_eq!(to_shell_string(Path::new(r"\\?\UNC\server\share\f")), r"\\server\share\f");
    assert_eq!(to_shell_string(Path::new(r"C:\a\b.txt")), r"C:\a\b.txt");
    assert_eq!(to_shell_string(Path::new(r"\\server\share\f")), r"\\server\share\f");
  }
}
