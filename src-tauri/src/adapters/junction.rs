//! 目录链接（junction）适配器（工作空间「外部文件夹挂进 WS 子目录」方向 b）：
//! NTFS 挂载点 reparse point 三件套——创建 / 读目标 / 删除，加列目录高频用的快速判定。
//! 选 junction 不选符号链接：无需 SeCreateSymbolicLinkPrivilege（普通用户可建）、跨卷可用、
//! 对资源管理器透明。删除只动 reparse point 本体（RemoveDirectoryW 不递归目标内容，
//! 红线实测见本文件 spike 测试）。非 Windows 暂不支持（PRD 目标平台 Windows 优先）。

use std::path::{Path, PathBuf};

/// 目录 reparse point 快速判定（列目录逐目录条目调用）：symlink_metadata 不跟随链接，
/// 属性位 FILE_ATTRIBUTE_REPARSE_POINT + FILE_ATTRIBUTE_DIRECTORY 同真即 junction
///（含断链——断链后 reparse point 本体仍在）；纯 std 零 Win32 开销
pub fn is_reparse_dir(path: &Path) -> bool {
  imp::is_reparse_dir(path)
}

/// 创建 junction：link 处建挂载点指向 target（须已存在的目录绝对路径）
pub fn create(link: &Path, target: &Path) -> Result<(), String> {
  imp::create(link, target)
}

/// 读 junction 目标（断链也能读出——reparse point 数据是静态字节，不依赖目标存活；
/// 这是它区别于 canonicalize 的价值：断链链接仍能还回原目标供登记清单对账）
pub fn read_target(link: &Path) -> Option<PathBuf> {
  imp::read_target(link)
}

/// 删除 junction 本体（RemoveDirectoryW 只删 reparse point，不递归目标内容）
pub fn remove(link: &Path) -> Result<(), String> {
  imp::remove(link)
}

#[cfg(windows)]
mod imp {
  use std::os::windows::ffi::OsStrExt;
  use std::os::windows::fs::MetadataExt;
  use std::path::{Path, PathBuf};

  use windows::core::PCWSTR;
  use windows::Win32::Foundation::{CloseHandle, HANDLE};
  use windows::Win32::Storage::FileSystem::{
    CreateFileW, RemoveDirectoryW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
    FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
  };
  use windows::Win32::System::Ioctl::{FSCTL_GET_REPARSE_POINT, FSCTL_SET_REPARSE_POINT};
  use windows::Win32::System::IO::DeviceIoControl;

  /// FILE_ATTRIBUTE_REPARSE_POINT / FILE_ATTRIBUTE_DIRECTORY（属性位判定用，纯数值不加依赖面）
  const ATTR_REPARSE: u32 = 0x400;
  const ATTR_DIRECTORY: u32 = 0x10;
  /// IO_REPARSE_TAG_MOUNT_POINT（junction = 挂载点标签，非符号链接的 0xA000000C）
  const TAG_MOUNT_POINT: u32 = 0xA000_0003;
  /// REPARSE_DATA_BUFFER 固定头长度：ReparseTag u32 + ReparseDataLength u16 + Reserved u16
  const FIXED_HEADER: usize = 8;
  /// MountPoint 路径缓冲头长度：SubNameOffset/Length + PrintNameOffset/Length 各 u16
  const PATH_HEADER: usize = 8;
  /// 名串区起点 = 固定头 + 路径缓冲头（SubstituteNameOffset 等偏移相对 PathBuffer 起点计）
  const NAMES_BASE: usize = FIXED_HEADER + PATH_HEADER;
  /// 读取缓冲：Microsoft 文档 REPARSE_DATA_BUFFER 上限 16KB
  const MAX_BUF: u32 = 16 * 1024;
  const GENERIC_READ: u32 = 0x8000_0000;
  const GENERIC_WRITE: u32 = 0x4000_0000;

  pub(super) fn is_reparse_dir(path: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
    // 直接比属性位，不踩 std FileType::is_dir / is_symlink 对 junction 归类的版本差异
    let attrs = meta.file_attributes();
    attrs & ATTR_REPARSE != 0 && attrs & ATTR_DIRECTORY != 0
  }

  /// 打开链接本体句柄（BACKUP_SEMANTICS 开目录 + OPEN_REPARSE_POINT 不跟随——
  /// 读写的是 reparse point 本身而不是目标）
  fn open_link(link: &Path, access: u32) -> Result<HANDLE, String> {
    let wide: Vec<u16> = link.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
      CreateFileW(
        PCWSTR(wide.as_ptr()),
        access,
        FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0),
        None,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        None,
      )
      .map_err(|e| format!("链接句柄打开失败（{}）：{e}", link.display()))
    }
  }

  /// 手工拼 REPARSE_DATA_BUFFER（MountPoint 布局）字节流，不跟 windows crate 的结构体纠缠：
  /// [tag u32][dataLen u16][reserved u16][subOff u16][subLen u16][printOff u16][printLen u16]
  /// 随后 = substitute UTF-16LE + NUL + print UTF-16LE + NUL（偏移相对名串区起点计）
  fn build_mount_point_buffer(substitute: &[u16], print: &[u16]) -> Vec<u8> {
    fn put_u16(buf: &mut Vec<u8>, v: u16) {
      buf.extend_from_slice(&v.to_le_bytes());
    }
    let sub_bytes = substitute.len() * 2;
    let print_bytes = print.len() * 2;
    let data_len = (PATH_HEADER + sub_bytes + 2 + print_bytes + 2) as u16;
    let mut buf = Vec::with_capacity(FIXED_HEADER + data_len as usize);
    buf.extend_from_slice(&TAG_MOUNT_POINT.to_le_bytes());
    put_u16(&mut buf, data_len);
    put_u16(&mut buf, 0); // Reserved
    put_u16(&mut buf, 0); // SubstituteNameOffset
    put_u16(&mut buf, sub_bytes as u16); // SubstituteNameLength（不含 NUL）
    put_u16(&mut buf, (sub_bytes + 2) as u16); // PrintNameOffset = sub + NUL 之后
    put_u16(&mut buf, print_bytes as u16);
    for &u in substitute {
      put_u16(&mut buf, u);
    }
    put_u16(&mut buf, 0);
    for &u in print {
      put_u16(&mut buf, u);
    }
    put_u16(&mut buf, 0);
    buf
  }

  /// 普通绝对路径 → junction 目标两形态：substitute = `\??\` + 路径（内核命名空间形态，
  /// junction 写入的规范形）；print = 路径原样（资源管理器显示用）。
  /// canonicalize 在 Windows 产出 `\\?\` 逐字前缀（UNC 形 `\\?\UNC\s\sh`），剥掉后统一拼
  fn target_names(canon_target: &Path) -> (Vec<u16>, Vec<u16>) {
    let s = canon_target.as_os_str().to_string_lossy().replace('/', "\\");
    let stripped = s.strip_prefix(r"\\?\").unwrap_or(&s);
    let substitute = format!(r"\??\{stripped}");
    (substitute.encode_utf16().collect(), stripped.encode_utf16().collect())
  }

  pub(super) fn create(link: &Path, target: &Path) -> Result<(), String> {
    // canonicalize 消 .. / 短名 / 跟随中层 reparse——substitute 必须是最终真实路径，
    // 否则「链接指链接」会埋穿透与对账的坑
    let canon = target.canonicalize().map_err(|e| format!("链接目标不可用（{}）：{e}", target.display()))?;
    let (substitute, print) = target_names(&canon);
    let buf = build_mount_point_buffer(&substitute, &print);
    std::fs::create_dir(link).map_err(|e| format!("链接目录创建失败（{}）：{e}", link.display()))?;
    let handle = match open_link(link, GENERIC_READ | GENERIC_WRITE) {
      Ok(h) => h,
      Err(e) => {
        let _ = std::fs::remove_dir(link); // 失败不留半截空目录
        return Err(e);
      }
    };
    let result = unsafe {
      DeviceIoControl(handle, FSCTL_SET_REPARSE_POINT, Some(buf.as_ptr().cast()), buf.len() as u32, None, 0, None, None)
    };
    unsafe {
      let _ = CloseHandle(handle);
    }
    if let Err(e) = result {
      let _ = std::fs::remove_dir(link); // 同上：挂载失败回滚成未发生
      return Err(format!("链接挂载失败（{}）：{e}", link.display()));
    }
    Ok(())
  }

  /// FSCTL_GET_REPARSE_POINT 字节流按 MountPoint 布局解析（与 build_mount_point_buffer 互逆）：
  /// 校验 tag 后取 substitute / print 两名串；返回 None = 不是挂载点 / 形状不符
  fn parse_mount_point(buf: &[u8]) -> Option<(String, String)> {
    if buf.len() < NAMES_BASE {
      return None;
    }
    if u32::from_le_bytes(buf[0..4].try_into().ok()?) != TAG_MOUNT_POINT {
      return None;
    }
    let u16_at = |o: usize| u16::from_le_bytes([buf[o], buf[o + 1]]);
    let sub_off = NAMES_BASE + u16_at(8) as usize;
    let sub_len = u16_at(10) as usize;
    let print_off = NAMES_BASE + u16_at(12) as usize;
    let print_len = u16_at(14) as usize;
    let take_str = |off: usize, len: usize| -> Option<String> {
      let slice = buf.get(off..off + len)?;
      let units: Vec<u16> = slice.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
      Some(String::from_utf16_lossy(&units))
    };
    Some((take_str(sub_off, sub_len)?, take_str(print_off, print_len)?))
  }

  pub(super) fn read_target(link: &Path) -> Option<PathBuf> {
    let handle = open_link(link, GENERIC_READ).ok()?;
    let mut buf = vec![0u8; MAX_BUF as usize];
    let result = unsafe {
      DeviceIoControl(handle, FSCTL_GET_REPARSE_POINT, None, 0, Some(buf.as_mut_ptr().cast()), MAX_BUF, None, None)
    };
    unsafe {
      let _ = CloseHandle(handle);
    }
    result.ok()?;
    let (substitute, print) = parse_mount_point(&buf)?;
    // substitute（\??\C:\foo 内核形态）是写入时的规范形，比 print（显示用字符串）可靠；
    // 剥 \??\ 前缀还原普通路径，剥不出才退 print
    let target = substitute.strip_prefix(r"\??\").map(str::to_string).unwrap_or(print);
    if target.is_empty() {
      return None;
    }
    Some(PathBuf::from(target))
  }

  pub(super) fn remove(link: &Path) -> Result<(), String> {
    let wide: Vec<u16> = link.as_os_str().encode_wide().chain(Some(0)).collect();
    // RemoveDirectoryW 对 reparse point 只删链接本体——这是 junction 区别于
    // 「递归删除」语义的关键，目标目录内容分毫不动
    unsafe { RemoveDirectoryW(PCWSTR(wide.as_ptr())) }.map_err(|e| format!("链接移除失败（{}）：{e}", link.display()))
  }
}

#[cfg(not(windows))]
mod imp {
  use std::path::{Path, PathBuf};

  pub(super) fn is_reparse_dir(_path: &Path) -> bool {
    false
  }

  pub(super) fn create(_link: &Path, _target: &Path) -> Result<(), String> {
    Err("当前平台不支持目录链接（Windows 优先，PRD 02）".to_string())
  }

  pub(super) fn read_target(_link: &Path) -> Option<PathBuf> {
    None
  }

  pub(super) fn remove(_link: &Path) -> Result<(), String> {
    Err("当前平台不支持目录链接（Windows 优先，PRD 02）".to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// junction 三件套：create 后 is_reparse_dir 真、read_target 还原目标、透过链接能读目标
  /// 内容；remove 只删链接本体，目标及内容完好
  #[cfg(windows)]
  #[test]
  fn create_read_remove_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("目标");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("内容.txt"), b"x").unwrap();
    let link = tmp.path().join("门");

    create(&link, &target).unwrap();
    assert!(is_reparse_dir(&link), "create 后应为 reparse 目录");
    assert!(!is_reparse_dir(&target), "普通目录不应误判");
    assert!(!is_reparse_dir(&target.join("内容.txt")), "文件不应误判");
    assert!(link.join("内容.txt").is_file(), "透过链接应能读目标内容");

    let want = target.canonicalize().unwrap();
    let want = want.as_os_str().to_string_lossy().trim_start_matches(r"\\?\").to_string();
    let got = read_target(&link).expect("read_target 应还原目标");
    assert_eq!(got.as_os_str().to_string_lossy().to_lowercase(), want.to_lowercase(), "read_target 应还原 canonical 目标");

    remove(&link).unwrap();
    assert!(!link.exists(), "remove 后链接本体消失");
    assert!(target.join("内容.txt").is_file(), "目标内容分毫不动");
  }

  /// spike 红线 a：回收站删除 junction 只删链接本体——目标目录及内容完好；
  /// 断链 junction（目标先删）回收站删除同样成功（「解除挂载」命令的可行性前提）
  #[cfg(windows)]
  #[test]
  #[ignore = "spike：回收站删 junction 红线实测（会真实进系统回收站，手动跑）"]
  fn spike_recycle_bin_deletes_link_only() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("目标");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("内容.txt"), b"x").unwrap();
    let root = tmp.path().join("根");
    std::fs::create_dir_all(&root).unwrap();
    let link = root.join("门");
    create(&link, &target).unwrap();
    assert!(is_reparse_dir(&link));

    crate::adapters::recycle_bin::delete_to_recycle(std::slice::from_ref(&link)).unwrap();
    assert!(!link.exists(), "junction 本体应已删除");
    assert!(target.join("内容.txt").is_file(), "红线：目标及内容必须完好");

    // 断链 junction：目标先删，回收站删除仍成功
    let link2 = root.join("断门");
    create(&link2, &target).unwrap();
    std::fs::remove_dir_all(&target).unwrap();
    assert!(is_reparse_dir(&link2), "断链后 reparse 本体仍在");
    crate::adapters::recycle_bin::delete_to_recycle(std::slice::from_ref(&link2)).unwrap();
    assert!(!link2.exists(), "断链 junction 回收站删除应成功");
  }

  /// spike 红线 b：rename 只动「门」——junction 改名后目标不动，新名仍指向原目标
  #[cfg(windows)]
  #[test]
  #[ignore = "spike：rename junction 红线实测（手动跑）"]
  fn spike_rename_moves_link_only() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("目标");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("内容.txt"), b"x").unwrap();
    let root = tmp.path().join("根");
    std::fs::create_dir_all(&root).unwrap();
    let link = root.join("门");
    create(&link, &target).unwrap();

    let link2 = root.join("新门");
    std::fs::rename(&link, &link2).unwrap();
    assert!(!link.exists() && link2.exists(), "rename 只动链接本体");
    assert!(target.join("内容.txt").is_file(), "目标不动");
    assert!(is_reparse_dir(&link2), "新名仍是 junction");
    assert!(link2.join("内容.txt").is_file(), "新名仍指向原目标");
  }

  /// spike 穿透：RDC（ReadDirectoryChangesW 递归监听）是否穿透 junction——监听 root，
  /// 改 target 内文件，收集事件路径形态：无事件 / junction 词法前缀 / target 真实路径。
  /// 结论决定工作空间 watcher 是否要为每条链接单独补监听
  #[cfg(windows)]
  #[test]
  #[ignore = "spike：RDC 穿透 junction 实测（手动跑，读 --nocapture 输出）"]
  fn spike_watch_penetration() {
    use std::sync::mpsc;
    use std::time::Duration;
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("目标");
    std::fs::create_dir_all(&target).unwrap();
    let root = tmp.path().join("根");
    std::fs::create_dir_all(&root).unwrap();
    let link = root.join("门");
    create(&link, &target).unwrap();

    let (tx, rx) = mpsc::channel();
    let mut handle = crate::adapters::inbox_watch::spawn_changes_only(root.clone(), move |paths| {
      for p in paths {
        let _ = tx.send(p.clone());
      }
    })
    .unwrap();
    std::thread::sleep(Duration::from_millis(500)); // 等监听就位

    let f = target.join("变化.txt");
    std::fs::write(&f, b"1").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    std::fs::write(&f, b"22").unwrap();

    let mut got = Vec::new();
    while let Ok(p) = rx.recv_timeout(Duration::from_secs(3)) {
      got.push(p);
      if got.len() >= 20 {
        break;
      }
    }
    handle.stop();
    println!("=== 穿透事件路径（{} 条） ===", got.len());
    for p in &got {
      println!("  {}", p.display());
    }
    println!("link   = {}", link.display());
    println!("target = {}", target.display());
  }
}
