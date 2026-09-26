//! 内嵌提示词扫描（2026-09-26 提示词轮，PRD 03_数据链接 02_文件「分析提示词」）：
//! 本地解析图片内嵌元数据（无 AI），门禁仅 png / jpg / jpeg / webp——其他格式报错
//! 「该格式无内嵌提示词可分析」；解析不到报错「未检测到内嵌提示词」。
//! - PNG：手写 chunk walk（8 字节签名后逐 chunk：length + type + data + crc）——
//!   tEXt（keyword\0text；keyword latin1，文本 UTF-8 严格优先 / latin1 兜底——规范是 latin1，
//!   现实工具直接写 UTF-8 字节）/ zTXt（同前 + zlib inflate）/ iTXt（keyword\0 压缩标记
//!   压缩方式 language\0 translated\0 text，恒 UTF-8，标记 = 1 时 zlib inflate）；
//!   eXIf chunk 与 JPEG / WEBP 共用同一 TIFF 解析。
//! - JPEG / WEBP：image 0.25 ImageDecoder::exif_metadata() 拿 EXIF 字节（jpeg/png/webp 解码器
//!   均实现）→ 手写 TIFF 头解析（II / MM 字节序 + IFD walk）取 UserComment(0x9286，剥 8 字节
//!   charset 前缀如 "ASCII\0\0\0") 与 ImageDescription(0x010E)。
//! - ComfyUI：chunk key `prompt` 的值是节点图 JSON——抽 class_type == "CLIPTextEncode" 的
//!   inputs.text（String 或数组），按出现序去重，双换行拼接；解析失败 / 无命中回落原 JSON 文本。
//!
//! 输出取首个非空源，优先级 parameters → Comment → Description → UserComment → ImageDescription
//! → ComfyUI 抽取（定案列出前四与 ComfyUI；ImageDescription 为 EXIF 第二源，排在 UserComment 后）。
//!
//! 零新依赖：flate2 提为直接依赖（已在 Cargo.lock 传递依赖树，zlib inflate 用），image 0.25 自带。

use std::fs;
use std::io::Read;
use std::path::Path;

/// 内嵌提示词扫描入口：按扩展名分流（门禁仅 png / jpg / jpeg / webp）
pub fn scan_prompt(origin: &Path, ext: &str) -> Result<String, String> {
  match ext.to_ascii_lowercase().as_str() {
    "png" => scan_png(origin),
    "jpg" | "jpeg" => scan_jpeg(origin),
    "webp" => scan_webp(origin),
    _ => Err("该格式无内嵌提示词可分析".to_string()),
  }
}

/// 候选归一：空白（含全 NUL / 空白符）视为无命中
fn non_empty(s: String) -> Option<String> {
  let t = s.trim().trim_matches('\0').trim();
  if t.is_empty() { None } else { Some(t.to_string()) }
}

// ---------- PNG ----------

const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// PNG 文本 chunk 候选（一次 walk 收集，按定案优先级取用）
#[derive(Default)]
struct PngCandidates {
  parameters: Option<String>,
  comment: Option<String>,
  description: Option<String>,
  /// ComfyUI `prompt` chunk 原始文本（抽取失败 / 无命中回落它本身）
  comfy_raw: Option<String>,
  /// eXIf chunk 原始字节（TIFF 数据，无 "Exif\0\0" 前缀）
  exif: Option<Vec<u8>>,
}

fn scan_png(path: &Path) -> Result<String, String> {
  let bytes = fs::read(path).map_err(|e| format!("文件读取失败：{e}"))?;
  let cand = walk_png_chunks(&bytes);
  for c in [cand.parameters, cand.comment, cand.description] {
    if let Some(t) = c.and_then(non_empty) {
      return Ok(t);
    }
  }
  if let Some(exif) = cand.exif.as_deref() {
    if let Some(t) = exif_prompt(exif) {
      return Ok(t);
    }
  }
  if let Some(raw) = cand.comfy_raw.and_then(non_empty) {
    return Ok(extract_comfy(&raw));
  }
  Err("未检测到内嵌提示词".to_string())
}

/// PNG chunk walk：签名后逐 chunk（length u32 BE + type 4 + data + crc 4，crc 不校验——
/// 只读提示词字段，容忍轻微损坏）；截断 / 畸形即停（已有候选照常取用）
fn walk_png_chunks(bytes: &[u8]) -> PngCandidates {
  let mut cand = PngCandidates::default();
  if bytes.len() < 8 || &bytes[0..8] != PNG_SIG {
    return cand;
  }
  let mut pos = 8;
  while pos + 8 <= bytes.len() {
    let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
    let ctype: &[u8] = &bytes[pos + 4..pos + 8];
    let Some(data_end) = (pos + 8).checked_add(len) else { break };
    match data_end.checked_add(4) {
      Some(end) if end <= bytes.len() => {}
      _ => break, // data + crc 越界 = 截断
    }
    let data = &bytes[pos + 8..data_end];
    match ctype {
      b"tEXt" => {
        if let Some((k, v)) = parse_text_chunk(data) {
          collect_png_text(&mut cand, &k, v);
        }
      }
      b"zTXt" => {
        if let Some((k, v)) = parse_ztxt_chunk(data) {
          collect_png_text(&mut cand, &k, v);
        }
      }
      b"iTXt" => {
        if let Some((k, v)) = parse_itxt_chunk(data) {
          collect_png_text(&mut cand, &k, v);
        }
      }
      b"eXIf" => cand.exif = Some(data.to_vec()),
      b"IEND" => break,
      _ => {}
    }
    pos = data_end + 4;
  }
  cand
}

/// 按 keyword 归位候选（大小写不敏感——各家工具大小写实践不一，2026-09-26 定案口径宽容）
fn collect_png_text(cand: &mut PngCandidates, keyword: &str, text: String) {
  if keyword.eq_ignore_ascii_case("parameters") {
    cand.parameters.get_or_insert(text);
  } else if keyword.eq_ignore_ascii_case("comment") {
    cand.comment.get_or_insert(text);
  } else if keyword.eq_ignore_ascii_case("description") {
    cand.description.get_or_insert(text);
  } else if keyword.eq_ignore_ascii_case("prompt") {
    cand.comfy_raw.get_or_insert(text);
  }
}

/// latin1 → String（iTXt keyword 的字节口径，PNG 规范）
fn latin1(bytes: &[u8] ) -> String {
  bytes.iter().map(|&b| b as char).collect()
}

/// tEXt / zTXt 文本解码：规范口径是 latin1，但现实工具（SD 系等）直接写 UTF-8 字节——
/// 严格 UTF-8 优先，非法序列回退 latin1 映射（2026-09-26 实测口径，否则中文提示词乱码）
fn decode_text_field(bytes: &[u8]) -> String {
  String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| latin1(bytes))
}

/// tEXt：keyword\0text
fn parse_text_chunk(data: &[u8]) -> Option<(String, String)> {
  let sep = data.iter().position(|&b| b == 0)?;
  Some((latin1(&data[..sep]), decode_text_field(&data[sep + 1..])))
}

/// zTXt：keyword\0 压缩方式（1 字节，0 = deflate/zlib）压缩后文本
fn parse_ztxt_chunk(data: &[u8]) -> Option<(String, String)> {
  let sep = data.iter().position(|&b| b == 0)?;
  if data.get(sep + 1) != Some(&0) {
    return None; // 未知压缩方式
  }
  let text = inflate(&data[sep + 2..]).map(|b| decode_text_field(&b))?;
  Some((latin1(&data[..sep]), text))
}

/// iTXt：keyword\0 压缩标记（1）压缩方式（1）language\0 translated\0 text（恒 UTF-8）
fn parse_itxt_chunk(data: &[u8]) -> Option<(String, String)> {
  let sep = data.iter().position(|&b| b == 0)?;
  let keyword = latin1(&data[..sep]);
  let compressed = *data.get(sep + 1)? == 1;
  let mut rest = sep + 3; // 压缩标记 + 压缩方式各 1 字节（方式值不校验，inflate 失败自然落空）
  for _ in 0..2 {
    // 跳过 language\0 translated\0 两段
    let end = data[rest..].iter().position(|&b| b == 0)? + rest;
    rest = end + 1;
  }
  if rest > data.len() {
    return None;
  }
  let text = if compressed {
    String::from_utf8_lossy(&inflate(&data[rest..])?).into_owned()
  } else {
    String::from_utf8_lossy(&data[rest..]).into_owned()
  };
  Some((keyword, text))
}

/// zlib inflate（PNG zTXt / iTXt 压缩文本）；上限 4MB 防解压炸弹
fn inflate(data: &[u8]) -> Option<Vec<u8>> {
  let mut out = Vec::new();
  flate2::read::ZlibDecoder::new(data).take(4 * 1024 * 1024).read_to_end(&mut out).ok()?;
  Some(out)
}

// ---------- JPEG / WEBP（EXIF） ----------

fn scan_jpeg(path: &Path) -> Result<String, String> {
  let file = fs::File::open(path).map_err(|e| format!("文件读取失败：{e}"))?;
  let mut dec = image::codecs::jpeg::JpegDecoder::new(std::io::BufReader::new(file)).map_err(|e| format!("图片解析失败：{e}"))?;
  let exif = image::ImageDecoder::exif_metadata(&mut dec).map_err(|e| format!("图片解析失败：{e}"))?;
  exif.as_deref().and_then(exif_prompt).ok_or_else(|| "未检测到内嵌提示词".to_string())
}

fn scan_webp(path: &Path) -> Result<String, String> {
  let file = fs::File::open(path).map_err(|e| format!("文件读取失败：{e}"))?;
  let mut dec = image::codecs::webp::WebPDecoder::new(std::io::BufReader::new(file)).map_err(|e| format!("图片解析失败：{e}"))?;
  let exif = image::ImageDecoder::exif_metadata(&mut dec).map_err(|e| format!("图片解析失败：{e}"))?;
  exif.as_deref().and_then(exif_prompt).ok_or_else(|| "未检测到内嵌提示词".to_string())
}

/// EXIF 字节 → 提示词：UserComment(0x9286) 优先，ImageDescription(0x010E) 次之；
/// 兼容带 "Exif\0\0" 前缀的形态（JPEG APP1 原始载荷 / 部分 WebP 编码器）
fn exif_prompt(bytes: &[u8]) -> Option<String> {
  let tiff = bytes.strip_prefix(b"Exif\0\0").unwrap_or(bytes);
  let (user_comment, image_desc) = parse_tiff(tiff)?;
  user_comment.and_then(non_empty).or_else(|| image_desc.and_then(non_empty))
}

// ---------- TIFF 手写解析（II / MM 字节序 + IFD walk） ----------

fn read_u16(d: &[u8], off: usize, le: bool) -> Option<u16> {
  let b: [u8; 2] = d.get(off..off + 2)?.try_into().ok()?;
  Some(if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) })
}

fn read_u32(d: &[u8], off: usize, le: bool) -> Option<u32> {
  let b: [u8; 4] = d.get(off..off + 4)?.try_into().ok()?;
  Some(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
}

/// TIFF 字段类型单位字节数（只要 1/2/7 够用，其余给全表防算错偏移）
fn type_size(t: u16) -> Option<usize> {
  Some(match t {
    1 | 2 | 6 | 7 => 1,
    3 | 8 => 2,
    4 | 9 | 11 => 4,
    5 | 10 | 12 => 8,
    _ => return None,
  })
}

/// 解析 TIFF 头 + IFD0（含 0x8769 EXIF 子 IFD 跟进一层），
/// 返回（UserComment, ImageDescription）原始文本（均未命中 = None）
fn parse_tiff(d: &[u8]) -> Option<(Option<String>, Option<String>)> {
  if d.len() < 8 {
    return None;
  }
  let le = match &d[0..2] {
    b"II" => true,
    b"MM" => false,
    _ => return None,
  };
  if read_u16(d, 2, le)? != 42 {
    return None;
  }
  let ifd0 = read_u32(d, 4, le)? as usize;
  let mut found = TiffFound::default();
  walk_ifd(d, ifd0, le, &mut found);
  // UserComment 常挂 EXIF 子 IFD（0x8769 指针）；IFD0 已命中则不跟进
  if found.user_comment.is_none() {
    if let Some(sub) = found.exif_ifd {
      walk_ifd(d, sub, le, &mut found);
    }
  }
  Some((found.user_comment, found.image_desc))
}

#[derive(Default)]
struct TiffFound {
  user_comment: Option<String>,
  image_desc: Option<String>,
  exif_ifd: Option<usize>,
}

/// 单 IFD walk：逐 12 字节条目取 0x010E / 0x9286 / 0x8769；全部读取带边界检查，畸形即停
fn walk_ifd(d: &[u8], offset: usize, le: bool, found: &mut TiffFound) {
  let Some(count) = read_u16(d, offset, le) else { return };
  for i in 0..count as usize {
    let Some(entry) = (offset + 2).checked_add(i.saturating_mul(12)) else { return };
    if entry + 12 > d.len() {
      return;
    }
    let (Some(tag), Some(typ), Some(cnt)) =
      (read_u16(d, entry, le), read_u16(d, entry + 2, le), read_u32(d, entry + 4, le))
    else {
      return;
    };
    let Some(unit) = type_size(typ) else { continue };
    let Some(total) = (cnt as usize).checked_mul(unit) else { return };
    // 值 ≤ 4 字节内联在条目值位，否则值为绝对偏移（自 TIFF 头起）
    let value: Option<&[u8]> = if total <= 4 {
      d.get(entry + 8..entry + 8 + total)
    } else {
      read_u32(d, entry + 8, le).and_then(|off| d.get(off as usize..off as usize + total))
    };
    let Some(value) = value else { continue };
    match tag {
      0x010E => found.image_desc = Some(decode_ascii(value)),
      0x9286 => found.user_comment = Some(decode_user_comment(value)),
      0x8769 => found.exif_ifd = read_u32(d, entry + 8, le).map(|v| v as usize),
      _ => {}
    }
  }
}

/// ASCII 字段（ImageDescription）：剥尾 NUL，UTF-8 lossy（超范围字节不炸）
fn decode_ascii(value: &[u8]) -> String {
  let s = value.strip_suffix(&[0]).unwrap_or(value);
  String::from_utf8_lossy(s).into_owned()
}

/// UserComment(0x9286)：前 8 字节 = charset 标识（"ASCII\0\0\0" / "UNICODE\0" / "JIS\0\0\0\0\0"
/// / 全零未定义）需剥离；UNICODE 按 UTF-16BE 解码，其余 UTF-8 lossy。
/// 前缀不匹配任何已知形态 = 编码器没写头，原样按 UTF-8 lossy
fn decode_user_comment(value: &[u8]) -> String {
  if value.len() > 8 {
    let (head, body) = value.split_at(8);
    if head == b"UNICODE\0" {
      let units: Vec<u16> = body.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
      return String::from_utf16_lossy(&units);
    }
    if head == b"ASCII\0\0\0" || head == b"JIS\0\0\0\0\0" || head == [0; 8] {
      return String::from_utf8_lossy(body).into_owned();
    }
  }
  String::from_utf8_lossy(value).into_owned()
}

// ---------- ComfyUI prompt chunk ----------

/// ComfyUI `prompt` chunk 抽取：节点图 JSON 遍历，收集 class_type == "CLIPTextEncode" 的
/// inputs.text（String 或数组——数组为动态输入形态），按出现序去重后双换行拼接；
/// JSON 解析失败 / 无命中节点 → 原 JSON 文本返回（定案回落口径）
fn extract_comfy(raw: &str) -> String {
  let Ok(serde_json::Value::Object(graph)) = serde_json::from_str::<serde_json::Value>(raw) else {
    return raw.to_string();
  };
  let mut texts: Vec<String> = Vec::new();
  for node in graph.values() {
    let Some(node) = node.as_object() else { continue };
    if node.get("class_type").and_then(|c| c.as_str()) != Some("CLIPTextEncode") {
      continue;
    }
    let Some(text) = node.get("inputs").and_then(|i| i.as_object()).and_then(|i| i.get("text")) else { continue };
    match text {
      serde_json::Value::String(s) => push_unique(&mut texts, s),
      serde_json::Value::Array(arr) => {
        for item in arr {
          if let Some(s) = item.as_str() {
            push_unique(&mut texts, s);
          }
        }
      }
      _ => {}
    }
  }
  if texts.is_empty() { raw.to_string() } else { texts.join("\n\n") }
}

/// 按出现序去重（空串不收）
fn push_unique(texts: &mut Vec<String>, s: &str) {
  if !s.trim().is_empty() && !texts.iter().any(|t| t == s) {
    texts.push(s.to_string());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 拼最小 PNG：签名 + IHDR（13 字节占位数据）+ 给定 chunk + IEND；crc 全 0（解析不校验）
  fn png_with(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = PNG_SIG.to_vec();
    let mut all: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"IHDR", vec![0; 13])];
    all.extend(chunks.iter().map(|(t, d)| (*t, d.clone())));
    all.push((b"IEND", Vec::new()));
    for (ctype, data) in all {
      out.extend_from_slice(&(data.len() as u32).to_be_bytes());
      out.extend_from_slice(ctype);
      out.extend_from_slice(&data);
      out.extend_from_slice(&[0; 4]);
    }
    out
  }

  fn text_chunk(keyword: &str, text: &str) -> Vec<u8> {
    let mut d = keyword.as_bytes().to_vec();
    d.push(0);
    d.extend_from_slice(text.as_bytes());
    d
  }

  /// iTXt：keyword\0 flag method lang\0 translated\0 text
  fn itxt_chunk(keyword: &str, flag: u8, text_bytes: &[u8]) -> Vec<u8> {
    let mut d = keyword.as_bytes().to_vec();
    d.extend_from_slice(&[0, flag, 0, 0, 0]); // keyword\0 + flag + method + lang\0 + translated\0
    d.extend_from_slice(text_bytes);
    d
  }

  fn zlib(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
  }

  fn scan_png_bytes(bytes: &[u8]) -> Result<String, String> {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("图.png");
    fs::write(&f, bytes).unwrap();
    scan_prompt(&f, "png")
  }

  #[test]
  fn png_text_parameters_hit() {
    let png = png_with(&[(b"tEXt", text_chunk("parameters", "1girl, masterpiece\nNegative prompt: bad"))]);
    let got = scan_png_bytes(&png).unwrap();
    assert_eq!(got, "1girl, masterpiece\nNegative prompt: bad");
  }

  #[test]
  fn png_priority_parameters_over_comment_over_description() {
    let png = png_with(&[
      (b"tEXt", text_chunk("Description", "描述源")),
      (b"tEXt", text_chunk("Comment", "注释源")),
      (b"tEXt", text_chunk("parameters", "参数源")),
    ]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "参数源", "parameters 优先");
    let png = png_with(&[(b"tEXt", text_chunk("Description", "描述源")), (b"tEXt", text_chunk("Comment", "注释源"))]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "注释源", "Comment 次之");
    // keyword 大小写宽容
    let png = png_with(&[(b"tEXt", text_chunk("PARAMETERS", "大写键"))]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "大写键");
  }

  #[test]
  fn png_itxt_compressed_utf8() {
    let png = png_with(&[(b"iTXt", itxt_chunk("parameters", 1, &zlib("压缩的中文提示词 🎬".as_bytes())))]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "压缩的中文提示词 🎬");
    // 未压缩 iTXt
    let png = png_with(&[(b"iTXt", itxt_chunk("Comment", 0, "plain itxt".as_bytes()))]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "plain itxt");
  }

  #[test]
  fn png_ztxt_inflate_latin1() {
    let mut d = b"Description".to_vec();
    d.push(0);
    d.push(0); // 压缩方式 0 = deflate
    d.extend_from_slice(&zlib(b"ztxt body"));
    let png = png_with(&[(b"zTXt", d)]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "ztxt body");
  }

  #[test]
  fn png_comfy_extract_dedup_and_fallback() {
    // 命中：CLIPTextEncode 按出现序去重，双换行拼接
    let graph = serde_json::json!({
      "1": { "class_type": "CLIPTextEncode", "inputs": { "text": "正向：猫" } },
      "2": { "class_type": "KSampler", "inputs": { "steps": 20 } },
      "3": { "class_type": "CLIPTextEncode", "inputs": { "text": "正向：猫" } },
      "4": { "class_type": "CLIPTextEncode", "inputs": { "text": ["负向：崩坏", "负向：崩坏"] } }
    });
    let png = png_with(&[(b"tEXt", text_chunk("prompt", &graph.to_string()))]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "正向：猫\n\n负向：崩坏");

    // 回落：无 CLIPTextEncode 命中 → 原 JSON 文本
    let nohit = serde_json::json!({ "1": { "class_type": "KSampler", "inputs": {} } }).to_string();
    assert_eq!(extract_comfy(&nohit), nohit);
    // 回落：非 JSON → 原文返回（其他工具写入的纯文本 prompt 键同理）
    assert_eq!(extract_comfy("not json {{{"), "not json {{{");
    // ComfyUI 优先级低于 parameters
    let png = png_with(&[
      (b"tEXt", text_chunk("prompt", &graph.to_string())),
      (b"tEXt", text_chunk("parameters", "参数优先")),
    ]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "参数优先");
  }

  /// 最小 EXIF TIFF 字节（II 小端）：IFD0 挂 ImageDescription + 0x8769 指针 → 子 IFD 挂 UserComment
  fn exif_tiff_le(user_comment: Option<&[u8]>, image_desc: Option<&[u8]>) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(b"II");
    d.extend_from_slice(&42u16.to_le_bytes());
    d.extend_from_slice(&8u32.to_le_bytes()); // IFD0 偏移
    let ifd0_count: u16 = image_desc.is_some() as u16 + user_comment.is_some() as u16;
    d.extend_from_slice(&ifd0_count.to_le_bytes());
    // 布局：IFD0 条目区 → next(4) →（有 UserComment 时）子 IFD（count + 1 条目 + next）→ 数据区
    let ifd0_entries_end = 8 + 2 + ifd0_count as usize * 12;
    let sub_ifd_off = ifd0_entries_end + 4;
    let data_off = if user_comment.is_some() { sub_ifd_off + 2 + 12 + 4 } else { ifd0_entries_end + 4 };
    let mut data = Vec::new();
    let mut entry = |tag: u16, typ: u16, value: &[u8]| {
      let mut e = Vec::new();
      e.extend_from_slice(&tag.to_le_bytes());
      e.extend_from_slice(&typ.to_le_bytes());
      e.extend_from_slice(&(value.len() as u32).to_le_bytes());
      if value.len() <= 4 {
        e.extend_from_slice(value);
        e.resize(e.len() + (4 - value.len()), 0);
      } else {
        let off = (data_off + data.len()) as u32;
        e.extend_from_slice(&off.to_le_bytes());
        data.extend_from_slice(value);
      }
      e
    };
    if let Some(desc) = image_desc {
      let mut v = desc.to_vec();
      v.push(0); // ASCII 尾 NUL
      d.extend_from_slice(&entry(0x010E, 2, &v));
    }
    if user_comment.is_some() {
      d.extend_from_slice(&entry(0x8769, 4, &(sub_ifd_off as u32).to_le_bytes()));
    }
    d.extend_from_slice(&0u32.to_le_bytes()); // next IFD
    if let Some(uc) = user_comment {
      d.extend_from_slice(&1u16.to_le_bytes());
      d.extend_from_slice(&entry(0x9286, 7, uc));
      d.extend_from_slice(&0u32.to_le_bytes());
    }
    d.extend_from_slice(&data);
    d
  }

  #[test]
  fn exif_user_comment_beats_image_description() {
    let mut uc = b"ASCII\0\0\0".to_vec();
    uc.extend_from_slice("用户注释提示词".as_bytes());
    let tiff = exif_tiff_le(Some(&uc), Some("图像描述".as_bytes()));
    let (u, i) = parse_tiff(&tiff).unwrap();
    assert_eq!(u.as_deref(), Some("用户注释提示词"));
    assert_eq!(i.as_deref(), Some("图像描述"));
    assert_eq!(exif_prompt(&tiff).as_deref(), Some("用户注释提示词"), "UserComment 优先");
    // 仅 ImageDescription
    let tiff = exif_tiff_le(None, Some("仅描述".as_bytes()));
    assert_eq!(exif_prompt(&tiff).as_deref(), Some("仅描述"));
    // "Exif\0\0" 前缀形态同解析（JPEG APP1 原始载荷口径）
    let mut prefixed = b"Exif\0\0".to_vec();
    prefixed.extend_from_slice(&tiff);
    assert_eq!(exif_prompt(&prefixed).as_deref(), Some("仅描述"));
  }

  #[test]
  fn exif_user_comment_unicode_prefix_decodes_utf16be() {
    let units: Vec<u16> = "你好提示词".encode_utf16().collect();
    let mut uc = b"UNICODE\0".to_vec();
    for u in units {
      uc.extend_from_slice(&u.to_be_bytes());
    }
    let tiff = exif_tiff_le(Some(&uc), None);
    assert_eq!(exif_prompt(&tiff).as_deref(), Some("你好提示词"));
    // 全零未定义头同剥离
    let mut uc = vec![0u8; 8];
    uc.extend_from_slice("plain".as_bytes());
    let tiff = exif_tiff_le(Some(&uc), None);
    assert_eq!(exif_prompt(&tiff).as_deref(), Some("plain"));
  }

  #[test]
  fn png_exif_chunk_walks_same_tiff() {
    let mut uc = b"ASCII\0\0\0".to_vec();
    uc.extend_from_slice("eXIf 里的提示词".as_bytes());
    let png = png_with(&[(b"eXIf", exif_tiff_le(Some(&uc), None))]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "eXIf 里的提示词");
    // EXIF 源低于 tEXt parameters、高于 ComfyUI
    let png = png_with(&[
      (b"eXIf", exif_tiff_le(Some(&uc), None)),
      (b"tEXt", text_chunk("prompt", r#"{"1":{}}"#)),
    ]);
    assert_eq!(scan_png_bytes(&png).unwrap(), "eXIf 里的提示词", "UserComment 高于 ComfyUI");
  }

  #[test]
  fn gate_and_no_hit() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("动图.gif");
    fs::write(&f, b"GIF89a").unwrap();
    assert_eq!(scan_prompt(&f, "gif").unwrap_err(), "该格式无内嵌提示词可分析");
    assert_eq!(scan_prompt(&f, "GIF").unwrap_err(), "该格式无内嵌提示词可分析", "扩展名大小写不敏感");
    // 无任何文本 chunk 的 PNG
    let png = png_with(&[]);
    assert_eq!(scan_png_bytes(&png).unwrap_err(), "未检测到内嵌提示词");
    // 截断 PNG（截进 tEXt chunk 体内 → 越界即停，无候选）
    let mut broken = png_with(&[(b"tEXt", text_chunk("parameters", "x"))]);
    broken.truncate(broken.len() - 20);
    assert_eq!(scan_png_bytes(&broken).unwrap_err(), "未检测到内嵌提示词");
    // 空白文本视为无命中
    let png = png_with(&[(b"tEXt", text_chunk("parameters", "   \0  "))]);
    assert_eq!(scan_png_bytes(&png).unwrap_err(), "未检测到内嵌提示词");
  }

  #[test]
  fn jpeg_webp_without_exif_report_no_prompt() {
    // image crate 现编码最小 JPEG / WebP（无 EXIF）→ 未检测到
    let dir = tempfile::tempdir().unwrap();
    let jpg = dir.path().join("图.jpg");
    // JPEG 编码不支持 RGBA（image crate 门禁），用 RGB 图
    image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3])).save(&jpg).unwrap();
    assert_eq!(scan_prompt(&jpg, "jpg").unwrap_err(), "未检测到内嵌提示词");
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]));
    let webp = dir.path().join("图.webp");
    img.save(&webp).unwrap();
    assert_eq!(scan_prompt(&webp, "webp").unwrap_err(), "未检测到内嵌提示词");
  }
}
