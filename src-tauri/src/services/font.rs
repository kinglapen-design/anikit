//! 字体服务（5.6）：ttf / otf / ttc 元数据解析（ttf-parser）+ 缩略图样张渲染（fontdue 光栅化，
//! 纯 Rust 无系统依赖）+ 系统级安装 / 卸载（Windows 用户级：%LOCALAPPDATA%\Microsoft\Windows\Fonts
//! + HKCU 注册表 Fonts 项 + WM_FONTCHANGE 广播——其他软件立即可见。**2026-08-23 用户痛点定案**：
//! Eagle 装字体其他软件识别不到（仅注册进 Eagle 自有体系），AniKit 必须装到系统级）。
//! 元数据字段口径对齐 Eagle「其它」页：名称（PostScript 名）/ 全名 / 系列 / 样式 / 版本 / 字数。
//! 样张字样口径（同日用户供 Eagle 参考图）：cmap 含中文（'永' U+6C38）→ 中文四行
//! 「永和九年 / 歲在癸丑 / AaBbCcDd / 01234567」；否则拉丁三行「ABCDEFG / abcdefg / 0123456」。

use std::fs;
use std::path::Path;

use image::RgbaImage;

/// 字体格式（缩略图适配器 / 预览 kind 分流共用判定）
const FONT_EXTS: &[&str] = &["ttf", "otf", "ttc"];

pub fn is_font(path: &Path) -> bool {
  let ext = path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
  FONT_EXTS.contains(&ext.as_str())
}

/// 样张画布尺寸（缩略图适配器 probe / decode 共用；640×480 = 4:3，四行中文样字排版不挤；
/// 生成后由 thumbnail 统一缩到短边 320——先渲大再缩等效超采样，边缘更干净）
pub const SPECIMEN_W: u32 = 640;
pub const SPECIMEN_H: u32 = 480;

/// 样张字色（深色主题常态文字观感）：RGB = --text-normal 实色部（247,255,252），
/// alpha 打 7 折对齐常态色 0.6941 档位（位图叠卡片 C2 底后与 UI 文字观感一致）
const SPECIMEN_RGB: [u8; 3] = [247, 255, 252];
const SPECIMEN_ALPHA_NUM: u32 = 7;
const SPECIMEN_ALPHA_DEN: u32 = 10;

/// 字体元数据（预览「其它」页；name 表英文（en-US）优先、回退第一条）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontMeta {
  /// 名称（name_id 6 PostScript 名，如 HisenseAlfabet-RegularItalic）
  pub name: String,
  /// 全名（name_id 4）
  pub full_name: String,
  /// 系列（name_id 16 typographic family，回退 name_id 1）
  pub family: String,
  /// 样式（name_id 17 typographic subfamily，回退 name_id 2）
  pub style: String,
  /// 版本（name_id 5）
  pub version: String,
  /// 字数（maxp num_glyphs）
  pub glyph_count: u32,
}

/// 解析字体元数据；非字体 / 解析失败返回 None（能力探测语义，由缩略图管线回退格式图标）
pub fn probe_font(path: &Path) -> Option<FontMeta> {
  if !is_font(path) {
    return None;
  }
  let data = fs::read(path).ok()?;
  let face = ttf_parser::Face::parse(&data, 0).ok()?;
  let pick = |preferred: &[u16]| -> String {
    for id in preferred {
      if let Some(s) = name_string(&face, *id) {
        return s;
      }
    }
    String::new()
  };
  Some(FontMeta {
    name: pick(&[6]),
    full_name: pick(&[4]),
    family: pick(&[16, 1]),
    style: pick(&[17, 2]),
    version: pick(&[5]),
    glyph_count: u32::from(face.number_of_glyphs()),
  })
}

/// name 表取字符串：en-US（language_id 0x409）优先，无则取第一条可解码项
fn name_string(face: &ttf_parser::Face, name_id: u16) -> Option<String> {
  let mut fallback: Option<String> = None;
  for name in face.names() {
    if name.name_id != name_id {
      continue;
    }
    let Some(s) = name.to_string() else { continue };
    if name.language_id == 0x409 {
      return Some(s);
    }
    if fallback.is_none() {
      fallback = Some(s);
    }
  }
  fallback
}

/// cmap 是否覆盖中文（'永' U+6C38；字样组分流用）
fn has_cjk(face: &ttf_parser::Face) -> bool {
  let Some(cmap) = face.tables().cmap else { return false };
  cmap.subtables.into_iter().filter(|s| s.is_unicode()).any(|s| s.glyph_index(0x6C38).is_some())
}

/// 渲染字体样张（缩略图适配器 decode）：字样按 cmap 中文覆盖分流，左对齐逐行排版，
/// 缺字形字符跳过不占位；画布透明底，字色 SPECIMEN_RGB × coverage（7 折 alpha）。
pub fn render_specimen(path: &Path, w: u32, h: u32) -> Option<RgbaImage> {
  if !is_font(path) {
    return None;
  }
  let data = fs::read(path).ok()?;
  // 中文判定与光栅化分属两库（ttf-parser cmap 查得准、fontdue 渲染）——同一字节各解一遍，成本可忽略
  let cjk = ttf_parser::Face::parse(&data, 0).map(|f| has_cjk(&f)).unwrap_or(false);
  let font = fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()).ok()?;
  const CJK_LINES: &[&str] = &["永和九年", "歲在癸丑", "AaBbCcDd", "01234567"];
  const LATIN_LINES: &[&str] = &["ABCDEFG", "abcdefg", "0123456"];
  let lines = if cjk { CJK_LINES } else { LATIN_LINES };
  Some(render_lines(&font, lines, w, h))
}

/// 逐行左对齐渲染：行高均分可用高、字号按行高 0.62 起算，各行按内容宽等比收敛（全组统一 px 不超宽）
fn render_lines(font: &fontdue::Font, lines: &[&str], w: u32, h: u32) -> RgbaImage {
  let mut img = RgbaImage::new(w, h);
  let margin_x = (w as f32 * 0.07).round();
  let margin_y = (h as f32 * 0.08).round();
  let band = ((h as f32) - margin_y * 2.0) / lines.len() as f32;
  let max_text_w = (w as f32) - margin_x * 2.0;
  // 统一字号：行高起算后，逐行量宽取收敛系数最小值（最宽行恰好装下）
  let mut px = band * 0.62;
  for line in lines {
    let tw = line_width(font, line, px);
    if tw > max_text_w && tw > 0.0 {
      px = (px * (max_text_w / tw)).floor().max(8.0);
    }
  }
  let lm = font.horizontal_line_metrics(px);
  for (i, line) in lines.iter().enumerate() {
    let top = margin_y + band * i as f32;
    // baseline 在行带内垂直居中（ascent/descent 取自字体，缺省按 0.8/0.2 估）
    let (ascent, descent) = lm.map(|m| (m.ascent, m.descent)).unwrap_or((px * 0.8, -px * 0.2));
    let baseline = top + (band - (ascent - descent)) / 2.0 + ascent;
    let mut x = margin_x;
    for ch in line.chars() {
      if font.lookup_glyph_index(ch) == 0 {
        continue; // 缺字形跳过不占位
      }
      let (m, bmp) = font.rasterize(ch, px);
      let dst_x = (x + m.xmin as f32).round() as i32;
      let dst_y = (baseline - m.ymin as f32 - m.height as f32).round() as i32;
      blit(&mut img, &bmp, m.width, m.height, dst_x, dst_y);
      x += m.advance_width;
    }
  }
  img
}

/// 行文本宽（advance 累加；缺字形不计）
fn line_width(font: &fontdue::Font, line: &str, px: f32) -> f32 {
  line
    .chars()
    .filter(|ch| font.lookup_glyph_index(*ch) != 0)
    .map(|ch| font.metrics(ch, px).advance_width)
    .sum()
}

/// coverage 位图 blit 进 RGBA 画布（越界裁剪；alpha = coverage × 7/10）
fn blit(img: &mut RgbaImage, bmp: &[u8], bw: usize, bh: usize, dx: i32, dy: i32) {
  for gy in 0..bh {
    for gx in 0..bw {
      let x = dx + gx as i32;
      let y = dy + gy as i32;
      if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        continue;
      }
      let cov = bmp[gy * bw + gx] as u32;
      let a = (cov * SPECIMEN_ALPHA_NUM / SPECIMEN_ALPHA_DEN) as u8;
      img.put_pixel(x as u32, y as u32, image::Rgba([SPECIMEN_RGB[0], SPECIMEN_RGB[1], SPECIMEN_RGB[2], a]));
    }
  }
}

// ---------- Windows 系统级安装 / 卸载（用户级，免管理员） ----------
// 口径（2026-08-23 用户痛点定案）：字体文件复制进 %LOCALAPPDATA%\Microsoft\Windows\Fonts（Win10+
// 用户级字体目录，系统原生机制）→ HKCU\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts 写
// "{全名} (TrueType)" = 绝对路径（ttc 每个 face 一项）→ AddFontResourceW 会话内立即可用 →
// 广播 WM_FONTCHANGE（运行中的软件收到后刷新字体列表）。卸载 = 反向。

#[cfg(windows)]
mod win_install {
  use std::fs;
  use std::path::{Path, PathBuf};

  use windows::core::{HSTRING, PCWSTR};
  use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
  use windows::Win32::Graphics::Gdi::{AddFontResourceW, RemoveFontResourceW};
  use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPEN_CREATE_OPTIONS, REG_SAM_FLAGS,
    REG_SZ,
  };
  use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG};

  const FONTS_SUBKEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";
  const WM_FONTCHANGE: u32 = 0x001D;

  /// 用户级字体目录（%LOCALAPPDATA%\Microsoft\Windows\Fonts）
  fn user_fonts_dir() -> Result<PathBuf, String> {
    let Some(base) = dirs::data_local_dir() else {
      return Err("无法定位用户数据目录".to_string());
    };
    Ok(base.join("Microsoft").join("Windows").join("Fonts"))
  }

  /// 注册表值名（Windows 惯例：ttf/otf 同挂 (TrueType) 后缀）
  pub fn reg_value_name(full_name: &str) -> String {
    format!("{full_name} (TrueType)")
  }

  /// 是否已安装（HKCU Fonts 项查到值名即视为已装；系统级 HKLM 不查——
  /// Win10 1809+ 双击字体"安装"本就写 HKCU，覆盖常态；系统级同名字体与 HKCU 共存无害）
  pub fn is_installed(full_names: &[String]) -> bool {
    full_names.iter().any(|n| reg_value_exists(&reg_value_name(n)))
  }

  fn reg_value_exists(value_name: &str) -> bool {
    let Some(hkey) = open_fonts_key(KEY_QUERY_VALUE) else { return false };
    let name = HSTRING::from(value_name);
    let mut size = 0u32;
    let r = unsafe { RegQueryValueExW(hkey, PCWSTR(name.as_ptr()), None, None, None, Some(&mut size)) };
    unsafe { RegCloseKey(hkey) };
    r.is_ok()
  }

  /// 打开 HKCU Fonts 键（只读 / 读写按 sam）
  fn open_fonts_key(sam: REG_SAM_FLAGS) -> Option<HKEY> {
    let mut hkey = HKEY::default();
    let subkey = HSTRING::from(FONTS_SUBKEY);
    let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), None, sam, &mut hkey) };
    r.is_ok().then_some(hkey)
  }

  /// 安装：复制进用户字体目录（撞名且字节不同加 -anikit 后缀避让）→ 逐 full_name 写注册表
  /// → AddFontResourceW → 广播。返回落盘路径（toast / 日志用）。
  pub fn install(origin: &Path, full_names: &[String]) -> Result<PathBuf, String> {
    let dir = user_fonts_dir()?;
    fs::create_dir_all(&dir).map_err(|e| format!("字体目录创建失败：{e}"))?;
    let target = unique_target(&dir, origin)?;
    fs::copy(origin, &target).map_err(|e| format!("字体文件复制失败：{e}"))?;
    // 注册表写入失败回滚文件（不留无注册的孤儿字体文件）
    if let Err(e) = write_registry(&target, full_names) {
      let _ = fs::remove_file(&target);
      return Err(e);
    }
    let wide = HSTRING::from(target.as_os_str());
    let added = unsafe { AddFontResourceW(PCWSTR(wide.as_ptr())) };
    if added == 0 {
      log::warn!("AddFontResourceW 返回 0（{}）：注册表已写，重启后生效", target.display());
    }
    broadcast();
    Ok(target)
  }

  /// 卸载：RemoveFontResourceW → 删注册表值 → 删文件（占用报错不删）→ 广播
  pub fn uninstall(full_names: &[String]) -> Result<(), String> {
    let dir = user_fonts_dir()?;
    // 逐值名收集注册表指向的路径（可能经 -anikit 后缀落盘，以注册表值数据为准）
    let mut paths = Vec::new();
    for n in full_names {
      let name = reg_value_name(n);
      if let Some(p) = read_registry_path(&name) {
        paths.push((name, p));
      }
    }
    if paths.is_empty() {
      return Err("该字体未安装到系统".to_string());
    }
    for (value_name, path) in &paths {
      let wide = HSTRING::from(path.as_os_str());
      unsafe { RemoveFontResourceW(PCWSTR(wide.as_ptr())) };
      delete_registry_value(value_name)?;
      // 文件占用（某软件加载中）：删不掉不硬删——注册表已清，报占用由用户关软件后手动删
      if path.is_file() && path.starts_with(&dir) {
        fs::remove_file(path).map_err(|_| {
          "已注销字体，但文件正被其他软件占用未能删除；请关闭相关软件后重试卸载以清理文件".to_string()
        })?;
      }
    }
    broadcast();
    Ok(())
  }

  /// 注册表值数据（字体文件绝对路径）；未安装返回 None
  fn read_registry_path(value_name: &str) -> Option<PathBuf> {
    let hkey = open_fonts_key(KEY_QUERY_VALUE)?;
    let name = HSTRING::from(value_name);
    let mut size = 0u32;
    let r = unsafe { RegQueryValueExW(hkey, PCWSTR(name.as_ptr()), None, None, None, Some(&mut size)) };
    if !r.is_ok() || size < 4 {
      unsafe { RegCloseKey(hkey) };
      return None;
    }
    let mut buf = vec![0u8; size as usize];
    let r = unsafe {
      RegQueryValueExW(hkey, PCWSTR(name.as_ptr()), None, None, Some(buf.as_mut_ptr()), Some(&mut size))
    };
    unsafe { RegCloseKey(hkey) };
    if !r.is_ok() {
      return None;
    }
    let wide: &[u16] = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u16>(), (size as usize) / 2) };
    let s = String::from_utf16_lossy(wide).trim_end_matches('\0').to_string();
    (!s.is_empty()).then_some(PathBuf::from(s))
  }

  fn write_registry(target: &Path, full_names: &[String]) -> Result<(), String> {
    let mut hkey = HKEY::default();
    let subkey = HSTRING::from(FONTS_SUBKEY);
    let r = unsafe {
      RegCreateKeyExW(
        HKEY_CURRENT_USER,
        PCWSTR(subkey.as_ptr()),
        None,
        PCWSTR::null(),
        REG_OPEN_CREATE_OPTIONS(0),
        KEY_SET_VALUE,
        None,
        &mut hkey,
        None,
      )
    };
    if !r.is_ok() {
      return Err(format!("字体注册表打开失败：{}", r.0));
    }
    let path_str = target.to_string_lossy();
    // REG_SZ 数据 = UTF-16 含结尾 NUL 的字节串
    let mut data: Vec<u8> = Vec::with_capacity((path_str.len() + 1) * 2);
    for u in path_str.encode_utf16().chain(std::iter::once(0u16)) {
      data.extend_from_slice(&u.to_le_bytes());
    }
    let mut result = Ok(());
    for n in full_names {
      let name = HSTRING::from(reg_value_name(n));
      let r = unsafe { RegSetValueExW(hkey, PCWSTR(name.as_ptr()), None, REG_SZ, Some(&data)) };
      if !r.is_ok() {
        result = Err(format!("字体注册失败（{n}）：{}", r.0));
        break;
      }
    }
    unsafe { windows::Win32::System::Registry::RegCloseKey(hkey) };
    result
  }

  fn delete_registry_value(value_name: &str) -> Result<(), String> {
    let mut hkey = HKEY::default();
    let subkey = HSTRING::from(FONTS_SUBKEY);
    let r = unsafe {
      RegCreateKeyExW(
        HKEY_CURRENT_USER,
        PCWSTR(subkey.as_ptr()),
        None,
        PCWSTR::null(),
        REG_OPEN_CREATE_OPTIONS(0),
        KEY_SET_VALUE,
        None,
        &mut hkey,
        None,
      )
    };
    if !r.is_ok() {
      return Err(format!("字体注册表打开失败：{}", r.0));
    }
    let name = HSTRING::from(value_name);
    let r = unsafe { RegDeleteValueW(hkey, PCWSTR(name.as_ptr())) };
    unsafe { windows::Win32::System::Registry::RegCloseKey(hkey) };
    r.is_ok().then_some(()).ok_or_else(|| format!("字体注册表删除失败：{}", r.0))
  }

  /// 目标路径避让：同名文件已存在且字节不同 → 词干加 -anikit、再撞 -2/-3…（最多试 10 个）
  fn unique_target(dir: &Path, origin: &Path) -> Result<PathBuf, String> {
    let file_name = origin.file_name().ok_or_else(|| "字体文件名不合法".to_string())?;
    let target = dir.join(file_name);
    if !target.is_file() || same_bytes(&target, origin) {
      return Ok(target);
    }
    let stem = origin.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let ext = origin.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for i in 0..10 {
      let suffix = if i == 0 { "-anikit".to_string() } else { format!("-anikit-{i}") };
      let t = dir.join(format!("{stem}{suffix}{ext}"));
      if !t.is_file() || same_bytes(&t, origin) {
        return Ok(t);
      }
    }
    Err("用户字体目录同名文件过多，请先清理".to_string())
  }

  /// 字节同一性粗判（大小一致即采信——同名字体文件场景，逐字节比对成本不值）
  fn same_bytes(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
      (Ok(ma), Ok(mb)) => ma.len() == mb.len(),
      _ => false,
    }
  }

  /// 广播 WM_FONTCHANGE（运行中的软件刷新字体列表；收件方也可能没有，无害）
  fn broadcast() {
    unsafe {
      SendMessageTimeoutW(HWND(0xFFFF as _), WM_FONTCHANGE, WPARAM(0), LPARAM(0), SMTO_ABORTIFHUNG, 100, None);
    }
  }
}

/// 字体是否已装到系统（Windows 注册表查询；非 Windows 恒 false）
pub fn font_installed(full_names: &[String]) -> bool {
  #[cfg(windows)]
  {
    win_install::is_installed(full_names)
  }
  #[cfg(not(windows))]
  {
    let _ = full_names;
    false
  }
}

/// 安装字体到系统（用户级）；返回落盘路径
pub fn install_font(origin: &Path, full_names: &[String]) -> Result<std::path::PathBuf, String> {
  #[cfg(windows)]
  {
    win_install::install(origin, full_names)
  }
  #[cfg(not(windows))]
  {
    let _ = (origin, full_names);
    Err("当前平台暂不支持安装字体".to_string())
  }
}

/// 从系统卸载字体（用户级）
pub fn uninstall_font(full_names: &[String]) -> Result<(), String> {
  #[cfg(windows)]
  {
    win_install::uninstall(full_names)
  }
  #[cfg(not(windows))]
  {
    let _ = full_names;
    Err("当前平台暂不支持卸载字体".to_string())
  }
}

/// 字体全部 face 全名（安装注册用：ttc 一个文件含多 face，逐 face 注册）
pub fn font_full_names(path: &Path) -> Vec<String> {
  let Ok(data) = fs::read(path) else { return Vec::new() };
  let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
  let mut names = Vec::new();
  for i in 0..count {
    let Ok(face) = ttf_parser::Face::parse(&data, i) else { continue };
    if let Some(s) = name_string(&face, 4) {
      if !s.is_empty() && !names.contains(&s) {
        names.push(s);
      }
    }
  }
  names
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ext_gate_rejects_non_font() {
    assert!(!is_font(Path::new("文档.txt")));
    assert!(is_font(Path::new("字库.ttf")));
    assert!(is_font(Path::new("字库.OTF")));
    assert!(is_font(Path::new("合集.ttc")));
    // 非字体不启动解析
    assert_eq!(probe_font(Path::new("文档.txt")), None);
    assert!(render_specimen(Path::new("文档.txt"), 640, 480).is_none());
  }

  #[test]
  fn probe_and_render_fail_gracefully_on_garbage() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("假字体.ttf");
    fs::write(&f, b"not a font").unwrap();
    assert_eq!(probe_font(&f), None);
    assert!(render_specimen(&f, SPECIMEN_W, SPECIMEN_H).is_none());
    assert!(font_full_names(&f).is_empty());
  }

  #[cfg(windows)]
  #[test]
  fn reg_value_name_appends_truetype_suffix() {
    assert_eq!(win_install::reg_value_name("Hisense Alfabet Regular Italic"), "Hisense Alfabet Regular Italic (TrueType)");
  }

  /// 需要真字体文件：ANIKIT_TEST_FONT=某.ttf（目检调试用，样张落盘 target/specimen_debug.png）
  #[test]
  fn real_font_probe_and_render() {
    let Ok(p) = std::env::var("ANIKIT_TEST_FONT") else { return };
    let path = Path::new(&p);
    let meta = probe_font(path).expect("字体元数据应成功");
    assert!(!meta.family.is_empty() || !meta.name.is_empty(), "{meta:?}");
    assert!(meta.glyph_count > 0);
    let img = render_specimen(path, SPECIMEN_W, SPECIMEN_H).expect("样张应渲染成功");
    assert_eq!((img.width(), img.height()), (SPECIMEN_W, SPECIMEN_H));
    let painted = img.pixels().filter(|px| px.0[3] > 0).count();
    assert!(painted > 1000, "样张应有成规模像素，实际 {painted}");
    let _ = img.save(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/specimen_debug.png"));
    assert!(!font_full_names(path).is_empty());
  }
}
