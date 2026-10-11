//! docx 服务（PRD 04-04 条 6）：两路——① **预览 = read_docx_bytes 给原始字节**（2026-10-10 mammoth 换引擎：
//! 解析 / 渲染全在前端 mammoth.js，表格带框线 + 图片内嵌；后端只管存在性 / 格式门 / 64MB 容量闸）；
//! ② 工作空间文本卡内容 = read_docx 段落提取（2026-09-22 R6 首例：zip 取 word/document.xml，quick-xml
//! 流式解析段落结构——标题级 / 粗斜体 / 列表标记；w:br → \n；上限：document.xml 解压后 32MB 截流防爆、
//! 文本 2MB 截断置 truncated）。零新后端依赖（zip / quick-xml 为 8.3 fla XFL 既有依赖）。

use std::fs;
use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;

/// document.xml 解压后上限（32MB）
const MAX_XML_BYTES: u64 = 32 * 1024 * 1024;
/// 提取文本总量上限（2MB，对齐 services/text MAX_TEXT_BYTES；按 UTF-8 字节计）
const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

/// 文本 run（粗 / 斜为 run 级标记）
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocxRun {
  pub text: String,
  pub bold: bool,
  pub italic: bool,
}

/// 段落（heading = 0 正文 / 1-6 标题级；list = 列表项标记，前端加项目符号）
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocxParagraph {
  pub heading: u8,
  pub list: bool,
  pub runs: Vec<DocxRun>,
}

/// 提取结果（truncated = 文本超 2MB 截断，前端提示「仅显示前一部分」）
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocxContent {
  pub paragraphs: Vec<DocxParagraph>,
  pub truncated: bool,
}

/// docx 判定（预览 kind 分流 / 命令格式门共用）
pub fn is_docx(path: &Path) -> bool {
  path
    .extension()
    .map(|s| s.to_string_lossy().eq_ignore_ascii_case("docx"))
    .unwrap_or(false)
}

/// 预览 HTML 化（2026-10-10 mammoth 换引擎）的文件字节上限（64MB——截图反馈文档图多量大；
/// 解压 / 转换由前端 mammoth + JSZip 自理，此闸只防巨型包一次读爆内存）
const MAX_DOCX_BYTES: u64 = 64 * 1024 * 1024;

/// 读取 docx 原始字节（ mammoth 预览链路：后端只做存在性 / 格式门 / 容量闸 + 给字节，解析全部前端）
pub fn read_docx_bytes(origin: &Path) -> Result<Vec<u8>, String> {
  if !is_docx(origin) {
    let name = origin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| origin.to_string_lossy().into_owned());
    return Err(format!("「{name}」不是 docx 资源"));
  }
  let size = fs::metadata(origin).map_err(|e| format!("文件读取失败：{e}"))?.len();
  if size > MAX_DOCX_BYTES {
    return Err("文档过大（超过 64MB），暂不支持预览".to_string());
  }
  fs::read(origin).map_err(|e| format!("文件读取失败：{e}"))
}

/// 读取并解析 docx：zip 取 word/document.xml → 段落结构提取
pub fn read_docx(origin: &Path) -> Result<DocxContent, String> {
  let file = fs::File::open(origin).map_err(|e| format!("文件读取失败：{e}"))?;
  let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("docx 解包失败（文件可能已损坏）：{e}"))?;
  let entry = zip
    .by_name("word/document.xml")
    .map_err(|_| "docx 缺少 word/document.xml（非标准 Word 文档）".to_string())?;
  // 解压炸弹防御：声明大小不可信，按实读字节截流
  let mut xml_bytes = Vec::new();
  entry
    .take(MAX_XML_BYTES + 1)
    .read_to_end(&mut xml_bytes)
    .map_err(|e| format!("文档读取失败：{e}"))?;
  if xml_bytes.len() as u64 > MAX_XML_BYTES {
    return Err("文档过大，暂不支持预览".to_string());
  }
  let xml = String::from_utf8(xml_bytes).map_err(|_| "文档 XML 非 UTF-8（非标准 Word 文档）".to_string())?;
  parse_document(&xml)
}

/// 解析 document.xml（抽出来供单测）：流式状态机，w:p 段落 / w:r run / w:t 文本
fn parse_document(xml: &str) -> Result<DocxContent, String> {
  let mut reader = Reader::from_str(xml);
  let mut buf = Vec::new();
  let mut paragraphs: Vec<DocxParagraph> = Vec::new();
  let mut cur_p: Option<DocxParagraph> = None;
  let mut cur_run: Option<DocxRun> = None;
  let mut in_ppr = false; // w:pPr 内（pStyle / numPr 归属段落）
  let mut in_rpr = false; // w:rPr 内（b / i 归属 run）
  let mut total = 0usize; // 已提取文本字节数
  let mut truncated = false;

  loop {
    match reader.read_event_into(&mut buf) {
      Ok(Event::Start(e)) => match e.name().as_ref() {
        b"w:p" => cur_p = Some(DocxParagraph { heading: 0, list: false, runs: Vec::new() }),
        b"w:pPr" => in_ppr = true,
        b"w:pStyle" => set_heading(&mut cur_p, in_ppr, &e),
        b"w:numPr" => {
          if in_ppr {
            if let Some(p) = &mut cur_p {
              p.list = true;
            }
          }
        }
        b"w:r" => cur_run = Some(DocxRun { text: String::new(), bold: false, italic: false }),
        b"w:rPr" => in_rpr = true,
        b"w:b" => set_flag(&mut cur_run, in_rpr, &e, true),
        b"w:i" => set_flag(&mut cur_run, in_rpr, &e, false),
        _ => {}
      },
      // 自闭合元素（docx 常态：pStyle / b / i / br / tab 多为 Empty；空段落 = <w:p/>）
      Ok(Event::Empty(e)) => match e.name().as_ref() {
        b"w:p" => paragraphs.push(DocxParagraph { heading: 0, list: false, runs: Vec::new() }),
        b"w:pStyle" => set_heading(&mut cur_p, in_ppr, &e),
        b"w:b" => set_flag(&mut cur_run, in_rpr, &e, true),
        b"w:i" => set_flag(&mut cur_run, in_rpr, &e, false),
        b"w:br" => push_text(&mut cur_run, &mut total, &mut truncated, "\n"),
        b"w:tab" => push_text(&mut cur_run, &mut total, &mut truncated, "\t"),
        _ => {}
      },
      Ok(Event::Text(e)) => {
        if cur_run.is_some() {
          // unescape = 字符集解码 + XML 实体反转义一体（quick-xml 0.36 API）
          let text = e.unescape().map_err(|e| format!("文档 XML 文本解码失败：{e}"))?;
          push_text(&mut cur_run, &mut total, &mut truncated, &text);
        }
      }
      Ok(Event::End(e)) => match e.name().as_ref() {
        b"w:p" => {
          if let Some(p) = cur_p.take() {
            paragraphs.push(p);
          }
        }
        b"w:pPr" => in_ppr = false,
        b"w:r" => {
          // 空 run（无 w:t / w:br）不推——零文本段落保持 runs 空 = 空行
          if let Some(run) = cur_run.take() {
            if !run.text.is_empty() {
              if let Some(p) = &mut cur_p {
                p.runs.push(run);
              }
            }
          }
        }
        b"w:rPr" => in_rpr = false,
        _ => {}
      },
      Ok(Event::Eof) => break,
      Err(e) => return Err(format!("文档 XML 解析失败：{e}")),
      _ => {}
    }
    buf.clear();
  }
  Ok(DocxContent { paragraphs, truncated })
}

/// 文本追加 + 截断闸（截断后丢弃余下全部内容，解析循环照常走完——结构简单无早退必要）
fn push_text(cur_run: &mut Option<DocxRun>, total: &mut usize, truncated: &mut bool, text: &str) {
  if *truncated {
    return;
  }
  let Some(run) = cur_run else { return };
  if *total + text.len() > MAX_TEXT_BYTES {
    let mut end = MAX_TEXT_BYTES - *total;
    while end > 0 && !text.is_char_boundary(end) {
      end -= 1;
    }
    run.text.push_str(&text[..end]);
    *truncated = true;
    return;
  }
  *total += text.len();
  run.text.push_str(text);
}

/// pStyle w:val → 标题级：Heading1-9 / 纯数字 id（中文 Word 标题样式 id = "1"-"9"）/ Title（按 1 级），clamp 1-6
fn set_heading(cur_p: &mut Option<DocxParagraph>, in_ppr: bool, e: &quick_xml::events::BytesStart) {
  if !in_ppr {
    return;
  }
  let Some(val) = attr_val(e) else { return };
  let level = val
    .strip_prefix("Heading")
    .and_then(|d| d.parse::<u8>().ok())
    .or_else(|| val.parse::<u8>().ok())
    .or_else(|| (val == "Title").then_some(1));
  if let Some(p) = cur_p {
    if let Some(lv) = level {
      p.heading = lv.clamp(1, 6);
    }
  }
}

/// w:b / w:i 开关：缺省 val = 开；w:val = "false" / "0" / "off" / "none" = 关
fn set_flag(cur_run: &mut Option<DocxRun>, in_rpr: bool, e: &quick_xml::events::BytesStart, bold: bool) {
  if !in_rpr {
    return;
  }
  let on = !matches!(attr_val(e).as_deref(), Some("false") | Some("0") | Some("off") | Some("none"));
  if let Some(run) = cur_run {
    if bold {
      run.bold = on;
    } else {
      run.italic = on;
    }
  }
}

/// 取 w:val 属性值
fn attr_val(e: &quick_xml::events::BytesStart) -> Option<String> {
  for attr in e.attributes().flatten() {
    if attr.key.as_ref() == b"w:val" {
      return Some(String::from_utf8_lossy(&attr.value).into_owned());
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;

  /// 造最小 docx（zip 内一份 document.xml）
  fn make_docx(dir: &Path, name: &str, xml: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let f = fs::File::create(&path).unwrap();
    let mut zw = zip::ZipWriter::new(f);
    zw.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
    zw.write_all(xml.as_bytes()).unwrap();
    zw.finish().unwrap();
    path
  }

  const DOC_OPEN: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#;
  const DOC_CLOSE: &str = r#"</w:body></w:document>"#;

  #[test]
  fn ext_gate() {
    assert!(is_docx(Path::new("剧本.docx")));
    assert!(!is_docx(Path::new("剧本.doc")));
    assert!(!is_docx(Path::new("笔记.txt")));
  }

  #[test]
  fn parse_heading_styles() {
    // Heading2 / 中文 Word 纯数字 id "1" / Title
    let xml = format!(
      "{DOC_OPEN}
<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/></w:pPr><w:r><w:t>二级标题</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val=\"1\"/></w:pPr><w:r><w:t>中文标题一</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val=\"Title\"/></w:pPr><w:r><w:t>大标题</w:t></w:r></w:p>
<w:p><w:r><w:t>正文段落</w:t></w:r></w:p>
{DOC_CLOSE}"
    );
    let c = parse_document(&xml).unwrap();
    assert_eq!(c.paragraphs.len(), 4);
    assert_eq!(c.paragraphs[0].heading, 2);
    assert_eq!(c.paragraphs[0].runs[0].text, "二级标题");
    assert_eq!(c.paragraphs[1].heading, 1);
    assert_eq!(c.paragraphs[2].heading, 1);
    assert_eq!(c.paragraphs[3].heading, 0);
  }

  #[test]
  fn parse_bold_italic_and_off_state() {
    let xml = format!(
      "{DOC_OPEN}
<w:p>
<w:r><w:rPr><w:b/><w:i/></w:rPr><w:t>粗斜</w:t></w:r>
<w:r><w:rPr><w:b w:val=\"false\"/></w:rPr><w:t>关闭粗</w:t></w:r>
<w:r><w:t>普通</w:t></w:r>
</w:p>
{DOC_CLOSE}"
    );
    let c = parse_document(&xml).unwrap();
    let runs = &c.paragraphs[0].runs;
    assert_eq!(runs.len(), 3);
    assert!(runs[0].bold && runs[0].italic);
    assert!(!runs[1].bold && !runs[1].italic);
    assert!(!runs[2].bold && !runs[2].italic);
  }

  #[test]
  fn parse_list_break_tab_and_entities() {
    // numPr 列表标记；br → \n、tab → \t；实体 &amp; &lt; 解码
    let xml = format!(
      "{DOC_OPEN}
<w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"3\"/></w:numPr></w:pPr>
<w:r><w:t>甲 &amp; 乙 &lt;丙&gt;</w:t><w:br/><w:tab/><w:t>第二行</w:t></w:r></w:p>
{DOC_CLOSE}"
    );
    let c = parse_document(&xml).unwrap();
    let p = &c.paragraphs[0];
    assert!(p.list);
    assert_eq!(p.runs.len(), 1);
    assert_eq!(p.runs[0].text, "甲 & 乙 <丙>\n\t第二行");
  }

  #[test]
  fn parse_table_paragraphs_flattened() {
    // 表格单元格内容按段落平铺（无框线还原）
    let xml = format!(
      "{DOC_OPEN}
<w:p><w:r><w:t>前文</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>格一</w:t></w:r></w:p></w:tc>
<w:tc><w:p><w:r><w:t>格二</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
{DOC_CLOSE}"
    );
    let c = parse_document(&xml).unwrap();
    let texts: Vec<&str> = c.paragraphs.iter().map(|p| p.runs[0].text.as_str()).collect();
    assert_eq!(texts, ["前文", "格一", "格二"]);
  }

  #[test]
  fn parse_empty_paragraph_kept() {
    // 空段落（<w:p/> 与无文本 run）保留为空行
    let xml = format!(
      "{DOC_OPEN}
<w:p><w:r><w:t>首段</w:t></w:r></w:p>
<w:p/>
<w:p><w:r><w:rPr><w:b/></w:rPr></w:r></w:p>
<w:p><w:r><w:t>末段</w:t></w:r></w:p>
{DOC_CLOSE}"
    );
    let c = parse_document(&xml).unwrap();
    assert_eq!(c.paragraphs.len(), 4);
    assert!(c.paragraphs[1].runs.is_empty());
    assert!(c.paragraphs[2].runs.is_empty());
  }

  #[test]
  fn parse_truncates_oversize_text() {
    let big = "字".repeat(MAX_TEXT_BYTES + 1024);
    let xml = format!("{DOC_OPEN}<w:p><w:r><w:t>{big}</w:t></w:r></w:p>{DOC_CLOSE}");
    let c = parse_document(&xml).unwrap();
    assert!(c.truncated);
    let text = &c.paragraphs[0].runs[0].text;
    assert!(text.len() <= MAX_TEXT_BYTES);
    // 截断点紧贴上限（"字" 3 字节，char 边界回退 ≤2 字节）
    assert!(text.len() > MAX_TEXT_BYTES - 3);
  }

  #[test]
  fn read_docx_bytes_roundtrip_and_gates() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_docx(
      dir.path(),
      "反馈.docx",
      &format!("{DOC_OPEN}<w:p><w:r><w:t>截图配字</w:t></w:r></w:p>{DOC_CLOSE}"),
    );
    // 字节级一致（mammoth 链路 = 原样给字节，不改一字）
    assert_eq!(read_docx_bytes(&path).unwrap(), fs::read(&path).unwrap());
    // 非 docx 格式门（扩展名先于存在性）
    let err = read_docx_bytes(dir.path().join("笔记.txt").as_path()).unwrap_err();
    assert!(err.contains("不是 docx 资源"), "{err}");
    // 不存在 → 读取失败
    let err = read_docx_bytes(dir.path().join("不存在.docx").as_path()).unwrap_err();
    assert!(err.contains("读取失败"), "{err}");
  }

  #[test]
  fn read_docx_roundtrip_and_gates() {
    let dir = tempfile::tempdir().unwrap();
    // 真实 zip 回路
    let path = make_docx(
      dir.path(),
      "剧本.docx",
      &format!("{DOC_OPEN}<w:p><w:r><w:t>第一幕 🎬</w:t></w:r></w:p>{DOC_CLOSE}"),
    );
    let c = read_docx(&path).unwrap();
    assert!(!c.truncated);
    assert_eq!(c.paragraphs[0].runs[0].text, "第一幕 🎬");

    // 缺 document.xml 的 zip → 中文报错
    let bad = dir.path().join("坏.docx");
    let f = fs::File::create(&bad).unwrap();
    let mut zw = zip::ZipWriter::new(f);
    zw.start_file("其他.xml", zip::write::SimpleFileOptions::default()).unwrap();
    zw.write_all(b"<x/>").unwrap();
    zw.finish().unwrap();
    let err = read_docx(&bad).unwrap_err();
    assert!(err.contains("document.xml"), "{err}");

    // 非 zip 文件 → 解包报错
    let not_zip = dir.path().join("假.docx");
    fs::write(&not_zip, b"not a zip").unwrap();
    let err = read_docx(&not_zip).unwrap_err();
    assert!(err.contains("解包失败"), "{err}");
  }
}
