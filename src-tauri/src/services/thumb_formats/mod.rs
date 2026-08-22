//! 图片格式解码注册表（4.2 落地，2026-07-25 改适配器注册表）：image crate 覆盖不了的格式
//! 由本模块补充，无 Adobe 依赖出内容缩略图。
//! 扩展纪律（规范第 2 条 / 05 待办 2026-07-23）：**新格式 = 本目录新增一个文件 + 下面
//! `ADAPTERS` 加一行**，不改 probe/decode 分发逻辑；解析 / 解码失败返回 None 属"能力探测"
//! 语义，由 thumbnail 回退到格式图标。远期 Eagle 式按需安装 = sidecar 扩展包（exe + manifest
//! 声明扩展名，子进程喂路径收像素），本注册表是它的内置形态（MVP 后封装统一扩展接口）。

use std::path::Path;

use image::RgbaImage;

mod photoshop;
mod svg;
mod font;

/// 格式适配器：声明扩展名 + 宽高探测 + 解码 RGBA（缩略图管线统一入口）
pub struct FormatAdapter {
  pub exts: &'static [&'static str],
  pub probe: fn(&Path) -> Option<(u32, u32)>,
  pub decode: fn(&Path) -> Option<RgbaImage>,
}

/// 内置适配器注册表：新格式只在这里加一行（文件放同目录）
static ADAPTERS: &[FormatAdapter] = &[photoshop::ADAPTER, svg::ADAPTER, font::ADAPTER];

/// 探测宽高；未注册格式 / 解析失败返回 None
pub fn probe(path: &Path) -> Option<(u32, u32)> {
  let ext = ext_of(path);
  ADAPTERS.iter().find(|a| a.exts.contains(&ext.as_str())).and_then(|a| (a.probe)(path))
}

/// 解码为 RGBA；未注册格式 / 解码失败返回 None（由 thumbnail 回退格式图标）
pub fn decode(path: &Path) -> Option<RgbaImage> {
  let ext = ext_of(path);
  ADAPTERS.iter().find(|a| a.exts.contains(&ext.as_str())).and_then(|a| (a.decode)(path))
}

fn ext_of(path: &Path) -> String {
  path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn registry_routes_by_ext_and_ignores_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("文档.txt");
    std::fs::write(&txt, "hello").unwrap();
    assert_eq!(probe(&txt), None);
    assert!(decode(&txt).is_none());

    // 已注册扩展名即使内容坏也走对应适配器（返回 None 由 thumbnail 回退，而不是漏分发）
    let psd = dir.path().join("假.psd");
    std::fs::write(&psd, b"not a psd").unwrap();
    assert!(decode(&psd).is_none());
  }

  #[test]
  fn adapters_have_unique_exts() {
    let mut seen = std::collections::HashSet::new();
    for a in ADAPTERS {
      for ext in a.exts {
        assert!(seen.insert(ext), "扩展名重复注册：{ext}");
      }
    }
  }
}
