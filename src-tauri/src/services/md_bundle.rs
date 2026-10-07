//! MD 打包导出（2026-10-08 S6，PRD 02-09「打包导出」定案——AE 收集打包的 MD 版）：
//! 扫描文档内 anikit 资产引用（anikit://item/{id} 与 http(s)://127.0.0.1:{port}/item/{id} 双形态，
//! 同一归一化口径见 core::anikit_ref），图源与 server/media.rs /item/{id} 端点同口径
//!（图片 = origin 原图 / 非图片 = webp 缩略图）→ 导出 out_dir/assets/ + 引用 URL 改写相对路径。
//! open/ 回链形态不收集不改写（原样保留——接收者无 AniKit 点不开，文档回本机后仍可用）；
//! folder 引用不收集。纯函数与编排分离：scan/rewrite 可单测，export_bundle 做库查询与 IO。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::core::library::Library;
use crate::services::{asset_menu, views};

/// webview / 浏览器可直接渲染的图片扩展名（与 import_task WEBVIEW_DIRECT_EXTS 对齐 + avif）：
/// 命中 → 图源 = origin 原图；其余（视频 / PSD / 字体等）→ webp 缩略图。
/// server/media.rs /item/{id} 端点与本模块打包导出共用同一口径（单一常量点）
pub const DIRECT_IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif"];

/// 一处引用命中：content[start..end] = 完整 URL 子串，asset_id = 解析出的资产 id
#[derive(Debug, Clone, PartialEq)]
pub struct RefHit {
  pub start: usize,
  pub end: usize,
  pub asset_id: String,
}

fn id_char(b: u8) -> bool {
  b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// 从 start 起读 id（字母数字 + - + _），返回结束索引（不含）
fn read_id(content: &str, start: usize) -> usize {
  let bytes = content.as_bytes();
  let mut e = start;
  while e < bytes.len() && id_char(bytes[e]) {
    e += 1;
  }
  e
}

/// 扫描 MD 文本里的 anikit 资产引用（双形态；open/ 回链与 folder 引用不收集）。返回按 start 升序。
/// 全程字节比较不切 str——非 ASCII 文本（中文文档）无 char boundary 风险（踩坑预防：str 切片必须字符边界）
pub fn scan_refs(content: &str) -> Vec<RefHit> {
  const ANIKIT: &[u8] = b"anikit://item/";
  const LOOPBACK: &[&[u8]] = &[b"http://127.0.0.1:", b"https://127.0.0.1:", b"http://localhost:", b"https://localhost:"];
  let mut hits = Vec::new();
  let bytes = content.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    // anikit://item/{id}
    if bytes[i..].starts_with(ANIKIT) {
      let id_start = i + ANIKIT.len();
      let id_end = read_id(content, id_start);
      if id_end > id_start {
        hits.push(RefHit { start: i, end: id_end, asset_id: content[id_start..id_end].to_string() });
        i = id_end;
        continue;
      }
      i += 1;
      continue;
    }
    // http(s)://127.0.0.1:{port}/item/{id}（localhost 同；open/ 前缀不收集）
    if let Some(scheme) = LOOPBACK.iter().find(|s| bytes[i..].starts_with(s)) {
      let port_start = i + scheme.len();
      let mut j = port_start;
      while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
      }
      // 端口段须非空且紧随 '/' + item/
      if j > port_start && j < bytes.len() && bytes[j] == b'/' && bytes[j + 1..].starts_with(b"item/") {
        let id_start = j + 1 + "item/".len();
        let id_end = read_id(content, id_start);
        if id_end > id_start {
          hits.push(RefHit { start: i, end: id_end, asset_id: content[id_start..id_end].to_string() });
          i = id_end;
          continue;
        }
      }
    }
    i += 1;
  }
  hits
}

/// 引用 URL 改写：url_of(asset_id) = Some("assets/xx.png") → 该处 URL 替换；None 保留原样。
/// 从后往前替换保持索引有效（hits 升序传入）
pub fn rewrite(content: &str, hits: &[RefHit], url_of: impl Fn(&str) -> Option<String>) -> String {
  let mut out = content.to_string();
  for h in hits.iter().rev() {
    if let Some(url) = url_of(&h.asset_id) {
      out.replace_range(h.start..h.end, &url);
    }
  }
  out
}

/// 打包报告（命令回包，camelCase）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleReport {
  /// 产物目录（{文档名}-打包/）
  pub out_dir: String,
  /// 导出图片张数（去重后）
  pub images: u32,
  /// 改写后的 md 副本文件名
  pub doc_file: String,
  /// 跳过清单（人类可读原因：资产不存在 / 无可用图片等）
  pub skipped: Vec<String>,
}

/// 图源解析（与 server/media.rs /item/{id} 同口径）：图片扩展名 → origin 原图；
/// 其余 → webp 缩略图（含 thumbAssetId 伴生代理）。返回（源文件路径, 期望导出文件名）
fn image_source(lib: &Library, asset_id: &str) -> Result<Option<(PathBuf, String)>, String> {
  let Some(meta) = lib.index.assets.get(asset_id) else {
    return Ok(None);
  };
  if DIRECT_IMAGE_EXTS.contains(&meta.ext.to_ascii_lowercase().as_str()) {
    let origin = asset_menu::real_origin_file(lib, asset_id)?;
    let name = origin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| format!("{asset_id}.{}", meta.ext));
    return Ok(Some((origin, name)));
  }
  let thumb_source = meta
    .thumb_asset_id
    .as_deref()
    .and_then(|i| lib.index.assets.get(i))
    .filter(|t| t.hidden == Some(true))
    .unwrap_or(meta);
  Ok(views::thumb_path(lib, thumb_source, &lib.format_icons).map(|p| (PathBuf::from(p), format!("{}.webp", meta.name))))
}

/// 打包编排：扫引用 → 逐 id（保序去重）解析图源、分配 assets/ 文件名（重名 _n 规避）→
/// 写 assets/ + 改写后的 md 副本（out_dir 由调用方建好；副本文件名 = {doc_stem}.md）。
/// 跳过项进报告（资产不存在 / 无可用图片 / 读盘失败），不阻断其余
pub fn export_bundle(lib: &Library, doc_stem: &str, content: &str, out_dir: &Path) -> Result<BundleReport, String> {
  let hits = scan_refs(content);
  // 保序去重的 id 清单
  let mut ids: Vec<&str> = Vec::new();
  for h in &hits {
    if !ids.contains(&h.asset_id.as_str()) {
      ids.push(&h.asset_id);
    }
  }
  let assets_dir = out_dir.join("assets");
  let mut used: HashSet<String> = HashSet::new();
  let mut url_of_id: Vec<(String, String)> = Vec::new(); // (asset_id, "assets/{file}")
  let mut skipped: Vec<String> = Vec::new();
  if !ids.is_empty() {
    std::fs::create_dir_all(&assets_dir).map_err(|e| format!("创建 assets 目录失败：{e}"))?;
  }
  for id in ids {
    match image_source(lib, id) {
      Ok(Some((src, want_name))) => {
        let file = unique_name(&mut used, &want_name);
        if let Err(e) = std::fs::copy(&src, assets_dir.join(&file)) {
          skipped.push(format!("{id}：图片复制失败（{e}）"));
          continue;
        }
        url_of_id.push((id.to_string(), format!("assets/{file}")));
      }
      Ok(None) => skipped.push(format!("{id}：资产不存在，可能已被删除")),
      Err(e) => skipped.push(format!("{id}：{e}")),
    }
  }
  let find = |asset_id: &str| url_of_id.iter().find(|(id, _)| id == asset_id).map(|(_, u)| u.clone());
  let doc_file = format!("{doc_stem}.md");
  let rewritten = rewrite(content, &hits, |id| find(id));
  std::fs::write(out_dir.join(&doc_file), rewritten).map_err(|e| format!("写入文档副本失败：{e}"))?;
  Ok(BundleReport {
    out_dir: out_dir.to_string_lossy().into_owned(),
    images: url_of_id.len() as u32,
    doc_file,
    skipped,
  })
}

/// 文件名去重：{stem}_2.{ext} / {stem}_3.{ext}…（与库内 unique 口径同形）
fn unique_name(used: &mut HashSet<String>, want: &str) -> String {
  if used.insert(want.to_string()) {
    return want.to_string();
  }
  let (stem, ext) = match want.rsplit_once('.') {
    Some((s, e)) => (s.to_string(), format!(".{e}")),
    None => (want.to_string(), String::new()),
  };
  for n in 2.. {
    let candidate = format!("{stem}_{n}{ext}");
    if used.insert(candidate.clone()) {
      return candidate;
    }
  }
  unreachable!()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scan_anikit_form() {
    let hits = scan_refs("见 ![窗帘](anikit://item/AN-20260901-ABCDEF) 与 [链接](anikit://item/AN-2)");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].asset_id, "AN-20260901-ABCDEF");
    assert_eq!(hits[1].asset_id, "AN-2");
    assert_eq!(&"见 ![窗帘](anikit://item/AN-20260901-ABCDEF) 与 [链接](anikit://item/AN-2)"[hits[0].start..hits[0].end], "anikit://item/AN-20260901-ABCDEF");
  }

  #[test]
  fn scan_http_form_and_open_skipped() {
    let doc = "![a](http://127.0.0.1:17701/item/AN-1) 回链 [![b](http://127.0.0.1:17701/item/AN-2)](http://127.0.0.1:17701/open/item/AN-2) localhost ![c](http://localhost:50423/item/AN-3)";
    let hits = scan_refs(doc);
    let ids: Vec<&str> = hits.iter().map(|h| h.asset_id.as_str()).collect();
    assert_eq!(ids, ["AN-1", "AN-2", "AN-3"], "open/ 回链不收集、item 图双形态都收");
  }

  #[test]
  fn scan_foreign_and_folder_skipped() {
    assert!(scan_refs("![a](https://example.com/item/AN-1)").is_empty(), "非回环不认");
    assert!(scan_refs("[夹](anikit://folder/F-1)").is_empty(), "folder 引用不收集");
    assert!(scan_refs("http://127.0.0.1:/item/AN-1").is_empty(), "空端口不认");
  }

  #[test]
  fn rewrite_replaces_and_keeps_open() {
    let doc = "![a](anikit://item/AN-1) [![b](anikit://item/AN-2)](http://127.0.0.1:17701/open/item/AN-2)";
    let hits = scan_refs(doc);
    let out = rewrite(doc, &hits, |id| match id {
      "AN-1" => Some("assets/a.png".to_string()),
      "AN-2" => Some("assets/b.png".to_string()),
      _ => None,
    });
    assert_eq!(out, "![a](assets/a.png) [![b](assets/b.png)](http://127.0.0.1:17701/open/item/AN-2)");
  }

  #[test]
  fn unique_name_dedup() {
    let mut used = HashSet::new();
    assert_eq!(unique_name(&mut used, "a.png"), "a.png");
    assert_eq!(unique_name(&mut used, "a.png"), "a_2.png");
    assert_eq!(unique_name(&mut used, "a.png"), "a_3.png");
  }
}
