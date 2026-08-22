//! 命令 API：预览窗口——资源文件路径解析、序列帧 / 动图帧清单、默认程序打开、新窗口打开。
//! 命令层只做取库与参数传递，路径推导走 services；重活（psd 解码 / 帧条生成）一律进
//! 预览解析队列异步执行，命令立即返回（2026-07-25 预览解析队列改造：快速切换素材不堆积）。
//! 封面帧切换（序列帧/动图/视频）在 commands/thumb_frame.rs；
//! 动图（2026-07-26 定案）解包后与序列帧同构，帧清单同走 list_sequence_frames。

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::adapters::shell_open;
use crate::commands::library::AppState;
use crate::core::json_io::read_json;
use crate::core::library::Library;
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
    // 阶段四一轮：动图播放改自研解码帧流（WS 推帧）；存量 anim.webp 文件留盘待清理口径
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

// ---------- 序列帧 / 动图帧清单 ----------

/// 序列帧帧清单（预览窗口播放条）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceFrames {
  /// 播放帧率（metadata.fps，缺省 24）
  pub fps: Option<f64>,
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
/// （存 cache/frames 不进 info，批次完成经 preview-strips-ready 推送）。
/// **动图不走此命令（2026-08-20 解包废除）**：播放 / 帧条 / 设封面全部对 origin 运行时解码
/// （帧流 / prepare_anim_strips / set_thumb_frame 抽帧），调用报"动图无序列帧清单"。
#[tauri::command]
pub async fn list_sequence_frames(state: State<'_, AppState>, asset_id: String) -> Result<SequenceFrames, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  sequence_frames(lib, &asset_id)
}

fn sequence_frames(lib: &Library, asset_id: &str) -> Result<SequenceFrames, String> {
  let (info_dir, _origin, meta) = locate_asset(lib, asset_id)?;
  if crate::services::anim::is_anim(&meta) {
    return Err(format!("「{}」是动图资源，无序列帧清单（播放/帧条走帧流）", meta.name));
  }
  let frames_dir = info_dir.join("frames");
  if !frames_dir.is_dir() {
    return Err(format!("「{}」不是序列帧资源", meta.name));
  }
  let frames = scan_frames(lib, asset_id, &frames_dir);
  Ok(SequenceFrames { fps: Some(meta.fps.unwrap_or(DEFAULT_FPS)), thumb_frame: meta.thumb_frame, frames })
}

/// 动图帧条清单（阶段四·解包废除，2026-08-20）：返回已缓存帧条（cache/frames/{id}/f-*.webp，
/// key = `f:{索引}`），同时提交预览解析队列单趟流式解码补齐未缓存帧（批次经 preview-strips-ready
/// 渐进推送，与序列帧帧条同一事件）。**立即返回**（不阻塞解码）。
#[tauri::command]
pub async fn prepare_anim_strips(state: State<'_, AppState>, asset_id: String) -> Result<AnimStrips, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  prepare_anim_strips_inner(lib, &asset_id)
}

fn prepare_anim_strips_inner(lib: &Library, asset_id: &str) -> Result<AnimStrips, String> {
  let (_info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  if !crate::services::anim::is_anim(&meta) {
    return Err(format!("「{}」不是动图资源", meta.name));
  }
  // 已缓存清单（扫目录而非按 frameCount 推导——探测帧数与实际帧数可能出入）
  let dir = lib.root.join(crate::core::library::CACHE_DIR).join("frames").join(asset_id);
  let mut items: Vec<AnimStripItem> = fs::read_dir(&dir)
    .map(|rd| {
      rd.filter_map(|e| e.ok())
        .filter_map(|e| {
          let name = e.file_name().to_string_lossy().into_owned();
          let idx = name.strip_prefix("f-")?.strip_suffix(".webp")?.parse::<u32>().ok()?;
          Some(AnimStripItem { name: format!("f:{idx}"), strip_path: e.path().to_string_lossy().into_owned() })
        })
        .collect()
    })
    .unwrap_or_default();
  items.sort_by_key(|it| it.name.clone());
  // 提交后台补齐（worker 内幂等跳过已缓存帧）
  preview_queue::global().submit(PreviewTask::AnimStrips {
    root: lib.root.clone(),
    asset_id: asset_id.to_string(),
    origin,
  });
  Ok(AnimStrips { items })
}

/// 动图帧条单条结果（name = `f:{索引}`，与 preview-strips-ready 事件 items 同构）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimStripItem {
  pub name: String,
  pub strip_path: String,
}

/// prepare_anim_strips 返回
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimStrips {
  pub items: Vec<AnimStripItem>,
}

/// 扫 frames/ 按文件名组装帧清单（帧可能缺号，不按序号推导）；未缓存帧条收集进预览解析队列
/// 后台生成
fn scan_frames(
  lib: &Library,
  asset_id: &str,
  frames_dir: &std::path::Path,
) -> Vec<FrameEntry> {
  let mut names: Vec<String> = fs::read_dir(frames_dir)
    .map(|rd| {
      rd.filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
    })
    .unwrap_or_default();
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
  frames
}

// ---------- 默认程序打开 ----------

/// 用系统默认程序打开源文件（Windows ShellExecuteW "open"；非 Windows 暂不支持）。
/// 打开成功才记最近使用（2026-07-29 用户定：recent 唯一启用入口）；记录失败只记日志降级
#[tauri::command]
pub async fn open_with_default_app(app: AppHandle, state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, _meta) = locate_asset(lib, &asset_id)?;
  shell_open::open(&origin)?;
  match crate::services::recent::touch_recent(lib, &asset_id) {
    Ok(()) => {
      // 最近使用热刷新（2026-07-29 复验反馈：原仅全量 refresh 拉取，要重启才更新）——
      // 记录成功广播，前端 content store 监听重拉 recentAssets
      //（未来哈希变更 / 发送动画软件 / 跨窗口拖出三入口挂钩时同发此事件即可）
      let _ = app.emit("recent-changed", ());
    }
    Err(e) => log::error!("最近使用记录失败（打开已成功，不阻断）：{e}"),
  }
  Ok(())
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
  let info_dir = lib.info_dir(asset_id);
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
    let lib = library::create_test(dir.path());
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
      sequence::build_sequence_asset(&h.lib.assets_dir, "走路", "png", 3, &frames, "sha256:t", id, Source::Copy, None)
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
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  #[test]
  fn get_asset_file_path_direct_render_returns_origin() {
    let mut h = make_lib();
    add_file(&mut h, "AN-PNG", "参考图.png");
    let result = asset_file_path(&h.lib, "AN-PNG").unwrap();
    let expected = h.lib.info_dir("AN-PNG").join("参考图.png");
    assert_eq!(
      result,
      AssetFilePath { path: Some(expected.to_string_lossy().into_owned()), pending: false }
    );
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

  /// 动图：path 恒为源文件（播放 = 自研解码帧流 start_anim_stream）
  #[test]
  fn get_asset_file_path_anim_returns_origin() {
    let mut h = make_lib();
    add_anim(&mut h, "AN-GIF3");
    let origin = h.lib.info_dir("AN-GIF3").join("anim3.gif");

    let r = asset_file_path(&h.lib, "AN-GIF3").unwrap();
    assert_eq!(r.path.as_deref(), Some(origin.to_string_lossy().as_ref()));
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
    assert_eq!(list.fps, Some(24.0));
    assert_eq!(list.thumb_frame.as_deref(), Some("走路-001.png"));
    let names: Vec<&str> = list.frames.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["走路-001.png", "走路-002.png", "走路-003.png"]);

    // 立即返回：帧条未缓存时 stripPath=null（后台队列异步生成）；语义：Some 时文件必须存在
    let info = h.lib.info_dir("AN-SEQ");
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
    let info = h.lib.info_dir("AN-SEQ2");
    write_json(&info.join("metadata.json"), &meta).unwrap();
    assert_eq!(sequence_frames(&h.lib, "AN-SEQ2").unwrap().fps, Some(24.0));

    // 非序列帧资源（无 frames 目录）中文报错
    add_file(&mut h, "AN-ONE", "单张.png");
    let err = sequence_frames(&h.lib, "AN-ONE").unwrap_err();
    assert!(err.contains("不是序列帧"), "{err}");
  }

  /// 造一个动图资源（真实样本 anim3.gif 入库，probe 写 frameCount/duration）并登记进内存索引
  fn add_anim(h: &mut Harness, id: &str) -> AssetMeta {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/anim3.gif");
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  /// 动图不走帧清单命令（2026-08-20 解包废除：播放/帧条/设封面对 origin 运行时解码）；
  /// 存量解包产物（frames/）不再被误当序列帧返回
  #[test]
  fn list_sequence_frames_rejects_anim() {
    let _lock = TEST_LOCK.lock().unwrap();
    let mut h = make_lib();
    add_anim(&mut h, "AN-GIF");
    let err = sequence_frames(&h.lib, "AN-GIF").unwrap_err();
    assert!(err.contains("动图"), "{err}");

    // 存量 frames/ 目录在场同判（历史解包产物）
    let info = h.lib.info_dir("AN-GIF");
    fs::create_dir_all(info.join("frames")).unwrap();
    fs::write(info.join("frames").join("anim3-00001.webp"), b"legacy").unwrap();
    let err = sequence_frames(&h.lib, "AN-GIF").unwrap_err();
    assert!(err.contains("动图"), "{err}");
  }

  /// 动图帧条：prepare_anim_strips 立即返回（空缓存）+ 队列单趟流式解码补齐（f-0000N.webp，
  /// 短边 128 会话级）；二次调用命中缓存直给；幂等（mtime 不变）
  #[test]
  fn prepare_anim_strips_decodes_origin_into_cache() {
    let _lock = TEST_LOCK.lock().unwrap();
    let mut h = make_lib();
    add_anim(&mut h, "AN-GIF3");

    let first = prepare_anim_strips_inner(&h.lib, "AN-GIF3").unwrap();
    assert!(first.items.is_empty(), "首次缓存为空：{:?}", first.items.len());

    // 队列后台生成（cache/frames/{id}/f-00000..2.webp，anim3.gif 共 3 帧，索引 0 起）
    let strip_dir = h.lib.root.join(CACHE_DIR).join("frames").join("AN-GIF3");
    wait_until(
      || (0..=2).all(|i| strip_dir.join(format!("f-{i:05}.webp")).is_file()),
      "动图帧条后台生成",
    );
    for i in 0..=2u32 {
      let (w, hh) = image::image_dimensions(strip_dir.join(format!("f-{i:05}.webp"))).unwrap();
      // anim3.gif 源 64×48 短边不足 128 不放大（shrink_to_edge 口径）
      assert_eq!((w, hh), (64, 48), "帧条应保持原尺寸不放大：{w}x{hh}");
    }

    // 二次调用：命中缓存直给（key = f:{索引}，按名排序）
    let second = prepare_anim_strips_inner(&h.lib, "AN-GIF3").unwrap();
    let keys: Vec<&str> = second.items.iter().map(|it| it.name.as_str()).collect();
    assert_eq!(keys, ["f:0", "f:1", "f:2"]);
    for it in &second.items {
      assert!(PathBuf::from(&it.strip_path).is_file(), "stripPath 文件必须存在：{}", it.strip_path);
    }

    // 幂等：已存在不重复生成（mtime 不变）
    let mtime = fs::metadata(strip_dir.join("f-00000.webp")).unwrap().modified().unwrap();
    prepare_anim_strips_inner(&h.lib, "AN-GIF3").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200)); // 等队列跑完一趟（全跳过）
    assert_eq!(fs::metadata(strip_dir.join("f-00000.webp")).unwrap().modified().unwrap(), mtime);

    // 非动图资源报错
    add_file(&mut h, "AN-PIC", "单张.png");
    assert!(prepare_anim_strips_inner(&h.lib, "AN-PIC").is_err());
  }
}
