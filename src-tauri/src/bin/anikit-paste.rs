//! anikit-paste：编辑器贴图直链小工具（2026-10-23 定案，PRD 05-01「编辑器贴图直链」/ 02-09 by-name 形态）。
//! Typora 自定义上传命令协议：argv = 图片文件路径列表 → 逐文件拷入最近储存空间的收件箱
//! （自动导入/，唯一时间戳名 贴图-yyyyMMdd-HHmmss-fff），stdout 每图一行回
//! http://127.0.0.1:{port}/item/by-name/{percent编码文件名} 直链，Typora 写进文档当场可渲。
//! AniKit 未开时文件躺收件箱（盘符卸载不影响真实目录），下次启动存量导入后链接即活。
//! 注意：不得加 windows_subsystem 属性——Typora 要读 stdout（控制台子系统默认即可）。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use app_lib::core::{anikit_ref, config, space};

/// config.json 最小镜像（只取 spaces[0].path = 最近打开的储存空间根；未知字段 serde 默认忽略，
/// 不回 pub 整个 AppConfig——bin 只读不写，字段漂移风险最小化）
#[derive(serde::Deserialize)]
struct ConfigMirror {
  #[serde(default)]
  spaces: Vec<SpaceMirror>,
}

#[derive(serde::Deserialize)]
struct SpaceMirror {
  path: String,
}

fn main() -> ExitCode {
  match run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(msg) => {
      eprintln!("anikit-paste：{msg}");
      ExitCode::FAILURE
    }
  }
}

fn run() -> Result<(), String> {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.is_empty() {
    return Err("用法：anikit-paste <图片文件路径> [更多路径...]（Typora 自定义上传命令协议）".into());
  }
  let inbox = inbox_dir()?;
  let port = read_port();
  for arg in &args {
    let src = Path::new(arg);
    if !src.is_file() {
      return Err(format!("文件不存在：{arg}"));
    }
    let dest = unique_target(&inbox, src);
    std::fs::copy(src, &dest).map_err(|e| format!("拷贝失败 {} → {}：{e}", src.display(), dest.display()))?;
    let name = dest.file_name().and_then(|n| n.to_str()).ok_or_else(|| "目标文件名含非法字符".to_string())?;
    println!("http://127.0.0.1:{port}/item/by-name/{}", anikit_ref::percent_encode(name));
  }
  Ok(())
}

/// 收件箱真实目录：%APPDATA%\AniKit\config.json 的 spaces[0].path（最近打开在前）+ 自动导入/；
/// 目录可能由本工具首建（AniKit 未开时），启动存量导入会收
fn inbox_dir() -> Result<PathBuf, String> {
  let cfg_path = config::config_path().map_err(|e| e.to_string())?;
  let text = std::fs::read_to_string(&cfg_path)
    .map_err(|_| format!("找不到配置文件 {}（请先运行 AniKit 完成初始设置）", cfg_path.display()))?;
  let cfg: ConfigMirror = serde_json::from_str(&text).map_err(|e| format!("配置文件解析失败：{e}"))?;
  let Some(first) = cfg.spaces.first() else {
    return Err("配置中还没有储存空间（请先在 AniKit 中打开一个储存空间）".into());
  };
  let inbox = PathBuf::from(&first.path).join(space::INBOX_DIR);
  std::fs::create_dir_all(&inbox).map_err(|e| format!("收件箱目录创建失败 {}：{e}", inbox.display()))?;
  Ok(inbox)
}

/// 媒体服务实际端口：config 同目录 mcp-server.port（内容 = 纯数字，启动覆写）；
/// 缺文件 / 解析失败回退固定端口 17701（AniKit 未开 / 首次启动前的常态）
fn read_port() -> u16 {
  let Ok(cfg_path) = config::config_path() else { return 17701 };
  let Ok(text) = std::fs::read_to_string(cfg_path.with_file_name("mcp-server.port")) else { return 17701 };
  text.trim().parse().unwrap_or(17701)
}

/// 唯一时间戳名：贴图-{yyyyMMdd}-{HHmmss}-{毫秒}.原扩展名（扩展名小写保留）；
/// 同毫秒撞名（一次粘贴多图）追加 -2 / -3 序号兜底
fn unique_target(inbox: &Path, src: &Path) -> PathBuf {
  let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
  let dot_ext = if ext.is_empty() { String::new() } else { format!(".{ext}") };
  let now = chrono::Local::now();
  let stem = format!("贴图-{}-{:03}", now.format("%Y%m%d-%H%M%S"), now.timestamp_subsec_millis());
  let mut candidate = inbox.join(format!("{stem}{dot_ext}"));
  let mut n = 2;
  while candidate.exists() {
    candidate = inbox.join(format!("{stem}-{n}{dot_ext}"));
    n += 1;
  }
  candidate
}
