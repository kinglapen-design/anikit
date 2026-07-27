//! ffmpeg / ffprobe sidecar 适配器（5.1 动图音视频底座，规范第 2 条）。
//! 二进制由 tools/fetch-ffmpeg.py 下载（BtbN win64 LGPL 静态构建），不入 git：
//! dev/test 在 src-tauri/binaries/{name}-x86_64-pc-windows-msvc.exe，打包后经 externalBin 放 exe 同目录。
//! 二进制缺失 / 执行失败一律返回 None（"能力探测"语义，同 thumbnail）——音视频退化为格式图标保底，
//! 导入不因此失败；只留 warn 日志。调用方都在后台线程（导入管线 / detail worker），同步阻塞可接受。

use std::path::{Path, PathBuf};
use std::process::Command;

/// Tauri externalBin 命名约定：{name}-{target triple}
const TRIPLE: &str = "x86_64-pc-windows-msvc";

/// Windows 隐藏控制台窗口标志（CREATE_NO_WINDOW）：sidecar 是控制台程序，不加会闪黑窗
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 音频 PCM 抽取的时长上限（秒）：8kHz mono f32 每分钟约 1.9MB，10 分钟封顶防超长文件撑内存
const PCM_MAX_SECS: u32 = 600;

/// sidecar 路径解析：dev = CARGO_MANIFEST_DIR/binaries（带 triple 后缀）；
/// prod = 当前 exe 同目录（externalBin 产物，带不带 triple 后缀都试）。找不到返回 None。
fn sidecar_path(name: &str) -> Option<PathBuf> {
  let mut candidates: Vec<PathBuf> = Vec::new();
  let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
  candidates.push(manifest.join("binaries").join(format!("{name}-{TRIPLE}.exe")));
  candidates.push(manifest.join("binaries").join(format!("{name}.exe")));
  if let Ok(exe) = std::env::current_exe() {
    if let Some(dir) = exe.parent() {
      candidates.push(dir.join(format!("{name}.exe")));
      candidates.push(dir.join(format!("{name}-{TRIPLE}.exe")));
    }
  }
  candidates.into_iter().find(|p| p.is_file())
}

/// 运行 sidecar 并取 stdout；二进制缺失 / 启动失败 / 非零退出 / stdout 为空均返回 None
fn run(name: &str, args: &[String]) -> Option<Vec<u8>> {
  let bin = sidecar_path(name)?;
  let mut cmd = Command::new(bin);
  cmd.args(args).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
  #[cfg(windows)]
  std::os::windows::process::CommandExt::creation_flags(&mut cmd, CREATE_NO_WINDOW);
  let out = match cmd.output() {
    Ok(o) => o,
    Err(e) => {
      log::warn!("{name} 启动失败：{e}");
      return None;
    }
  };
  if !out.status.success() {
    log::warn!("{name} 退出码 {:?}：{}", out.status.code(), String::from_utf8_lossy(&out.stderr).trim());
    return None;
  }
  if out.stdout.is_empty() {
    log::warn!("{name} 无输出");
    return None;
  }
  Some(out.stdout)
}

/// ffmpeg sidecar 是否可用：转码类命令据此区分"组件缺失"与"转码失败"两种中文文案
pub fn ffmpeg_available() -> bool {
  sidecar_path("ffmpeg").is_some()
}

/// 运行 sidecar 只判退出码（转码类调用输出写文件，stdout 无内容）；缺失 / 启动失败 / 非零退出均 false + warn
fn run_status(name: &str, args: &[String]) -> bool {
  let Some(bin) = sidecar_path(name) else {
    log::warn!("{name} sidecar 缺失");
    return false;
  };
  let mut cmd = Command::new(bin);
  cmd.args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
  #[cfg(windows)]
  std::os::windows::process::CommandExt::creation_flags(&mut cmd, CREATE_NO_WINDOW);
  match cmd.output() {
    Ok(o) if o.status.success() => true,
    Ok(o) => {
      log::warn!("{name} 退出码 {:?}：{}", o.status.code(), String::from_utf8_lossy(&o.stderr).trim());
      false
    }
    Err(e) => {
      log::warn!("{name} 启动失败：{e}");
      false
    }
  }
}

/// 是否含音频流（转码组参用：无音频流不带 -c:a）；探测失败视同无音频——转出无声视频仍可播
fn has_audio_stream(path: &Path) -> bool {
  probe_json(path)
    .and_then(|j| j.get("streams").and_then(|s| s.as_array()).cloned())
    .is_some_and(|streams| {
      streams.iter().any(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("audio"))
    })
}

/// 转码为 webview 可播格式（5.2 播放源兜底）：视频 → mp4（**libopenh264** + faststart，
/// 有音频流才带 aac）；音频 → wav（pcm_s16le）。二进制缺失 / 非零退出 / 产物未生成均 false + warn。
/// 编码器说明（2026-07-27 实测定位）：sidecar 是 LGPL 构建，**没有 libx264**（x264 是 GPL）——
/// 转码失败报"该格式暂时无法播放"的根因；libopenh264（Cisco，BSD）是 LGPL 构建内可用的
/// h264 软编码器，无许可传染、跨机器一致（h264_mf 依赖 Windows 组件，不作首选）。
/// openh264 不支持 x264 的 crf/preset，用码率控制（1080p 动画素材 3M 预览足够）。
pub fn transcode_for_webview(path: &Path, out_path: &Path, is_audio: bool) -> bool {
  let src = path.to_string_lossy().into_owned();
  let out = out_path.to_string_lossy().into_owned();
  let args: Vec<String> = if is_audio {
    vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), src, "-vn".into(), "-c:a".into(), "pcm_s16le".into(), out]
  } else {
    let mut a = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), src, "-map".into(), "0:v:0".into()];
    if has_audio_stream(path) {
      a.extend(["-map".into(), "0:a:0".into(), "-c:a".into(), "aac".into()]);
    }
    a.extend([
      "-c:v".into(), "libopenh264".into(), "-b:v".into(), "3M".into(),
      "-movflags".into(), "+faststart".into(), out,
    ]);
    a
  };
  if !run_status("ffmpeg", &args) {
    return false;
  }
  if out_path.is_file() {
    true
  } else {
    log::warn!("转码产物未生成：{}", out_path.display());
    false
  }
}

/// ffprobe 元数据（format + streams，JSON）；解析失败返回 None
pub fn probe_json(path: &Path) -> Option<serde_json::Value> {
  let args = vec![
    "-v".into(),
    "quiet".into(),
    "-print_format".into(),
    "json".into(),
    "-show_format".into(),
    "-show_streams".into(),
    path.to_string_lossy().into_owned(),
  ];
  let out = run("ffprobe", &args)?;
  serde_json::from_slice(&out).ok()
}

/// ffmpeg 抽视频单帧为 PNG 字节（at_secs 快速seek，-ss 在 -i 前）；抽帧失败返回 None
pub fn video_frame_png(path: &Path, at_secs: f64) -> Option<Vec<u8>> {
  let args = vec![
    "-v".into(),
    "error".into(),
    "-ss".into(),
    format!("{at_secs:.3}"),
    "-i".into(),
    path.to_string_lossy().into_owned(),
    "-frames:v".into(),
    "1".into(),
    "-f".into(),
    "image2pipe".into(),
    "-vcodec".into(),
    "png".into(),
    "-".into(),
  ];
  run("ffmpeg", &args)
}

/// 长视频抽帧只解关键帧的时长阈值（秒）：>30s 加 -skip_frame nokey 只解关键帧、fps 滤镜复制补齐
/// （2026-07-28 实测 185s/303MB mp4：全片解码 3.9s → 关键帧 0.95s，4 倍提速、CPU 大降）；
/// 短视频关键帧太少（可能仅 1 个）会退化为全同帧，30s 内仍全片解码（本就零点几秒）
const SKIP_KEYFRAME_MIN_SECS: f64 = 30.0;

/// ffmpeg 全片均匀抽预览帧序列（5.3 视频悬停 scrub，Eagle 帧图方案——鼠标移动只切图零解码延迟）：
/// fps 滤镜（fps = count/duration_secs 均匀抽 count 帧）+ scale=-2:{height}（高恒 height 宽等比），
/// 输出 out_dir/f-%03d.webp；**必须显式 `-c:v libwebp`**——.webp 扩展名默认走原生单帧 webp 编码器，
/// image2 序列模式 40 帧全覆盖到同一个 f-001.webp（2026-07-28 实测定位：png/显式 libwebp 均正常 40 帧）；
/// 成功返回实际生成文件清单（按名排序，帧数随浮点 ±1），无帧生成 / 失败返回 None。
/// 调用方负责目录创建清理与幂等戳；同步阻塞（调用方在线程池，同模块口径）
pub fn video_scrub_frames(path: &Path, out_dir: &Path, duration_secs: f64, count: u32, height: u32) -> Option<Vec<PathBuf>> {
  if duration_secs <= 0.0 || count == 0 {
    return None;
  }
  let fps = count as f64 / duration_secs;
  let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into()];
  if duration_secs > SKIP_KEYFRAME_MIN_SECS {
    // 输入选项（必须在 -i 前）：只解关键帧，fps 滤镜复制补齐均匀帧——
    // scrub 定位用关键帧画面足够，精度退化为 GOP 级（动画素材通常 2-5 秒）
    args.extend(["-skip_frame".into(), "nokey".into()]);
  }
  args.extend([
    "-i".into(),
    path.to_string_lossy().into_owned(),
    "-map".into(),
    "0:v:0".into(),
    "-vf".into(),
    format!("fps={fps:.6},scale=-2:{height}"),
    "-c:v".into(),
    "libwebp".into(),
    "-quality".into(),
    "80".into(),
    out_dir.join("f-%03d.webp").to_string_lossy().into_owned(),
  ]);
  if !run_status("ffmpeg", &args) {
    return None;
  }
  let mut files: Vec<PathBuf> = std::fs::read_dir(out_dir)
    .ok()?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| p.is_file() && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("f-")))
    .collect();
  files.sort();
  if files.is_empty() { None } else { Some(files) }
}

/// ffmpeg 解音频为 f32le PCM（sample_rate Hz，时长封顶 PCM_MAX_SECS）；
/// channels = 声道数（1 = 混音 mono，2 = 立体声交错 LRLR…）；返回逐样本 f32；解码失败返回 None
pub fn audio_pcm_f32(path: &Path, sample_rate: u32, channels: u16) -> Option<Vec<f32>> {
  let args = vec![
    "-v".into(),
    "error".into(),
    "-i".into(),
    path.to_string_lossy().into_owned(),
    "-t".into(),
    PCM_MAX_SECS.to_string(),
    "-ac".into(),
    channels.to_string(),
    "-ar".into(),
    sample_rate.to_string(),
    "-f".into(),
    "f32le".into(),
    "-acodec".into(),
    "pcm_f32le".into(),
    "-".into(),
  ];
  let bytes = run("ffmpeg", &args)?;
  let samples: Vec<f32> = bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
  if samples.is_empty() {
    return None;
  }
  Some(samples)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sidecar_path_prefers_manifest_binaries() {
    // 本仓库 dev 环境：下载后命中 binaries/ 带 triple 后缀；未下载的机器上允许 None（测试不强制）
    if let Some(p) = sidecar_path("ffmpeg") {
      assert!(p.file_name().unwrap().to_string_lossy().starts_with("ffmpeg"));
    }
  }

  #[test]
  fn run_returns_none_for_missing_binary() {
    // 不存在的 sidecar 名必然解析不到路径 → None，不 panic
    assert_eq!(run("definitely-not-a-real-sidecar", &[]), None);
  }

  /// 需要真二进制 + 真媒体文件：cargo test -- --ignored ffmpeg_probe --nocapture
  #[test]
  #[ignore]
  fn ffmpeg_probe_smoke() {
    // 手动验证：把任意 mp4 路径填进来跑
    let sample = std::env::var("ANIKIT_TEST_VIDEO").expect("设 ANIKIT_TEST_VIDEO=某 mp4 路径");
    let json = probe_json(Path::new(&sample)).expect("ffprobe 应成功");
    assert!(json.get("format").is_some());
    let png = video_frame_png(Path::new(&sample), 0.0).expect("抽帧应成功");
    assert!(png.starts_with(b"\x89PNG"));
  }

  /// 需要真二进制 + 真媒体文件：cargo test -- --ignored ffmpeg_transcode --nocapture
  #[test]
  #[ignore]
  fn ffmpeg_transcode_smoke() {
    let sample = std::env::var("ANIKIT_TEST_VIDEO").expect("设 ANIKIT_TEST_VIDEO=某 mp4 路径");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.mp4");
    assert!(transcode_for_webview(Path::new(&sample), &out, false), "视频转码应成功");
    assert!(out.is_file());
    if let Ok(a) = std::env::var("ANIKIT_TEST_AUDIO") {
      let wav = dir.path().join("out.wav");
      assert!(transcode_for_webview(Path::new(&a), &wav, true), "音频转码应成功");
      assert!(wav.is_file());
    }
  }
}
