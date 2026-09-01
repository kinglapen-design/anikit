//! 发送至 Animate 舞台（PRD 07-04，8.3）：fla（XFL zip）→ Flash 11 Picture 剪贴板载荷转换。
//!
//! 剪贴板格式（spike/clipboard-probe 实证）：内容 = XFL 包（与 fla 同构），其 DOMDocument 的
//! timelines = 单场景、每层一帧、元素带 selected="true"（matrix 即原位坐标）。
//! 转换口径：folders / media / symbols 等块原样透传；timelines 重写为第一个 DOMTimeline
//! 每层首个 DOMFrame（收单帧、去 duration 等属性，首帧 index≠0 即前导空白帧则置空帧），
//! elements 直接子元素注入 selected="true"；其余 zip entry（LIBRARY / bin / PublishSettings 等）
//! 原字节原序复制（依赖闭包裁剪优化记 09 待办）。
//! CEP 导出物（保持坐标档）主时间轴 = 一层一帧一实例，转换后原位粘贴坐标准确（PRD 联动）。

use std::io::{Cursor, Read, Write};
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// fla → 剪贴板载荷（Flash 11 Picture 内容字节）。失败 = 调用方走兜底（pasteFla）。
pub fn build_clip_payload(fla_path: &Path) -> Result<Vec<u8>, String> {
  let bytes = std::fs::read(fla_path).map_err(|e| format!("读取 fla 失败：{e}"))?;
  build_from_bytes(&bytes)
}

/// zip 级转换：DOMDocument.xml 重写，其余 entry 原样复制（可测核心）
fn build_from_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
  let mut src = ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("fla 不是有效的 XFL 包：{e}"))?;
  let mut entries: Vec<(String, Vec<u8>, bool)> = Vec::new(); // (名, 内容, 是否目录)
  let mut has_mimetype = false;
  let mut dom: Option<Vec<u8>> = None;
  for i in 0..src.len() {
    let mut f = src.by_index(i).map_err(|e| format!("读取 XFL 条目失败：{e}"))?;
    let name = f.name().to_string();
    if name.ends_with('/') {
      entries.push((name, Vec::new(), true)); // 空目录保留（spike 样本含 META-INF/ 等空目录）
      continue;
    }
    let mut buf = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut buf).map_err(|e| format!("读取 XFL 条目失败：{e}"))?;
    match name.as_str() {
      "mimetype" => has_mimetype = true,
      "DOMDocument.xml" => {
        dom = Some(transform_dom_document(&buf)?);
        continue; // 重写内容最后单独写
      }
      _ => {}
    }
    entries.push((name, buf, false));
  }
  if !has_mimetype {
    return Err("fla 缺 mimetype 条目".to_string());
  }
  let dom = dom.ok_or_else(|| "fla 缺 DOMDocument.xml".to_string())?;

  let mut out = Vec::new();
  {
    let mut w = ZipWriter::new(Cursor::new(&mut out));
    let opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, buf, is_dir) in entries {
      if is_dir {
        w.add_directory(name, opt).map_err(|e| format!("打包失败：{e}"))?;
      } else {
        w.start_file(name, opt).map_err(|e| format!("打包失败：{e}"))?;
        w.write_all(&buf).map_err(|e| format!("打包失败：{e}"))?;
      }
    }
    w.start_file("DOMDocument.xml", opt).map_err(|e| format!("打包失败：{e}"))?;
    w.write_all(&dom).map_err(|e| format!("打包失败：{e}"))?;
    w.finish().map_err(|e| format!("打包失败：{e}"))?;
  }
  Ok(out)
}

/// timelines 改写状态机上下文
struct Ctx {
  depth: usize,        // 当前标签深度（Start +1 / End -1）
  skip_depth: usize,   // >0 = 正丢弃的子树（多场景 / 层内次帧），Start 递归 +1、End -1
  in_timeline: bool,   // 第一个 DOMTimeline 内
  timeline_seen: bool, // 已处理过一个 DOMTimeline（后续场景丢弃）
  layer_frame_done: bool, // 当前层首帧已输出
  elements_depth: usize, // 0 = 不在 elements 内；否则 = elements 标签的深度
  has_element: bool,   // 是否有元素被写入（全程无 → Err 走兜底）
}

/// DOMDocument 重写：timelines 收「第一帧 + selected」，其余块原样透传（事件流保属性序）
fn transform_dom_document(src: &[u8]) -> Result<Vec<u8>, String> {
  let text = std::str::from_utf8(src).map_err(|_| "DOMDocument.xml 非 UTF-8".to_string())?;
  let mut reader = Reader::from_str(text);
  let mut writer = Writer::new(Vec::new());
  let mut c = Ctx { depth: 0, skip_depth: 0, in_timeline: false, timeline_seen: false, layer_frame_done: false, elements_depth: 0, has_element: false };

  loop {
    match reader.read_event().map_err(|e| format!("DOMDocument 解析失败：{e}"))? {
      Event::Eof => break,
      Event::Start(e) => {
        c.depth += 1;
        if c.skip_depth > 0 {
          c.skip_depth += 1;
          continue;
        }
        if on_start(&mut writer, &mut c, &e)? {
          writer.write_event(Event::Start(e.clone())).map_err(|e| e.to_string())?;
        }
      }
      Event::Empty(e) => {
        if c.skip_depth > 0 {
          continue;
        }
        if on_empty(&mut writer, &mut c, &e)? {
          writer.write_event(Event::Empty(e.clone())).map_err(|e| e.to_string())?;
        }
      }
      Event::End(e) => {
        if c.skip_depth > 0 {
          c.skip_depth -= 1;
          c.depth -= 1;
          continue;
        }
        if c.elements_depth == c.depth && e.name().as_ref() == b"elements" {
          c.elements_depth = 0;
        }
        if c.in_timeline && e.name().as_ref() == b"DOMTimeline" {
          c.in_timeline = false;
        }
        writer.write_event(Event::End(e.clone())).map_err(|e| e.to_string())?;
        c.depth -= 1;
      }
      ev => {
        if c.skip_depth == 0 {
          writer.write_event(ev.into_owned()).map_err(|e| e.to_string())?;
        }
      }
    }
  }
  if !c.has_element {
    return Err("fla 主时间轴第一帧无内容".to_string());
  }
  Ok(writer.into_inner())
}

/// Start 事件：返回 false = 已自行处理（调用方不再透传）
fn on_start(writer: &mut Writer<Vec<u8>>, c: &mut Ctx, e: &BytesStart) -> Result<bool, String> {
  let name = e.name().as_ref().to_vec();
  if !c.in_timeline {
    if name == b"DOMTimeline" {
      if c.timeline_seen {
        c.skip_depth = 1; // 多场景：仅保留第一场景
        return Ok(false);
      }
      c.timeline_seen = true;
      c.in_timeline = true;
    }
    return Ok(true); // 根/三块等非 timelines 内容原样透传
  }
  match name.as_slice() {
    b"DOMLayer" => {
      c.layer_frame_done = false;
      Ok(true)
    }
    b"DOMFrame" => {
      if c.layer_frame_done {
        c.skip_depth = 1; // 层内次帧起丢弃
        return Ok(false);
      }
      c.layer_frame_done = true;
      // 首帧 index≠0（前导空白帧）= 第一帧无内容：输出空帧占位，原帧子树丢弃（skip_depth 对应其 End）
      if attr_value(e, b"index").as_deref() != Some("0") {
        write_empty_frame(writer)?;
        c.skip_depth = 1;
        return Ok(false);
      }
      // 收单帧：index="0" 之外属性一律不带（duration/tween 等对单帧无意义）
      let mut bs = BytesStart::new("DOMFrame");
      bs.push_attribute(("index", "0"));
      writer.write_event(Event::Start(bs)).map_err(|e| e.to_string())?;
      Ok(false)
    }
    b"elements" => {
      c.elements_depth = c.depth;
      Ok(true)
    }
    _ if c.elements_depth > 0 && c.depth == c.elements_depth + 1 && name.starts_with(b"DOM") => {
      write_selected(writer, e, false)?;
      c.has_element = true;
      Ok(false)
    }
    _ => Ok(true),
  }
}

/// Empty 事件（自闭合标签）：返回 false = 已自行处理
fn on_empty(writer: &mut Writer<Vec<u8>>, c: &mut Ctx, e: &BytesStart) -> Result<bool, String> {
  let name = e.name().as_ref().to_vec();
  if !c.in_timeline {
    return Ok(true);
  }
  if name == b"DOMFrame" {
    if c.layer_frame_done {
      return Ok(false); // 次帧空帧丢弃
    }
    c.layer_frame_done = true;
    write_empty_frame(writer)?; // 自闭合空帧 → 统一收 index="0" 空帧
    return Ok(false);
  }
  if c.elements_depth > 0 && c.depth == c.elements_depth + 1 && name.starts_with(b"DOM") {
    write_selected(writer, e, true)?;
    c.has_element = true;
    return Ok(false);
  }
  Ok(true)
}

/// 输出 `<DOMFrame index="0"/>` 空帧（前导空白帧占位 / 自闭合首帧归一）
fn write_empty_frame(writer: &mut Writer<Vec<u8>>) -> Result<(), String> {
  let mut bs = BytesStart::new("DOMFrame");
  bs.push_attribute(("index", "0"));
  writer.write_event(Event::Empty(bs)).map_err(|e| e.to_string())
}

/// elements 直接子元素开标签注入 selected="true"（已有 selected 属性不重复加——源文档选中态沿用）；
/// self_close = true 写 Empty（原事件为自闭合，写 Start 会破坏配对）
fn write_selected(writer: &mut Writer<Vec<u8>>, e: &BytesStart, self_close: bool) -> Result<(), String> {
  let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
  let mut bs = BytesStart::new(&name);
  let mut has_selected = false;
  for attr in e.attributes().flatten() {
    if attr.key.as_ref() == b"selected" {
      has_selected = true;
    }
    let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
    let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
    bs.push_attribute((key.as_str(), value.as_str()));
  }
  if !has_selected {
    bs.push_attribute(("selected", "true"));
  }
  if self_close {
    writer.write_event(Event::Empty(bs)).map_err(|e| e.to_string())
  } else {
    writer.write_event(Event::Start(bs)).map_err(|e| e.to_string())
  }
}

fn attr_value(e: &BytesStart, key: &[u8]) -> Option<String> {
  e.attributes().flatten().find(|a| a.key.as_ref() == key).map(|a| String::from_utf8_lossy(a.value.as_ref()).into_owned())
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 内存构造 fla zip（条目序 = mimetype / DOMDocument.xml / LIBRARY/x.xml / bin/x.dat）
  fn make_fla(dom: &str) -> Vec<u8> {
    let mut out = Vec::new();
    {
      let mut w = ZipWriter::new(Cursor::new(&mut out));
      let opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
      for (name, content) in [
        ("mimetype", "application/vnd.adobe.xfl"),
        ("DOMDocument.xml", dom),
        ("LIBRARY/元件1.xml", "<DOMSymbolItem name=\"元件1\"/>"),
        ("bin/M 1 123.dat", "RAWDATA"),
      ] {
        w.start_file(name, opt).unwrap();
        w.write_all(content.as_bytes()).unwrap();
      }
      w.finish().unwrap();
    }
    out
  }

  fn payload_dom(payload: &[u8]) -> String {
    let mut z = ZipArchive::new(Cursor::new(payload)).unwrap();
    let mut s = String::new();
    z.by_name("DOMDocument.xml").unwrap().read_to_string(&mut s).unwrap();
    s
  }

  const DOM_ONE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<DOMDocument width="1920" height="1080" frameRate="25" currentTimeline="1"><folders><DOMFolderItem name="素" itemID="x1"/></folders><media><DOMBitmapItem name="素/位图1" itemID="x2" href="素/位图1.png" bitmapDataHRef="M 1 123.dat"/></media><symbols><Include href="元件1.xml" itemID="x3"/></symbols><timelines><DOMTimeline name="Scene 1"><layers><DOMLayer name="Layer 1" color="#000000"><frames><DOMFrame index="0" duration="5"><elements><DOMSymbolInstance libraryItemName="元件1" symbolType="graphic" firstFrame="161" loop="loop"><matrix><Matrix a="1.5" d="1.5" tx="902.95" ty="498.6"/></matrix></DOMSymbolInstance></elements></DOMFrame><DOMFrame index="5"><elements/></DOMFrame></frames></DOMLayer></layers></DOMTimeline></timelines></DOMDocument>"##;

  #[test]
  fn unit_symbol_instance_end_to_end() {
    let payload = build_from_bytes(&make_fla(DOM_ONE)).unwrap();
    let mut z = ZipArchive::new(Cursor::new(&payload)).unwrap();
    for name in ["mimetype", "LIBRARY/元件1.xml", "bin/M 1 123.dat", "DOMDocument.xml"] {
      assert!(z.by_name(name).is_ok(), "缺条目 {name}");
    }
    let dom = payload_dom(&payload);
    // folders / media / symbols 三块原样透传
    assert!(dom.contains(r#"<DOMFolderItem name="素" itemID="x1"/>"#));
    assert!(dom.contains(r#"bitmapDataHRef="M 1 123.dat""#));
    assert!(dom.contains(r#"<Include href="元件1.xml" itemID="x3"/>"#));
    // selected 注入 + 原位 matrix 与实例属性保留
    assert!(dom.contains(r#"selected="true""#), "{dom}");
    assert!(dom.contains(r#"symbolType="graphic""#) && dom.contains(r#"firstFrame="161""#));
    assert!(dom.contains(r#"tx="902.95""#));
    // 收单帧：duration 去除、次帧丢弃
    assert!(!dom.contains("duration="), "duration 应去除：{dom}");
    assert!(!dom.contains(r#"index="5""#), "次帧应丢弃：{dom}");
  }

  #[test]
  fn multi_layer_first_frame_only() {
    let dom = r#"<DOMDocument width="100" height="100"><timelines><DOMTimeline name="Scene 1"><layers><DOMLayer name="a"><frames><DOMFrame index="0" duration="3"><elements><DOMShape><fills/></DOMShape></elements></DOMFrame><DOMFrame index="3"><elements><DOMShape/></elements></DOMFrame></frames></DOMLayer><DOMLayer name="b"><frames><DOMFrame index="5"><elements><DOMSymbolInstance libraryItemName="x"/></elements></DOMFrame></frames></DOMLayer></layers></DOMTimeline><DOMTimeline name="Scene 2"><layers><DOMLayer name="c"><frames><DOMFrame index="0"><elements><DOMShape/></elements></DOMFrame></frames></DOMLayer></layers></DOMTimeline></timelines></DOMDocument>"#;
    let out = transform_dom_document(dom.as_bytes()).unwrap();
    let s = String::from_utf8(out).unwrap();
    // 第一层首帧收单帧、DOMShape 加 selected、其内部 <fills/> 透传且不加 selected
    assert!(s.contains(r#"<DOMShape selected="true">"#), "{s}");
    assert!(s.contains("<fills/>"), "{s}");
    assert!(!s.contains(r#"<fills selected="true"/>"#), "elements 深层元素不应注入：{s}");
    // 每层仅一帧：第一层 1 + 第二层空帧 1
    assert_eq!(s.matches("<DOMFrame").count(), 2, "{s}");
    assert!(!s.contains("duration="), "{s}");
    assert!(s.contains(r#"<DOMFrame index="0"/>"#), "前导空白帧收空帧：{s}");
    // 多场景仅留第一场景
    assert!(!s.contains("Scene 2"), "{s}");
    // 第二层 index=5 的实例应随丢弃帧一并消失
    assert!(!s.contains(r#"libraryItemName="x""#), "{s}");
  }

  #[test]
  fn empty_first_frame_is_error() {
    let dom = r#"<DOMDocument width="100" height="100"><timelines><DOMTimeline name="Scene 1"><layers><DOMLayer name="a"><frames><DOMFrame index="0"><elements/></DOMFrame></frames></DOMLayer></layers></DOMTimeline></timelines></DOMDocument>"#;
    let err = transform_dom_document(dom.as_bytes()).unwrap_err();
    assert!(err.contains("第一帧无内容"), "{err}");
    // 无 timelines 同报错
    let err = transform_dom_document(r#"<DOMDocument width="1" height="1"/>"#.as_bytes()).unwrap_err();
    assert!(err.contains("第一帧无内容"), "{err}");
  }

  #[test]
  fn missing_entries_are_errors() {
    let opt = SimpleFileOptions::default();
    // 缺 mimetype
    let mut out = Vec::new();
    {
      let mut w = ZipWriter::new(Cursor::new(&mut out));
      w.start_file("DOMDocument.xml", opt).unwrap();
      w.write_all(DOM_ONE.as_bytes()).unwrap();
      w.finish().unwrap();
    }
    assert!(build_from_bytes(&out).unwrap_err().contains("mimetype"));
    // 缺 DOMDocument
    let mut out2 = Vec::new();
    {
      let mut w = ZipWriter::new(Cursor::new(&mut out2));
      w.start_file("mimetype", opt).unwrap();
      w.write_all(b"application/vnd.adobe.xfl").unwrap();
      w.finish().unwrap();
    }
    assert!(build_from_bytes(&out2).unwrap_err().contains("DOMDocument"));
    // 非 zip
    assert!(build_from_bytes(b"not a zip").is_err());
  }
}
