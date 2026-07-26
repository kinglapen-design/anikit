//! 命令 API：预览窗口——资源文件路径解析、序列帧帧清单、默认程序打开、新窗口打开。
//! 命令层只做取库与参数传递，路径推导走 services；重活（psd 解码 / 帧条生成）一律进
//! 预览解析队列异步执行，命令立即返回（2026-07-25 预览解析队列改造：快速切换素材不堆积）。
//! 封面帧切换（序列帧/动图/视频）在 commands/thumb_frame.rs；动图帧清单在 commands/anim.rs。

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{Manager, State};

use crate::adapters::shell_open;
use crate::commands::library::AppState;
use crate::core::json_io::read_json;
use crate::core::library::{Library, ASSTES_DIR};
use crate::core::models::AssetMeta;
use crate::services::preview_queue::{self, PreviewTask};
use crate::services::sequence::DEFAULT_FPS;
use crate::services::thumbnail;

// ---------- 预览文件路径解析 ----------

/// webview 可直接渲染的扩展名（小写，无点）
fn is_webview_direct(ext: &str) -> bool {
  matches!(ext, "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "svg")
}

/// 预览文件路径解析结果
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetFilePath {
  /// 可渲染的文件路径（直渲格式 = 源路径；psd/psb = 已缓存的解码预览）；暂不可得为 null
  pub path: Option<String>,
  /// true = 解析中（psd/psb 已进预览解析队列），完成经 preview-full-ready 事件推送
  pub pending: bool,
}

/// 预览文件路径解析：webview 可直渲格式返回源文件绝对路径；psd / psb 缓存命中直给、
/// 未命中进预览解析队列（pending=true，解码完成经 preview-full-ready 推送）；
/// 其余格式 path=null（前端用格式图标兜底）。
/// async：Tauri v2 非 async 命令在主线程执行，IO 重活一律线程池（2026-07-25 死锁教训）
#[tauri::command]
pub async fn get_asset_file_path(state: State<'_, AppState>, asset_id: String) -> Result<AssetFilePath, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_file_path(lib, &asset_id)
}

fn asset_file_path(lib: &Library, asset_id: &str) -> Result<AssetFilePath, String> {
  let (_info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  let ext = meta.ext.to_lowercase();
  if is_webview_direct(&ext) {
    return Ok(AssetFilePath { path: Some(origin.to_string_lossy().into_owned()), pending: false });
  }
  if matches!(ext.as_str(), "psd" | "psb") {
    // 渐进加载高清层（Eagle 式，2026-07-25 用户定）：全尺寸缓存 cache/previews（持久、按源文件失效）。
    // 只判存在不解码——generate_full_preview 未命中会同步解码 1-2s，命令层直接调会堵线程池
    if let Some(path) = thumbnail::full_preview_path(&lib.root, &origin, asset_id) {
      return Ok(AssetFilePath { path: Some(path), pending: false });
    }
    preview_queue::global().submit(PreviewTask::FullPreview {
      root: lib.root.clone(),
      asset_id: asset_id.to_string(),
      origin,
    });
    return Ok(AssetFilePath { path: None, pending: true });
  }
  Ok(AssetFilePath { path: None, pending: false })
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
  /// 帧条小图绝对路径（cache/frames/{asset_id}/{帧名}.webp，短边 128 等比不放大）；
  /// null = 未缓存（已进预览解析队列后台生成，批次完成经 preview-strips-ready 推送；
  /// 生成失败维持 null，前端小尺寸直渲 fullPath 兜底）
  pub strip_path: Option<String>,
}

/// 序列帧帧清单：扫 frames/ 按文件名排序（帧可能缺号，不按序号推导）；**立即返回**——
/// 帧条不再同步生成：已缓存的填 stripPath，未缓存帧收集进预览解析队列后台生成
/// （存 cache/frames 不进 info，批次完成经 preview-strips-ready 推送）。非序列帧资源（无 frames 目录）报错。
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
  let mut uncached: Vec<(String, PathBuf)> = Vec::new();
  let frames = names
    .into_iter()
    .map(|name| {
      let full = frames_dir.join(&name);
      let strip_path = thumbnail::frame_strip_path(&lib.root, asset_id, &name);
      if strip_path.is_none() {
        uncached.push((name.clone(), full.clone()));
      }
      FrameEntry { name, full_path: full.to_string_lossy().into_owned(), strip_path }
    })
    .collect();
  if !uncached.is_empty() {
    preview_queue::global().submit(PreviewTask::FrameStrips {
      root: lib.root.clone(),
      asset_id: asset_id.to_string(),
      frames: uncached,
    });
  }
  Ok(SequenceFrames { fps: meta.fps.unwrap_or(DEFAULT_FPS), thumb_frame: meta.thumb_frame, frames })
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
/// pub(crate)：thumb_frame / anim 等预览系命令共用
pub(crate) fn locate_asset(lib: &Library, asset_id: &str) -> Result<(PathBuf, PathBuf, AssetMeta), String> {
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
  use crate::core::json_io::write_json;
  use crate::core::library::{self, CACHE_DIR};
  use crate::services::import::{self, PendingFile, Source};
  use crate::services::preview_queue::TEST_LOCK;
  use crate::services::sequence;

  /// 轮询等待条件成立（10s 超时判定失败），不裸 sleep 长等
  fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !cond() {
      assert!(std::time::Instant::now() < deadline, "等待超时：{what}");
      std::thread::sleep(std::time::Duration::from_millis(5));
    }
  }

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
    let result = asset_file_path(&h.lib, "AN-PNG").unwrap();
    let expected = h.lib.root.join(ASSTES_DIR).join("AN-PNG.info").join("参考图.png");
    assert_eq!(result, AssetFilePath { path: Some(expected.to_string_lossy().into_owned()), pending: false });
  }

  #[test]
  fn get_asset_file_path_other_format_returns_none() {
    let mut h = make_lib();
    add_file(&mut h, "AN-FLA", "工程.fla");
    // 不可直渲也不走解码的格式：path=null 且不入队（前端格式图标兜底）
    assert_eq!(asset_file_path(&h.lib, "AN-FLA").unwrap(), AssetFilePath { path: None, pending: false });
    // 不存在的资源报错
    assert!(asset_file_path(&h.lib, "AN-不存在").is_err());
  }

  // psd / psb 分支：缓存命中走 full_preview_path 直给（thumbnail.rs 已测），未命中进队列
  // 返回 pending=true。单测造不出合法 PSD 二进制（psd crate 只解码不编码），该分支不覆盖——
  // 分支本身只有 ext 匹配 + 查缓存 + submit 转发，队列与缓存各有自身测试。

  #[test]
  fn list_sequence_frames_returns_immediately_and_enqueues_strips() {
    let _lock = TEST_LOCK.lock().unwrap();
    let mut h = make_lib();
    add_sequence(&mut h, "AN-SEQ");
    let list = sequence_frames(&h.lib, "AN-SEQ").unwrap();

    // 按文件名排序，fps 取 metadata（建档默认 24），thumbFrame 取 metadata
    assert_eq!(list.fps, 24.0);
    assert_eq!(list.thumb_frame.as_deref(), Some("走路-001.png"));
    let names: Vec<&str> = list.frames.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["走路-001.png", "走路-002.png", "走路-003.png"]);

    // 立即返回：帧条未缓存时 stripPath=null（后台队列异步生成）；语义：Some 时文件必须存在
    let info = h.lib.root.join(ASSTES_DIR).join("AN-SEQ.info");
    let strips: Vec<PathBuf> = list
      .frames
      .iter()
      .map(|f| h.lib.root.join(CACHE_DIR).join("frames").join("AN-SEQ").join(format!("{}.webp", f.name)))
      .collect();
    for f in &list.frames {
      assert!(PathBuf::from(&f.full_path).is_file(), "帧原图缺失：{}", f.full_path);
      assert!(f.full_path.starts_with(info.join("frames").to_string_lossy().as_ref()));
      if let Some(p) = &f.strip_path {
        assert!(PathBuf::from(p).is_file(), "stripPath 非空时文件必须存在：{p}");
      }
    }

    // 后台队列最终生成全部帧条（cache/frames/{id}/，短边 128 等比，不进 info）
    wait_until(|| strips.iter().all(|p| p.is_file()), "帧条后台生成");
    for (f, strip) in list.frames.iter().zip(&strips) {
      let (w, h) = image::image_dimensions(strip).unwrap();
      assert_eq!(h.min(w), 128, "帧条短边应为 128：{w}x{h}");
      assert!(!info.join(format!("{}.webp", f.name)).exists(), "帧条不应进 info");
    }

    // 二次调用：全部命中缓存，stripPath 直给且不再入队
    let again = sequence_frames(&h.lib, "AN-SEQ").unwrap();
    for (f, strip) in again.frames.iter().zip(&strips) {
      assert_eq!(f.strip_path.as_deref(), Some(strip.to_string_lossy().as_ref()));
    }

    // 幂等：已存在不重复生成（mtime 不变）
    let mtime = fs::metadata(&strips[0]).unwrap().modified().unwrap();
    sequence_frames(&h.lib, "AN-SEQ").unwrap();
    assert_eq!(fs::metadata(&strips[0]).unwrap().modified().unwrap(), mtime);
  }

  #[test]
  fn list_sequence_frames_defaults_fps_and_rejects_non_sequence() {
    let _lock = TEST_LOCK.lock().unwrap();
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
}
