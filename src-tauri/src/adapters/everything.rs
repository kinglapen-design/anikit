//! Everything 搜索 IPC 适配器（工作空间搜索 2026-10-03，PRD 02「搜索」双层架构主层）：
//! 手写 1.4 WM_COPYDATA QUERY2 协议——不引 Everything64.dll、不用 es.exe，零分发授权问题。
//! - 探测 = FindWindow 任务栏通知窗（1.5a 类名优先、回落 1.4）+ WM_USER 版本查询；
//!   未运行再看注册表卸载项 / 默认安装路径区分「未安装 / 已安装未运行」（代拉起依据）
//! - 查询 = QUERY2（dwData 18）：自建 message-only 接收窗 + ChangeWindowMessageFilterEx
//!   放行 WM_COPYDATA（UIPI），SendMessageTimeout 发出，本线程消息泵等回包（超时降级）
//!
//! 协议参照 = voidtools Everything_IPC.h + Windhawk better-file-sizes 生产实证模块；
//! 真机验证 = tools/everything-ipc-probe.py（1.4.1.1030：44.8 万条全量回包约 1.1 秒含解析）。
//! 工程口径：全部同步阻塞，调用方一律 spawn_blocking（踩坑 074）；
//! **搜索串绝不可整体加引号**（引号 = 短语字面搜索，ext:/path: 等修饰符全失效——探针实录）。

use std::path::PathBuf;

/// 单条命中：绝对路径 + 目录标记 + 大小 / 修改时间（Everything 索引值；未索引 = None）
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
  pub path: String,
  pub is_dir: bool,
  /// 文件大小（字节）；文件夹未索引大小时 Everything 回 -1 → None
  pub size: Option<u64>,
  /// 修改时间（Unix 毫秒）；索引缺失 = None
  pub mtime_ms: Option<i64>,
}

/// 能力三态（ffmpeg sidecar 降级范式）：运行中（含版本）/ 已安装未运行（含 exe 路径，可代拉起）/ 未安装
#[derive(Debug, Clone, PartialEq)]
pub enum EverythingStatus {
  Running { version: (u32, u32, u32, u32) },
  InstalledNotRunning { exe: Option<PathBuf> },
  NotInstalled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryError {
  /// Everything 未运行（接收窗找不到）
  NotRunning,
  /// 版本过低（QUERY2 需要 1.4.1+）
  UnsupportedVersion,
  /// 发出或等回包超时
  Timeout,
  /// 其它 IPC 失败（窗口创建 / 发送被拒等）
  Failed(String),
}

/// 能力探测（轻量：FindWindow + 最多一次注册表读；失败/缺失不抛错，调用方按降级处理）
pub fn detect() -> EverythingStatus {
  imp::detect()
}

/// 同步查询（**必须**在 spawn_blocking 等阻塞容忍线程调用——内含窗口消息泵等待）。
/// search = Everything 原生查询串（原样透传，**勿整体加引号**）；max_results 截断上限。
pub fn query(search: &str, max_results: u32) -> Result<Vec<Hit>, QueryError> {
  imp::query(search, max_results)
}

// ---------- Windows 实现 ----------
#[cfg(windows)]
mod imp {
  use super::{EverythingStatus, Hit, QueryError};
  use std::path::{Path, PathBuf};
  use std::sync::atomic::{AtomicU32, Ordering};
  use std::sync::Mutex;
  use std::time::{Duration, Instant};

  use windows::core::{w, PCWSTR};
  use windows::Win32::Foundation::{ERROR_CLASS_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
  use windows::Win32::System::DataExchange::COPYDATASTRUCT;
  use windows::Win32::System::LibraryLoader::GetModuleHandleW;
  use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW};
  use windows::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW,
    MSG, MSGFLT_ALLOW, PeekMessageW, PM_REMOVE, RegisterClassExW, SendMessageTimeoutW, TranslateMessage,
    HWND_MESSAGE, SMTO_ABORTIFHUNG, SMTO_BLOCK, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COPYDATA, WM_USER, WNDCLASSEXW,
  };

  /// 1.4 任务栏通知窗类名；1.5a 实例窗口类名独立（Windhawk 参照：先 1.5a 后 1.4）
  const WNDCLASS_14: PCWSTR = w!("EVERYTHING_TASKBAR_NOTIFICATION");
  const WNDCLASS_15A: PCWSTR = w!("EVERYTHING_TASKBAR_NOTIFICATION_(1.5a)");
  /// 本进程回包接收窗类名
  const REPLY_CLASS: PCWSTR = w!("anikit-everything-reply");
  /// QUERY2 的 COPYDATA dwData；QUERY1 为 2（本实现只用 QUERY2——要带大小 / 时间回包）
  const COPYDATA_QUERY2W: usize = 18;
  /// QUERY2 请求项：完整路径 + 大小 + 修改时间（值 = Everything_IPC.h）
  const REQ_FULL_PATH_AND_NAME: u32 = 0x04;
  const REQ_SIZE: u32 = 0x10;
  const REQ_DATE_MODIFIED: u32 = 0x40;
  /// QUERY2 sort：名称升序
  const SORT_NAME_ASCENDING: u32 = 1;
  /// WM_USER + 0..3 = 版本号四段查询（Everything_IPC.h）
  const IPC_GET_MAJOR: usize = 0;
  /// 发送 / 回包超时（Everything 常态毫秒级；10 秒 = 病态降级线）
  const TIMEOUT_MS: u32 = 10_000;
  /// item flags：文件夹位（文件 = 0x00——实测 1.4.1.1030 文件不带 FILE 位，只认 FOLDER 位）
  const ITEM_FLAG_FOLDER: u32 = 0x01;

  /// 查询串行锁（Everything 对每个来源窗口串行处理；进程内单飞行防回包串台）
  static QUERY_LOCK: Mutex<()> = Mutex::new(());
  /// 回包消息 id 计数（每次查询唯一，回包 COPYDATASTRUCT.dwData 据此对号）
  static REPLY_ID: AtomicU32 = AtomicU32::new(1);

  pub fn detect() -> EverythingStatus {
    if let Some(hwnd) = find_window() {
      let v = query_version(hwnd).unwrap_or((0, 0, 0, 0));
      return EverythingStatus::Running { version: v };
    }
    match find_installed_exe() {
      Some(exe) => EverythingStatus::InstalledNotRunning { exe: Some(exe) },
      None => EverythingStatus::NotInstalled,
    }
  }

  pub fn query(search: &str, max_results: u32) -> Result<Vec<Hit>, QueryError> {
    let _guard = QUERY_LOCK.lock().map_err(|e| QueryError::Failed(format!("查询锁异常：{e}")))?;
    let hwnd = find_window().ok_or(QueryError::NotRunning)?;
    if let Some(v) = query_version(hwnd) {
      if v.0 < 1 || (v.0 == 1 && (v.1 < 4 || (v.1 == 4 && v.2 < 1))) {
        return Err(QueryError::UnsupportedVersion); // QUERY2 需 1.4.1+
      }
    } // 版本查不到不硬拒（窗口在 = 大概率可用），查询自身超时兜底
    query_roundtrip(hwnd, search, max_results)
  }

  /// 找 Everything 窗口（1.5a 优先）
  fn find_window() -> Option<HWND> {
    unsafe {
      for class in [WNDCLASS_15A, WNDCLASS_14] {
        // FindWindow 找不到返回 NULL；windows crate 对部分版本包装成 Err（GetLastError=0），两态都算未找到
        if let Ok(h) = FindWindowW(class, PCWSTR::null()) {
          if !h.is_invalid() {
            return Some(h);
          }
        }
      }
      None
    }
  }

  /// 版本查询：SendMessage(WM_USER, 0..3)；超时 / 失败 = None（不据此判死）
  fn query_version(hwnd: HWND) -> Option<(u32, u32, u32, u32)> {
    let mut out = [0u32; 4];
    for (i, slot) in out.iter_mut().enumerate() {
      let mut got = 0usize;
      let r = unsafe {
        SendMessageTimeoutW(hwnd, WM_USER, WPARAM(IPC_GET_MAJOR + i), LPARAM(0), SMTO_ABORTIFHUNG, 500, Some(&mut got))
      };
      if r.0 == 0 {
        return None;
      }
      *slot = got as u32;
    }
    Some((out[0], out[1], out[2], out[3]))
  }

  /// 注册表 / 默认路径找安装：卸载项 InstallLocation（HKCU 优先，Everything 支持免管理员装 HKCU）+
  /// 默认 Program Files 路径兜底
  fn find_installed_exe() -> Option<PathBuf> {
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
      if let Some(dir) = read_reg_string(root, w!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Everything"), w!("InstallLocation")) {
        let exe = Path::new(&dir).join("Everything.exe");
        if exe.is_file() {
          return Some(exe);
        }
      }
    }
    let fallback = PathBuf::from(r"C:\Program Files\Everything\Everything.exe");
    if fallback.is_file() {
      return Some(fallback);
    }
    None
  }

  fn read_reg_string(root: HKEY, sub: PCWSTR, value: PCWSTR) -> Option<String> {
    unsafe {
      let mut key = HKEY::default();
      if RegOpenKeyExW(root, sub, None, KEY_READ, &mut key) != ERROR_SUCCESS {
        return None;
      }
      let mut len = 0u32;
      // 先问长度再问值（含终止 null；按 WCHAR 缓冲）
      if RegQueryValueExW(key, value, None, None, None, Some(&mut len)) != ERROR_SUCCESS || len == 0 {
        let _ = RegCloseKey(key);
        return None;
      }
      let mut buf = vec![0u16; (len as usize).div_ceil(2) + 1];
      let ok = RegQueryValueExW(key, value, None, None, Some(buf.as_mut_ptr() as *mut u8), Some(&mut len));
      let _ = RegCloseKey(key);
      if ok != ERROR_SUCCESS {
        return None;
      }
      let s = String::from_utf16_lossy(&buf);
      let s = s.trim_end_matches('\0').to_string();
      if s.is_empty() { None } else { Some(s) }
    }
  }

  /// 回包登记处（id → 字节整份拷贝——WM_COPYDATA 契约：lpData 仅在消息处理期有效）。
  /// 走 Mutex 而非 GWLP_USERDATA 挂栈对象：**回包由 Everything 线程 SendMessage 触发、在
  /// SendMessageTimeout 阻塞期内经本线程 WndProc 落笔**——栈对象 + 裸指针直写在优化器
  /// noalias 视角下不可见，等包循环的读会被提升出循环成死等（live_query_smoke 实录）
  static REPLIES: Mutex<Vec<(u32, Vec<u8>)>> = Mutex::new(Vec::new());

  /// QUERY2 查询往返：建接收窗 → 发包 → 本线程消息泵等回包 → 解析
  fn query_roundtrip(hwnd: HWND, search: &str, max_results: u32) -> Result<Vec<Hit>, QueryError> {
    let receiver = create_reply_window().map_err(QueryError::Failed)?;
    let reply_id = REPLY_ID.fetch_add(1, Ordering::Relaxed);

    let result = query_send_and_wait(hwnd, receiver, reply_id, search, max_results);

    unsafe {
      let _ = DestroyWindow(receiver);
    }
    result
  }

  fn query_send_and_wait(hwnd: HWND, receiver: HWND, reply_id: u32, search: &str, max_results: u32) -> Result<Vec<Hit>, QueryError> {
    // QUERY2 头 = 7×u32（reply_hwnd 截 32 位——窗口句柄 64 位系统也保证低 32 位有效，协议如此）
    let mut packet = Vec::with_capacity(28 + (search.len() + 1) * 2);
    let push = |v: u32, packet: &mut Vec<u8>| packet.extend_from_slice(&v.to_le_bytes());
    push(receiver.0 as usize as u32, &mut packet);
    push(reply_id, &mut packet);
    push(0, &mut packet); // search_flags = 0（不区分大小写 / 非整词 / 关键字只匹配名——path: 等显式语法不受影响）
    push(0, &mut packet); // offset
    push(max_results, &mut packet);
    push(REQ_FULL_PATH_AND_NAME | REQ_SIZE | REQ_DATE_MODIFIED, &mut packet);
    push(SORT_NAME_ASCENDING, &mut packet);
    for u in search.encode_utf16() {
      packet.extend_from_slice(&u.to_le_bytes());
    }
    packet.extend_from_slice(&0u16.to_le_bytes()); // 终止 null

    let cds = COPYDATASTRUCT { dwData: COPYDATA_QUERY2W, cbData: packet.len() as u32, lpData: packet.as_ptr() as *mut core::ffi::c_void };
    let sent = unsafe {
      SendMessageTimeoutW(hwnd, WM_COPYDATA, WPARAM(receiver.0 as usize), LPARAM(&cds as *const COPYDATASTRUCT as isize), SMTO_BLOCK | SMTO_ABORTIFHUNG, TIMEOUT_MS, None)
    };
    if sent.0 == 0 {
      return Err(QueryError::Failed(format!("发送超时或被拒（GetLastError={:?}）", unsafe { GetLastError() })));
    }

    // 本线程消息泵等回包（回包 = Everything 线程 SendMessage 进接收窗，阻塞期内即触发 WndProc）
    let deadline = Instant::now() + Duration::from_millis(TIMEOUT_MS as u64);
    let mut msg = MSG::default();
    let buf = loop {
      if let Some(buf) = REPLIES.lock().ok().and_then(|mut m| m.iter().position(|(id, _)| *id == reply_id).map(|i| m.swap_remove(i).1)) {
        break buf;
      }
      if Instant::now() >= deadline {
        return Err(QueryError::Timeout);
      }
      unsafe {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
          let _ = TranslateMessage(&msg);
          DispatchMessageW(&msg);
        }
      }
      std::thread::sleep(Duration::from_millis(1));
    };
    parse_list2(&buf)
  }

  fn create_reply_window() -> Result<HWND, String> {
    unsafe {
      let hinstance: HINSTANCE = GetModuleHandleW(None).map_err(|e| format!("模块句柄获取失败：{e}"))?.into();
      let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(reply_wnd_proc),
        hInstance: hinstance,
        lpszClassName: REPLY_CLASS,
        ..Default::default()
      };
      if RegisterClassExW(&wc) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
        return Err(format!("回包窗口类注册失败：{:?}", GetLastError()));
      }
      let hwnd = CreateWindowExW(WINDOW_EX_STYLE(0), REPLY_CLASS, w!(""), WINDOW_STYLE(0), 0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(hinstance), None)
        .map_err(|e| format!("回包窗口创建失败：{e}"))?;
      // UIPI：Everything 以更低完整性运行时需要显式放行 WM_COPYDATA（Windhawk 同款）
      let _ = ChangeWindowMessageFilterEx(hwnd, WM_COPYDATA, MSGFLT_ALLOW, None);
      Ok(hwnd)
    }
  }

  /// 回包 WndProc：dwData（= 我方 reply_id）对号即整份拷贝进登记处；其余一律 DefWindowProcW
  unsafe extern "system" fn reply_wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_COPYDATA {
      let cds = &*(l.0 as *const COPYDATASTRUCT);
      if !cds.lpData.is_null() {
        if let Ok(mut m) = REPLIES.lock() {
          m.push((cds.dwData as u32, std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize).to_vec()));
        }
      }
      return LRESULT(1);
    }
    DefWindowProcW(hwnd, msg, w, l)
  }

  // ---------- 回包解析（纯函数，单测可测） ----------

  /// LIST2 = 5×u32（totitems/numitems/offset/request_flags/sort_type）+ items[numitems]（flags+data_offset）
  /// + 数据区；item 数据位于 data_offset（相对 LIST2 起点），字段按请求项固定序：
  ///   full_path_and_name（u32 长 + UTF-16 串含 null）→ size（i64）→ date_modified（FILETIME）
  fn parse_list2(buf: &[u8]) -> Result<Vec<Hit>, QueryError> {
    if buf.len() < 20 {
      return Err(QueryError::Failed("回包过短（不足 LIST2 头 20 字节）".into()));
    }
    let rd = |at: usize| -> Option<u32> { buf.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())) };
    let num = rd(4).ok_or_else(|| QueryError::Failed("回包过短".into()))? as usize;
    let flags_echo = rd(12).unwrap_or(0);
    let mut out = Vec::with_capacity(num);
    for i in 0..num {
      let base = 20 + i * 8;
      let (Some(iflags), Some(data_off)) = (rd(base), rd(base + 4)) else { break };
      let mut p = data_off as usize;
      // full_path_and_name
      let path = if flags_echo & REQ_FULL_PATH_AND_NAME != 0 {
        let Some(len) = rd(p) else { break };
        p += 4;
        let chars = len as usize;
        let Some(bytes) = buf.get(p..p + (chars + 1) * 2) else { break };
        let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        if units.len() < chars {
          break; // 截断回包防御（协议正常不会发生）
        }
        p += (chars + 1) * 2;
        String::from_utf16_lossy(&units[..chars])
      } else {
        String::new()
      };
      // size（文件夹未索引 = -1 → None）
      let size = if flags_echo & REQ_SIZE != 0 {
        let Some(b) = buf.get(p..p + 8) else { break };
        p += 8;
        let v = i64::from_le_bytes(b.try_into().unwrap());
        if v >= 0 { Some(v as u64) } else { None }
      } else {
        None
      };
      // date_modified（FILETIME = 1601 起 100ns；0 = 未知 → None）
      let mtime_ms = if flags_echo & REQ_DATE_MODIFIED != 0 {
        let Some(b) = buf.get(p..p + 8) else { break };
        let ft = u64::from_le_bytes(b.try_into().unwrap());
        if ft >= 116_444_736_000_000_000 { Some(((ft - 116_444_736_000_000_000) / 10_000) as i64) } else { None }
      } else {
        None
      };
      if path.is_empty() {
        continue;
      }
      out.push(Hit { path, is_dir: iflags & ITEM_FLAG_FOLDER != 0, size, mtime_ms });
    }
    Ok(out)
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    /// 拼一条 LIST2 回包：两命中（文件 + 文件夹），字段序 = full_path → size → date_modified
    fn build_reply() -> Vec<u8> {
      let items: [(u32, &str, i64, u64); 2] = [
        (0, "E:\\ws\\图.png", 568, 134_000_000_000_000_000),
        (1, "E:\\ws\\子目录", -1, 134_000_000_000_000_000),
      ];
      let header = 20 + items.len() * 8;
      let mut data = Vec::new();
      let mut offsets = Vec::new();
      for (_, path, size, ft) in &items {
        offsets.push((header + data.len()) as u32);
        let wide: Vec<u16> = path.encode_utf16().collect();
        data.extend_from_slice(&(wide.len() as u32).to_le_bytes());
        for u in &wide {
          data.extend_from_slice(&u.to_le_bytes());
        }
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&size.to_le_bytes());
        data.extend_from_slice(&ft.to_le_bytes());
      }
      let mut buf = Vec::new();
      for v in [2u32, items.len() as u32, 0, REQ_FULL_PATH_AND_NAME | REQ_SIZE | REQ_DATE_MODIFIED, SORT_NAME_ASCENDING] {
        buf.extend_from_slice(&v.to_le_bytes());
      }
      for (i, (flags, _, _, _)) in items.iter().enumerate() {
        buf.extend_from_slice(&flags.to_le_bytes());
        buf.extend_from_slice(&offsets[i].to_le_bytes());
      }
      buf.extend_from_slice(&data);
      buf
    }

    #[test]
    fn parse_list2_reads_hits() {
      let hits = parse_list2(&build_reply()).unwrap();
      assert_eq!(hits.len(), 2);
      assert_eq!(hits[0].path, "E:\\ws\\图.png");
      assert!(!hits[0].is_dir);
      assert_eq!(hits[0].size, Some(568));
      assert!(hits[0].mtime_ms.is_some());
      assert!(hits[1].is_dir);
      assert_eq!(hits[1].size, None); // 文件夹 -1 → None
    }

    #[test]
    fn parse_list2_short_buffer_errs() {
      assert!(parse_list2(&[0u8; 8]).is_err());
      assert!(parse_list2(&[]).is_err());
    }

    /// 真机冒烟（需 Everything 运行中）：cargo test -p app everything -- --ignored
    #[test]
    #[ignore = "需本机运行 Everything"]
    fn live_query_smoke() {
      let hits = query("anikit", 5).expect("Everything 运行中应可查询");
      assert!(!hits.is_empty(), "anikit 关键字应命中（本机开发目录）");
    }
  }
}

// ---------- 非 Windows 占位（工作空间搜索一期只支持 Windows；探测恒未安装，查询恒不可用） ----------
#[cfg(not(windows))]
mod imp {
  use super::{EverythingStatus, Hit, QueryError};

  pub fn detect() -> EverythingStatus {
    EverythingStatus::NotInstalled
  }

  pub fn query(_search: &str, _max_results: u32) -> Result<Vec<Hit>, QueryError> {
    Err(QueryError::NotRunning)
  }
}
