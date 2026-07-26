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
}
