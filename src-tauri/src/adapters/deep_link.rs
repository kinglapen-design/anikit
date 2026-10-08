//! anikit:// 系统协议注册（2026-10-08 deep-link 复活，PRD 02-09）：Windows 注册表 HKCU
//! \Software\Classes\anikit（免提权、portable 绿色零管理员）——外部软件（Obsidian 等）点击
//! anikit:// 链接 = 系统拉起 exe 带 URL 参数，单实例插件回调转发运行中实例（lib.rs），
//! **零浏览器标签页**（http /open 端点的整窗标签代价消除）。
//! 废案背景：2026-08-21 定案曾规划系统注册，当日改定 http open 端点替代；实机后浏览器标签
//! 堆积干扰坐实，且单实例插件回调本就携带命令行参数（基础设施现成）——废案复活。
//! http /open 端点保留：AI 客户端只认 http 链接 + 未注册环境兜底。
//! 启动时 ensure_registered 幂等覆写（exe 漂移自愈：portable 换目录后指向最新 exe）。

use std::sync::{Mutex, OnceLock};

use crate::core::anikit_ref;

/// 从命令行参数解析 anikit 引用：首个合法 anikit://item|folder/{id} 参数 → (kind, id)
pub fn parse_args(args: &[String]) -> Option<(String, String)> {
  args.iter().find_map(|a| anikit_ref::parse_ref(a))
}

/// 冷启动待消费链接：进程由协议链接拉起（AniKit 未在跑）时，setup 存此——前端就绪 + 库打开后
/// 经 take_pending_deep_link 命令取走跳转（initMediaOpenListener 消费）
static PENDING: OnceLock<Mutex<Option<(String, String)>>> = OnceLock::new();

fn pending_slot() -> &'static Mutex<Option<(String, String)>> {
  PENDING.get_or_init(|| Mutex::new(None))
}

/// 存冷启动链接（setup 调用；单实例场景走回调 emit 不经此）
pub fn set_pending(r: (String, String)) {
  if let Ok(mut g) = pending_slot().lock() {
    *g = Some(r);
  }
}

/// 取走冷启动链接（一次性消费；take 语义防重复跳转）
pub fn take_pending() -> Option<(String, String)> {
  pending_slot().lock().ok()?.take()
}

/// 启动时注册 / 修复 anikit:// 协议（失败仅日志不阻断启动）
pub fn ensure_registered() {
  #[cfg(windows)]
  {
    let exe = std::env::current_exe();
    match exe {
      Ok(path) => {
        if let Err(e) = imp::ensure_registered(&path) {
          log::warn!("anikit:// 协议注册失败：{e}");
        }
      }
      Err(e) => log::warn!("anikit:// 协议注册跳过（取不到 exe 路径）：{e}"),
    }
  }
}

#[cfg(windows)]
mod imp {
  use windows::core::{HSTRING, PCWSTR};
  use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPEN_CREATE_OPTIONS, REG_SZ, RegCloseKey, RegCreateKeyExW, RegSetValueExW,
  };

  const CLASSES_KEY: &str = r"Software\Classes\anikit";

  fn create_key(path: &str) -> Result<HKEY, String> {
    let mut hkey = HKEY::default();
    let subkey = HSTRING::from(path);
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
    if r.is_ok() { Ok(hkey) } else { Err(format!("注册表打开失败（{path}）：{}", r.0)) }
  }

  /// 写 REG_SZ 值（name = None 写默认值）；UTF-16 含 NUL 终止
  fn write_str(hkey: HKEY, name: Option<&str>, value: &str) -> Result<(), String> {
    let name_ws = name.map(HSTRING::from);
    let name_ptr = name_ws.as_ref().map_or(PCWSTR::null(), |h| PCWSTR(h.as_ptr()));
    let utf16: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = unsafe { std::slice::from_raw_parts(utf16.as_ptr() as *const u8, utf16.len() * 2) };
    let r = unsafe { RegSetValueExW(hkey, name_ptr, None, REG_SZ, Some(bytes)) };
    if r.is_ok() { Ok(()) } else { Err(format!("注册表写入失败：{}", r.0)) }
  }

  /// 注册/修复 anikit://：HKCU\Software\Classes\anikit 三件套——
  /// (Default) = "URL:AniKit Protocol" / "URL Protocol" = "" / shell\open\command = exe "%1"
  pub fn ensure_registered(exe: &std::path::Path) -> Result<(), String> {
    let cmd = format!("\"{}\" \"%1\"", exe.to_string_lossy());
    let root = create_key(CLASSES_KEY)?;
    let r1 = write_str(root, None, "URL:AniKit Protocol");
    let r2 = write_str(root, Some("URL Protocol"), "");
    let _ = unsafe { RegCloseKey(root) };
    r1.and(r2)?;
    let command = create_key(&format!(r"{CLASSES_KEY}\shell\open\command"))?;
    let r3 = write_str(command, None, &cmd);
    let _ = unsafe { RegCloseKey(command) };
    r3
  }
}
