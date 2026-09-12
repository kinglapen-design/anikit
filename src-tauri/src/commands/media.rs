//! 命令 API：音视频播放源（5.2）。前端用 <video>/<audio> + convertFileSrc 播放，
//! WebView2（Chromium）可直播：mp4(H.264 + AAC/MP3) / mp3 / wav；其余格式（wma、mov/avi、
//! HEVC 等）经 ffmpeg 转码兜底——视频转 cache/media/media-{assetId}.mp4（libopenh264 + aac），
//! 音频转 cache/media/media-{assetId}.wav（pcm_s16le）。转码幂等：产物旁 .stamp 记源文件
//! "大小:mtime秒"，一致直接复用不重复转码；源变化重转。sidecar 缺失 / 转码失败给中文文案。
//! 产物目录 cache/media 跨会话保留（2026-09-13 用户实机定：cache/tmp 开库清空导致每次启动
//! 后首次播放都重新转码，CPU 长时间满载）——stamp 已能保证源变化重转，无需清空防膨胀。
//! 锁纪律：全局库锁仅用于入口快照（locate_asset + 克隆 root 后立即放锁），ffprobe / ffmpeg
//! 重活全部在锁外线程池执行——持锁转码（数十秒级）会堵死主线程同步命令，导致界面「未响应」。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{Emitter, State};

use crate::adapters::ffmpeg;
use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::json_io::write_json;
use crate::core::library::CACHE_DIR;
use crate::core::models::AssetMeta;
use crate::services::{media, thumbnail};

/// 音视频播放源解析结果
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaSource {
  /// 可播放文件绝对路径（transcoded=false = 源文件；true = cache/media 转码产物）
  pub path: String,
  /// true = webview 播不了源格式，返回的是后端转码产物（mp4/wav）
  pub transcoded: bool,
  /// 素材自身帧率（视频；2026-07-28 复验反馈修复：自愈后的准确值随响应下发，
  /// 前端播放条/笔记立即正确，不等重拉 PlacementView）；音频 / 未知为 null
  pub fps: Option<f64>,
}

/// 播放路由：直渲（源文件可直接喂 <video>/<audio>）还是转码兜底
#[derive(Debug, PartialEq, Eq)]
enum Route {
  Direct,
  Transcode,
}

/// 同资产播放源解析并发去重：并发重复调用串行化，后者拿到锁时转码产物 stamp 已命中，
/// 直接复用，消除双重转码 / 重复探测。条目按资产留驻（每资产一条 Arc，体积极小，不清理）
static MEDIA_SRC_LOCKS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<tokio::sync::Mutex<()>>>>> =
  std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// 音视频播放源解析：可直播格式直给源文件绝对路径；其余转码（幂等复用，见模块头）。
/// 非音视频资源中文报错；ffmpeg 缺失报"组件缺失"，转码失败报"该格式暂时无法播放"。
/// 三段式：短锁快照 → 锁外线程池重活（ffprobe / ffmpeg，规范第 2 条）→ 自愈回写短锁
#[tauri::command]
pub async fn get_media_source(
  app: tauri::AppHandle,
  state: State<'_, AppState>,
  asset_id: String,
) -> Result<MediaSource, String> {
  // 短锁快照：定位资源、克隆出后续所需全部字段后立即放锁
  let (info_dir, origin, meta, root) = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let (info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
    (info_dir, origin, meta, lib.root.clone())
  };
  // 同资产并发去重（见 MEDIA_SRC_LOCKS 注释），guard 持到重活与回写完成
  let asset_lock = {
    let mut map = MEDIA_SRC_LOCKS.lock().map_err(|e| e.to_string())?;
    map
      .entry(asset_id.clone())
      .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
      .clone()
  };
  let _permit = asset_lock.lock().await;
  // 锁外重活：fps 补探测 / ffprobe / 转码走线程池
  let asset_id2 = asset_id.clone();
  let (src, healed) =
    tauri::async_runtime::spawn_blocking(move || media_source_heavy(&info_dir, &origin, meta, &root, &asset_id2))
      .await
      .map_err(|e| format!("播放源解析任务失败：{e}"))??;
  // fps 自愈回写（仅视频且探测到新 fps）：metadata.json 已在锁外落盘，这里短锁同步内存索引；
  // 库已关则跳过同步与广播（落盘已完成，无害）
  if let Some(meta) = healed {
    let mut guard = state.library.lock().map_err(|e| e.to_string())?;
    if let Some(lib) = guard.as_mut() {
      let fps = meta.fps.unwrap_or(0.0);
      lib.index.assets.insert(asset_id.clone(), meta);
      let _ = app.emit("video-fps-healed", serde_json::json!({ "assetId": asset_id, "fps": fps }));
    }
  }
  Ok(src)
}

/// 播放源解析重活（锁外线程池执行）：fps 自愈探测 → ffprobe 路由 → 直渲 / 转码。
/// 返回 (播放源, 自愈成功待回写内存索引的 meta)；fps 自愈先落盘 metadata.json 再交由调用方
/// 短锁同步内存索引（规范第 7 条），探测 / 落盘失败静默跳过——下次播放再试，不阻塞播放
fn media_source_heavy(
  info_dir: &Path,
  origin: &Path,
  mut meta: AssetMeta,
  root: &Path,
  asset_id: &str,
) -> Result<(MediaSource, Option<AssetMeta>), String> {
  let video = media::is_video(origin);
  if !video && !media::is_audio(origin) {
    return Err(format!("「{}」不是音视频资源", meta.name));
  }
  let ext = meta.ext.to_lowercase();
  let mut fps = meta.fps;
  let mut healed = None;
  // 存量视频 fps 自愈（5.3 帧化改造）：5.3 前导入的视频 metadata 无 fps，
  // 首次播放解析时补探测写回；写回成功后调用方广播 video-fps-healed
  // （2026-07-28 复验反馈修复）：前端 content store 同步 PlacementView.fps——
  // 卡片悬停胶囊 / 详情笔记即时正确，免重开窗口重拉
  if video && fps.is_none() {
    if let Some(f) = media::probe_video(origin).and_then(|v| v.fps) {
      meta.fps = Some(f);
      match write_json(&info_dir.join("metadata.json"), &meta) {
        Ok(()) => {
          fps = Some(f);
          healed = Some(meta);
        }
        Err(e) => log::warn!("视频 fps 自愈落盘失败（{asset_id}）：{e}"),
      }
    }
  }
  let probe = ffmpeg::probe_json(origin);
  let mut src = if route(&ext, video, probe.as_ref()) == Route::Direct {
    MediaSource { path: origin.to_string_lossy().into_owned(), transcoded: false, fps: None }
  } else {
    transcode_source(root, asset_id, origin, video)?
  };
  src.fps = fps;
  Ok((src, healed))
}

/// 播放路由判定（纯逻辑，单测直接构造 probe JSON）：
/// 音频 = 扩展名 mp3/wav 直渲；视频 = 容器 mp4/m4v + 视频流 h264 + 音频流 aac/mp3（或无音频流）直渲
fn route(ext: &str, is_video: bool, probe: Option<&serde_json::Value>) -> Route {
  if !is_video {
    return if matches!(ext, "mp3" | "wav") { Route::Direct } else { Route::Transcode };
  }
  if !matches!(ext, "mp4" | "m4v") {
    return Route::Transcode;
  }
  let streams = probe.and_then(|p| p.get("streams")).and_then(|s| s.as_array());
  let Some(streams) = streams else { return Route::Transcode };
  let h264 = streams
    .iter()
    .any(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("video") && s.get("codec_name").and_then(|c| c.as_str()) == Some("h264"));
  let audio_ok = streams
    .iter()
    .filter(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("audio"))
    .all(|s| matches!(s.get("codec_name").and_then(|c| c.as_str()), Some("aac" | "mp3")));
  if h264 && audio_ok {
    Route::Direct
  } else {
    Route::Transcode
  }
}

/// 转码兜底：产物与 .stamp 一致直接复用（先查复用再查二进制——已有产物不需要 ffmpeg）。
/// root = 库根（快照值），只用于拼 CACHE_DIR/media，不吃整个 Library
fn transcode_source(root: &Path, asset_id: &str, origin: &Path, is_video: bool) -> Result<MediaSource, String> {
  let out_dir = root.join(CACHE_DIR).join("media");
  let out = out_dir.join(format!("media-{asset_id}.{}", if is_video { "mp4" } else { "wav" }));
  let stamp = stamp_path(&out);
  if out.is_file() && source_stamp(origin).is_some_and(|s| read_stamp(&stamp) == Some(s)) {
    return Ok(MediaSource { path: out.to_string_lossy().into_owned(), transcoded: true, fps: None });
  }
  if !ffmpeg::ffmpeg_available() {
    return Err("音视频组件缺失，无法播放该格式".to_string());
  }
  fs::create_dir_all(&out_dir).map_err(|e| format!("转码缓存目录创建失败：{e}"))?;
  let _ = fs::remove_file(&out); // 先清旧产物：防转码失败留下半截文件被下次复用
  if !ffmpeg::transcode_for_webview(origin, &out, !is_video) {
    return Err("该格式暂时无法播放".to_string());
  }
  if let Some(s) = source_stamp(origin) {
    if let Err(e) = fs::write(&stamp, s) {
      log::warn!("转码戳写入失败（{}）：{e}", stamp.display());
    }
  }
  Ok(MediaSource { path: out.to_string_lossy().into_owned(), transcoded: true, fps: None })
}

// ---------- 波形图按需重采样 ----------

/// 波形图渲染尺寸钳制范围（防前端异常值）：宽 32~7680、高 8~1080
const WAVE_W: std::ops::RangeInclusive<u32> = 32..=7680;
const WAVE_H: std::ops::RangeInclusive<u32> = 8..=1080;

/// 按需重采样波形图结果
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WaveformImage {
  /// 重采样波形图绝对路径（cache/waveforms/{assetId}_{宽}x{高}.webp）
  pub path: String,
}

/// 按需重采样波形图（2026-07-26 波形清晰度修复）：前端按播放条 / 预览大图实际显示像素
/// （CSS px × devicePixelRatio）请求，后端按该尺寸重渲 PCM 波形（渲染口径同 5.1 定稿，
/// 任意宽度 1px 级清晰，对齐 Eagle），产物缓存 cache/waveforms/（会话级，开库清空），
/// 已存在直给不重渲。非音频资源中文报错；ffmpeg 缺失 / 提取失败给中文文案（前端回退
/// info 缩略图拉伸旧行为）。短锁快照后重采样 / 落盘走锁外线程池（规范第 2 条）；
/// 产物按 宽x高 文件名幂等，无需并发去重
#[tauri::command]
pub async fn get_waveform(
  state: State<'_, AppState>,
  asset_id: String,
  width: u32,
  height: u32,
) -> Result<WaveformImage, String> {
  // 短锁快照：定位资源、克隆所需字段后立即放锁（原因同 get_media_source）
  let (origin, meta, root) = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
    (origin, meta, lib.root.clone())
  };
  tauri::async_runtime::spawn_blocking(move || waveform(&origin, &meta, &root, &asset_id, width, height))
    .await
    .map_err(|e| format!("波形图任务失败：{e}"))?
}

fn waveform(origin: &Path, meta: &AssetMeta, root: &Path, asset_id: &str, width: u32, height: u32) -> Result<WaveformImage, String> {
  if !media::is_audio(origin) {
    return Err(format!("「{}」不是音频资源", meta.name));
  }
  let (w, h) = clamp_wave_size(width, height);
  let dir = root.join(CACHE_DIR).join("waveforms");
  let out = dir.join(format!("{asset_id}_{w}x{h}.webp"));
  if out.is_file() {
    return Ok(WaveformImage { path: out.to_string_lossy().into_owned() });
  }
  if !ffmpeg::ffmpeg_available() {
    return Err("音视频组件缺失，无法生成波形图".to_string());
  }
  let Some(img) = media::waveform_image_sized(&origin, w, h) else {
    return Err("波形提取失败，请检查音频文件".to_string());
  };
  fs::create_dir_all(&dir).map_err(|e| format!("波形缓存目录创建失败：{e}"))?;
  thumbnail::save_webp(&image::DynamicImage::ImageRgba8(img), &out).map_err(|e| format!("波形图保存失败：{e}"))?;
  Ok(WaveformImage { path: out.to_string_lossy().into_owned() })
}

/// 波形图请求尺寸钳制（前端异常值不炸渲染：过小糊成一团、过大撑爆内存）
fn clamp_wave_size(w: u32, h: u32) -> (u32, u32) {
  (w.clamp(*WAVE_W.start(), *WAVE_W.end()), h.clamp(*WAVE_H.start(), *WAVE_H.end()))
}

/// 产物旁的戳文件：media-{assetId}.mp4.stamp
fn stamp_path(out: &Path) -> PathBuf {
  let mut s = out.as_os_str().to_os_string();
  s.push(".stamp");
  PathBuf::from(s)
}

/// 源文件戳："大小:mtime秒"（与产物 .stamp 比对判源是否变化）
fn source_stamp(origin: &Path) -> Option<String> {
  let meta = fs::metadata(origin).ok()?;
  let secs = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
  Some(format!("{}:{secs}", meta.len()))
}

fn read_stamp(stamp: &Path) -> Option<String> {
  fs::read_to_string(stamp).ok().map(|t| t.trim().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library::{self, Library};
  use crate::services::import::{self, PendingFile, Source};

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  /// 造一个单文件资源（假字节，只够走扩展名路由与 locate_asset）并登记进内存索引
  fn add_file(h: &mut Harness, id: &str, name: &str) {
    let src = h.dir.path().join(name);
    fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta);
  }

  /// 模拟命令入口的短锁快照：locate_asset + 克隆 lib.root（内部函数已不再吃 &Library）
  fn snap(h: &Harness, id: &str) -> (PathBuf, PathBuf, AssetMeta, PathBuf) {
    let (info_dir, origin, meta) = locate_asset(&h.lib, id).unwrap();
    (info_dir, origin, meta, h.lib.root.clone())
  }

  fn probe_json(streams: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "streams": streams })
  }

  #[test]
  fn clamp_wave_size_bounds() {
    assert_eq!(clamp_wave_size(1, 1), (32, 8));
    assert_eq!(clamp_wave_size(99999, 99999), (7680, 1080));
    assert_eq!(clamp_wave_size(1200, 48), (1200, 48));
  }

  #[test]
  fn waveform_rejects_non_audio_and_serves_cache_idempotently() {
    let mut h = make_lib();
    // 非音频资源中文报错
    add_file(&mut h, "AN-PIC", "参考图.png");
    let (_i, origin, meta, root) = snap(&h, "AN-PIC");
    let err = waveform(&origin, &meta, &root, "AN-PIC", 640, 272).unwrap_err();
    assert!(err.contains("不是音频资源"), "{err}");

    // 缓存命中直给（命中判定先于二进制检查：无 ffmpeg / 假字节也直接返回）
    add_file(&mut h, "AN-AU", "配乐.mp3");
    let out = h.lib.root.join(CACHE_DIR).join("waveforms").join("AN-AU_1200x48.webp");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    fs::write(&out, b"webp").unwrap();
    let (_i, origin, meta, root) = snap(&h, "AN-AU");
    let r = waveform(&origin, &meta, &root, "AN-AU", 1200, 48).unwrap();
    assert_eq!(r, WaveformImage { path: out.to_string_lossy().into_owned() });
    // 幂等：已存在不重写（mtime 不变）
    let mtime = fs::metadata(&out).unwrap().modified().unwrap();
    waveform(&origin, &meta, &root, "AN-AU", 1200, 48).unwrap();
    assert_eq!(fs::metadata(&out).unwrap().modified().unwrap(), mtime);

    // 未命中且源是假字节：组件缺失 / 提取失败 中文文案（按环境有无 ffmpeg 二选一）
    let err = waveform(&origin, &meta, &root, "AN-AU", 640, 272).unwrap_err();
    assert!(err.contains("组件缺失") || err.contains("波形提取失败"), "{err}");
  }

  #[test]
  fn route_audio_direct_only_mp3_wav() {
    assert_eq!(route("mp3", false, None), Route::Direct);
    assert_eq!(route("wav", false, None), Route::Direct);
    // wma 与探测缺失都走转码
    assert_eq!(route("wma", false, None), Route::Transcode);
    assert_eq!(route("mp4", false, None), Route::Transcode); // 非视频按音频判
  }

  #[test]
  fn route_video_direct_only_h264_mp4_with_webview_audio() {
    let h264_aac = probe_json(serde_json::json!([
      { "codec_type": "video", "codec_name": "h264" },
      { "codec_type": "audio", "codec_name": "aac" }
    ]));
    assert_eq!(route("mp4", true, Some(&h264_aac)), Route::Direct);
    assert_eq!(route("m4v", true, Some(&h264_aac)), Route::Direct);

    // mp3 音频流可直播；无音频流也可直播
    let h264_mp3 = probe_json(serde_json::json!([
      { "codec_type": "video", "codec_name": "h264" },
      { "codec_type": "audio", "codec_name": "mp3" }
    ]));
    assert_eq!(route("mp4", true, Some(&h264_mp3)), Route::Direct);
    let h264_only = probe_json(serde_json::json!([{ "codec_type": "video", "codec_name": "h264" }]));
    assert_eq!(route("mp4", true, Some(&h264_only)), Route::Direct);

    // 容器不对（mov/avi）、编码不对（hevc/mpeg4）、音频编码不对（ac3/opus）全转码
    assert_eq!(route("mov", true, Some(&h264_aac)), Route::Transcode);
    let hevc = probe_json(serde_json::json!([{ "codec_type": "video", "codec_name": "hevc" }]));
    assert_eq!(route("mp4", true, Some(&hevc)), Route::Transcode);
    let h264_ac3 = probe_json(serde_json::json!([
      { "codec_type": "video", "codec_name": "h264" },
      { "codec_type": "audio", "codec_name": "ac3" }
    ]));
    assert_eq!(route("mp4", true, Some(&h264_ac3)), Route::Transcode);
    // 探测失败（sidecar 缺失 / 文件损坏）保守转码
    assert_eq!(route("mp4", true, None), Route::Transcode);
  }

  #[test]
  fn media_source_rejects_non_av_and_direct_renders_mp3() {
    let mut h = make_lib();
    add_file(&mut h, "AN-PIC", "参考图.png");
    let (info_dir, origin, meta, root) = snap(&h, "AN-PIC");
    let err = media_source_heavy(&info_dir, &origin, meta, &root, "AN-PIC").unwrap_err();
    assert!(err.contains("不是音视频资源"), "{err}");

    // mp3 不看内容直渲（假字节也直给源路径）
    add_file(&mut h, "AN-AU", "配乐.mp3");
    let (info_dir, origin, meta, root) = snap(&h, "AN-AU");
    let (src, healed) = media_source_heavy(&info_dir, &origin, meta, &root, "AN-AU").unwrap();
    assert!(healed.is_none());
    assert_eq!(src, MediaSource { path: origin.to_string_lossy().into_owned(), transcoded: false, fps: None });
  }

  #[test]
  fn video_fps_self_heal_skips_silently_on_probe_failure() {
    // 存量视频 fps 自愈（5.3）：假字节探测失败 → 静默跳过，fps 保持 None 不改写 metadata，
    // 播放源解析照常走通（不阻塞播放，下次播放再试）
    let mut h = make_lib();
    add_file(&mut h, "AN-VID2", "片段.mp4");
    let (info_dir, origin, meta, root) = snap(&h, "AN-VID2");
    let out = root.join(CACHE_DIR).join("media").join("media-AN-VID2.mp4");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    fs::write(&out, b"transcoded").unwrap();
    fs::write(stamp_path(&out), source_stamp(&origin).unwrap()).unwrap();
    let (src, healed) = media_source_heavy(&info_dir, &origin, meta, &root, "AN-VID2").unwrap();
    assert!(src.transcoded);
    assert!(healed.is_none(), "探测失败不应产生自愈回写");
    let disk: crate::core::models::AssetMeta =
      crate::core::json_io::read_json(&h.lib.info_dir("AN-VID2").join("metadata.json")).unwrap();
    assert_eq!(disk.fps, None, "探测失败不应写入 fps");
  }

  #[test]
  fn transcode_reuses_stamped_product_and_fails_gracefully() {
    let mut h = make_lib();
    add_file(&mut h, "AN-VID", "片段.mp4");
    let (info_dir, origin, meta, root) = snap(&h, "AN-VID");

    // 预置产物 + 匹配戳：无 ffmpeg 也直接复用（复用判定先于二进制检查）
    let out = root.join(CACHE_DIR).join("media").join("media-AN-VID.mp4");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    fs::write(&out, b"transcoded").unwrap();
    fs::write(stamp_path(&out), source_stamp(&origin).unwrap()).unwrap();
    let (src, healed) = media_source_heavy(&info_dir, &origin, meta, &root, "AN-VID").unwrap();
    assert!(healed.is_none());
    assert_eq!(src, MediaSource { path: out.to_string_lossy().into_owned(), transcoded: true, fps: None });

    // 源变化（大小变了戳失配）→ 重转：假 mp4 转码失败给中文文案；无 ffmpeg 则组件缺失文案
    fs::write(&origin, b"fake-changed-longer").unwrap();
    let (info_dir, origin, meta, root) = snap(&h, "AN-VID");
    let err = media_source_heavy(&info_dir, &origin, meta, &root, "AN-VID").unwrap_err();
    assert!(
      err.contains("该格式暂时无法播放") || err.contains("音视频组件缺失"),
      "{err}"
    );
  }
}
