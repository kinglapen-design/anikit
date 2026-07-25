//! 命令 API：预览窗口——资源文件路径解析、序列帧帧清单、封面帧切换、默认程序打开。
//! 命令层只做取库与参数传递，路径推导与帧条预生成走 services（01_开发规范 第 1 条）。

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{Manager, State};

use crate::adapters::shell_open;
use crate::commands::library::AppState;
use crate::core::json_io::{read_json, write_json};
use crate::core::library::{Library, ASSTES_DIR};
use crate::core::models::AssetMeta;
use crate::services::sequence::DEFAULT_FPS;
use crate::services::thumbnail;

// ---------- 预览文件路径解析 ----------

/// webview 可直接渲染的扩展名（小写，无点）
fn is_webview_direct(ext: &str) -> bool {
  matches!(ext, "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "svg")
}

/// 预览文件路径解析：webview 可直渲格式返回源文件绝对路径；psd / psb 走解码预览缓存
/// （cache/tmp，会话级临时）；其余格式返回 null（前端用格式图标兜底）。
/// async：Tauri v2 非 async 命令在主线程执行，IO 重活一律线程池（2026-07-25 死锁教训）
#[tauri::command]
pub async fn get_asset_file_path(state: State<'_, AppState>, asset_id: String) -> Result<Option<String>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_file_path(lib, &asset_id)
}

fn asset_file_path(lib: &Library, asset_id: &str) -> Result<Option<String>, String> {
  let (_info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  let ext = meta.ext.to_lowercase();
  if is_webview_direct(&ext) {
    return Ok(Some(origin.to_string_lossy().into_owned()));
  }
  if matches!(ext.as_str(), "psd" | "psb") {
    return Ok(thumbnail::generate_preview(&lib.root, &origin, asset_id));
  }
  Ok(None)
}

// ---------- 序列帧帧清单 ----------

/// 序列帧帧清单（预览窗口播放条）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceFrames {
  /// 播放帧率（metadata.fps，缺省 24）
  pub fps: f64,
  /// 封面帧文件名（frames/ 内归一名；metadata 未记时为 null）
  pub thumb_frame: Option<String>,
  pub frames: Vec<FrameEntry>,
}

/// 单帧条目
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameEntry {
  /// 帧文件名（frames/ 内归一名 标题-序号.扩展名）
  pub name: String,
  /// 帧原图绝对路径
  pub full_path: String,
  /// 帧条小图绝对路径（cache/frames/{asset_id}/{帧名}.webp，短边 128 等比不放大；
  /// 生成失败时回退原图路径，前端小尺寸直渲）
  pub strip_path: String,
}

/// 序列帧帧清单：扫 frames/ 按文件名排序（帧可能缺号，不按序号推导）；逐帧预生成帧条小图
/// （存 cache/frames 不进 info，已存在跳过）。非序列帧资源（无 frames 目录）报错。
#[tauri::command]
pub async fn list_sequence_frames(state: State<'_, AppState>, asset_id: String) -> Result<SequenceFrames, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  sequence_frames(lib, &asset_id)
}

fn sequence_frames(lib: &Library, asset_id: &str) -> Result<SequenceFrames, String> {
  let (info_dir, _origin, meta) = locate_asset(lib, asset_id)?;
  let frames_dir = info_dir.join("frames");
  if !frames_dir.is_dir() {
    return Err(format!("「{}」不是序列帧资源", meta.name));
  }
  let mut names: Vec<String> = fs::read_dir(&frames_dir)
    .map_err(|e| format!("帧目录读取失败：{e}"))?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| p.is_file())
    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    .collect();
  names.sort();
  let frames = names
    .into_iter()
    .map(|name| {
      let full = frames_dir.join(&name);
      let full_path = full.to_string_lossy().into_owned();
      let strip_path =
        thumbnail::generate_frame_strip(&lib.root, &full, asset_id, &name).unwrap_or_else(|| full_path.clone());
      FrameEntry { name, full_path, strip_path }
    })
    .collect();
  Ok(SequenceFrames { fps: meta.fps.unwrap_or(DEFAULT_FPS), thumb_frame: meta.thumb_frame, frames })
}

// ---------- 封面帧切换 ----------

/// 封面帧切换结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetThumbResult {
  /// 新缩略图绝对路径
  pub thumb_path: String,
}

/// 切换序列帧封面帧：用指定帧重新生成缩略图（与导入同管线、同命名 {资源名}_thumbnail.webp）→
/// metadata 落盘（thumbFrame / thumbnail / modifiedAt）→ 同步内存索引（先落盘再更新索引）
#[tauri::command]
pub async fn set_thumb_frame(
  state: State<'_, AppState>,
  asset_id: String,
  frame_name: String,
) -> Result<SetThumbResult, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  apply_thumb_frame(lib, &asset_id, &frame_name)
}

fn apply_thumb_frame(lib: &mut Library, asset_id: &str, frame_name: &str) -> Result<SetThumbResult, String> {
  let (info_dir, _origin, mut meta) = locate_asset(lib, asset_id)?;
  let frames_dir = info_dir.join("frames");
  if !frames_dir.is_dir() {
    return Err(format!("「{}」不是序列帧资源", meta.name));
  }
  let frame_path = frames_dir.join(frame_name);
  if !frame_path.is_file() {
    return Err(format!("帧不存在：{frame_name}"));
  }
  // 删旧缩略图再生成（sequence.rs 建档同款）。词干 = 资源名，新旧文件名通常相同：
  // 同名交给 generate 直接覆盖（避免先删后生成失败留下悬空引用），仅不同名（改名遗留）才先删
  let old_thumb = meta.thumbnail.clone();
  let stem = meta.name.clone();
  if let Some(old) = old_thumb.as_deref() {
    if old != format!("{stem}_thumbnail.webp") && old != format!("{stem}_thumbnail.png") {
      if let Err(e) = fs::remove_file(info_dir.join(old)) {
        log::warn!("旧缩略图删除失败（{old}）：{e}");
      }
    }
  }
  let thumb =
    thumbnail::generate(&info_dir, &frame_path, &stem).ok_or_else(|| format!("帧图片解码失败：{frame_name}"))?;
  // 先落盘再更新内存索引（01_开发规范 第 7 条）
  meta.thumb_frame = Some(frame_name.to_string());
  meta.thumbnail = Some(thumb.clone());
  meta.modified_at = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), meta);
  Ok(SetThumbResult { thumb_path: info_dir.join(&thumb).to_string_lossy().into_owned() })
}

// ---------- 默认程序打开 ----------

/// 用系统默认程序打开源文件（Windows ShellExecuteW "open"；非 Windows 暂不支持）
#[tauri::command]
pub async fn open_with_default_app(state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, _meta) = locate_asset(lib, &asset_id)?;
  shell_open::open(&origin)
}

// ---------- 新窗口打开 ----------

/// 新窗口打开预览（4.3）：同资产复用聚焦，否则开无边框预览窗（视觉参数同主窗口）。
/// **必须 async**——Tauri v2 非 async 命令在主线程执行，主线程内 build 窗口会与事件循环互等死锁
/// （2026-07-25 实测：界面/托盘全卡死）；async 走线程池，build 内部 dispatch 回主线程不阻塞
#[tauri::command]
pub async fn open_preview_window(app: tauri::AppHandle, asset_key: String) -> Result<(), String> {
  let asset_id = asset_key.split('|').next().unwrap_or(&asset_key);
  let label = format!("preview-{asset_id}");
  if let Some(win) = app.get_webview_window(&label) {
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }
  let url = tauri::WebviewUrl::App(
    format!("index.html?preview={}", asset_key.replace('|', "%7C")).into(),
  );
  tauri::WebviewWindowBuilder::new(&app, &label, url)
    .title("AniKit 预览")
    .inner_size(1000.0, 760.0)
    .resizable(true)
    .decorations(false)
    .shadow(true)
    .center()
    .background_color(tauri::window::Color(9, 9, 10, 255))
    .build()
    .map_err(|e| e.to_string())?;
  Ok(())
}

// ---------- 共用 ----------

/// 定位资源：读 metadata + 推导 info 目录与源文件绝对路径（PRD 01-2.3：真实路径不存绝对路径，由 ID 推导）
fn locate_asset(lib: &Library, asset_id: &str) -> Result<(PathBuf, PathBuf, AssetMeta), String> {
  let info_dir = lib.root.join(ASSTES_DIR).join(format!("{asset_id}.info"));
  if !info_dir.is_dir() {
    return Err(format!("资源不存在：{asset_id}"));
  }
  let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).map_err(|e| e.to_string())?;
  let origin = info_dir.join(&meta.origin_name);
  Ok((info_dir, origin, meta))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::services::import::{self, PendingFile, Source};
  use crate::services::sequence;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { dir, lib }
  }

  fn make_png(path: &std::path::Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([30, 30, 200, 255])).save(path).unwrap();
  }

  /// 造一个序列帧资源（走路_1..=3.png，入库归一 走路-001..003）并登记进内存索引
  fn add_sequence(h: &mut Harness, id: &str) -> AssetMeta {
    let src = h.dir.path().join(format!("源-{id}"));
    fs::create_dir_all(&src).unwrap();
    let frames: Vec<(PathBuf, u32)> = (1..=3)
      .map(|n| {
        let p = src.join(format!("走路_{n}.png"));
        make_png(&p, 640, 480);
        (p, n)
      })
      .collect();
    let meta =
      sequence::build_sequence_asset(&h.lib.root, "走路", "png", 3, &frames, "sha256:t", id, Source::Copy, None)
        .unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  /// 造一个单文件资源（png 造真图，其他扩展名写假字节）并登记进内存索引
  fn add_file(h: &mut Harness, id: &str, name: &str) -> AssetMeta {
    let src = h.dir.path().join(name);
    if name.ends_with(".png") {
      make_png(&src, 100, 80);
    } else {
      fs::write(&src, b"fake").unwrap();
    }
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.root, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  #[test]
  fn get_asset_file_path_direct_render_returns_origin() {
    let mut h = make_lib();
    add_file(&mut h, "AN-PNG", "参考图.png");
    let path = asset_file_path(&h.lib, "AN-PNG").unwrap().unwrap();
    let expected = h.lib.root.join(ASSTES_DIR).join("AN-PNG.info").join("参考图.png");
    assert_eq!(path, expected.to_string_lossy());
  }

  #[test]
  fn get_asset_file_path_other_format_returns_none() {
    let mut h = make_lib();
    add_file(&mut h, "AN-FLA", "工程.fla");
    assert_eq!(asset_file_path(&h.lib, "AN-FLA").unwrap(), None);
    // 不存在的资源报错
    assert!(asset_file_path(&h.lib, "AN-不存在").is_err());
  }

  // psd / psb 分支走 thumbnail::generate_preview（cache/tmp 幂等缓存），
  // 单测造不出合法 PSD 二进制（psd crate 只解码不编码），该分支不覆盖——
  // 分支本身只有 ext 匹配 + 一行转发，generate_preview 已有自身测试。

  #[test]
  fn list_sequence_frames_sorts_and_pregenerates_strips() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-SEQ");
    let list = sequence_frames(&h.lib, "AN-SEQ").unwrap();

    // 按文件名排序，fps 取 metadata（建档默认 24），thumbFrame 取 metadata
    assert_eq!(list.fps, 24.0);
    assert_eq!(list.thumb_frame.as_deref(), Some("走路-001.png"));
    let names: Vec<&str> = list.frames.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["走路-001.png", "走路-002.png", "走路-003.png"]);

    // 每帧：原图在 info/frames，帧条小图预生成到 cache/frames/{id}/（短边 128 等比，不进 info）
    let info = h.lib.root.join(ASSTES_DIR).join("AN-SEQ.info");
    for f in &list.frames {
      assert!(PathBuf::from(&f.full_path).is_file(), "帧原图缺失：{}", f.full_path);
      assert!(f.full_path.starts_with(info.join("frames").to_string_lossy().as_ref()));
      let strip = PathBuf::from(&f.strip_path);
      assert!(strip.is_file(), "帧条缺失：{}", f.strip_path);
      assert!(f.strip_path.contains("cache"), "帧条应存 cache：{}", f.strip_path);
      assert!(f.strip_path.ends_with(&format!("{}.webp", f.name)));
      let (w, h) = image::image_dimensions(&strip).unwrap();
      assert_eq!(h.min(w), 128, "帧条短边应为 128：{w}x{h}");
      assert!(!info.join(format!("{}.webp", f.name)).exists(), "帧条不应进 info");
    }

    // 二次调用幂等：已存在跳过（mtime 不变）
    let first = list.frames[0].strip_path.clone();
    let mtime = fs::metadata(&first).unwrap().modified().unwrap();
    let again = sequence_frames(&h.lib, "AN-SEQ").unwrap();
    assert_eq!(again.frames[0].strip_path, first);
    assert_eq!(fs::metadata(&first).unwrap().modified().unwrap(), mtime);
  }

  #[test]
  fn list_sequence_frames_defaults_fps_and_rejects_non_sequence() {
    let mut h = make_lib();
    let mut meta = add_sequence(&mut h, "AN-SEQ2");
    // metadata 无 fps 字段（历史数据 / 手改）→ 缺省 24
    meta.fps = None;
    let info = h.lib.root.join(ASSTES_DIR).join("AN-SEQ2.info");
    write_json(&info.join("metadata.json"), &meta).unwrap();
    assert_eq!(sequence_frames(&h.lib, "AN-SEQ2").unwrap().fps, 24.0);

    // 非序列帧资源（无 frames 目录）中文报错
    add_file(&mut h, "AN-ONE", "单张.png");
    let err = sequence_frames(&h.lib, "AN-ONE").unwrap_err();
    assert!(err.contains("不是序列帧"), "{err}");
  }

  #[test]
  fn set_thumb_frame_regenerates_and_syncs_index() {
    let mut h = make_lib();
    let before = add_sequence(&mut h, "AN-SEQ3");
    let info = h.lib.root.join(ASSTES_DIR).join("AN-SEQ3.info");

    let result = apply_thumb_frame(&mut h.lib, "AN-SEQ3", "走路-002.png").unwrap();
    // 新缩略图沿用导入命名（词干 = 资源名），绝对路径指向 info 内文件
    assert!(result.thumb_path.ends_with("走路_thumbnail.webp"), "{}", result.thumb_path);
    assert!(PathBuf::from(&result.thumb_path).is_file());

    // metadata 落盘：thumbFrame / thumbnail / modifiedAt 更新
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.thumb_frame.as_deref(), Some("走路-002.png"));
    assert_eq!(disk.thumbnail.as_deref(), Some("走路_thumbnail.webp"));
    assert_ne!(disk.thumb_frame, before.thumb_frame);

    // 内存索引同步（先落盘再更新）
    let indexed = &h.lib.index.assets["AN-SEQ3"];
    assert_eq!(indexed.thumb_frame.as_deref(), Some("走路-002.png"));
    assert_eq!(indexed.thumbnail.as_deref(), Some("走路_thumbnail.webp"));

    // 帧不存在 / 非序列帧资源 → 中文报错
    assert!(apply_thumb_frame(&mut h.lib, "AN-SEQ3", "走路-999.png").is_err());
    add_file(&mut h, "AN-ONE2", "单张.png");
    let err = apply_thumb_frame(&mut h.lib, "AN-ONE2", "走路-001.png").unwrap_err();
    assert!(err.contains("不是序列帧"), "{err}");
  }
}
