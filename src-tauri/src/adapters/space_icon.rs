//! 储存空间文件夹品牌图标（2026-10-03 用户定，Eagle `.library` 文件夹同款思路——品牌感与
//! 辨识度）：Windows 文件夹自定义图标的标准机制 = desktop.ini `[.ShellClassInfo] IconResource` 加
//! **文件夹只读属性位**（desktop.ini 生效前提；目录只读位无写保护语义，Explorer 自注
//! 「仅应用于文件夹中的文件」）。三件套 = `{空间根}/AniKit.ico`（嵌入 icons/space-folder.ico，
//! 设计稿 AniKit-library-folder-clean-v3-1024）、desktop.ini（IconResource 相对路径 + InfoTip
//! 悬停提示）、两文件隐藏位（不干扰资源管理器浏览）。
//! 幂等：ico 字节一致跳过（不一致覆写回品牌图标——专有文件名单一写点）；desktop.ini 已存在
//! **不动**（可能承载用户其它 shell 自定义键）。失败静默 log——图标是装饰，不阻塞空间创建 / 打开。
//! 非 Windows 暂不支持（PRD 目标平台 Windows 优先）。

use std::path::Path;

/// 空间文件夹图标（设计稿 `design/LOGO/image/AniKit-library-folder-clean-v3-1024.png` 离线转
/// ICO，16/24/32/48/64/256 六尺寸；换图标 = 重转覆盖同路径，exe 重编随包）
const SPACE_ICON: &[u8] = include_bytes!("../../icons/space-folder.ico");

const ICO_NAME: &str = "AniKit.ico";
const INI_NAME: &str = "desktop.ini";
/// IconResource 相对路径（相对所在文件夹，Vista+ 标准写法）；InfoTip = 资源管理器悬停提示
const INI_BODY: &str = "[.ShellClassInfo]\r\nIconResource=AniKit.ico,0\r\nInfoTip=AniKit 储存空间\r\n";

/// 落地空间文件夹品牌图标（activate_space 调用：创建 / 打开同覆盖，现有空间惰性补写）
pub fn apply(space_root: &Path) {
  if let Err(e) = imp::apply(space_root) {
    log::warn!("空间文件夹图标落地失败（静默跳过）：{e}");
  }
}

#[cfg(windows)]
mod imp {
  use std::fs;
  use std::os::windows::ffi::OsStrExt;
  use std::path::Path;

  use windows::core::PCWSTR;
  use windows::Win32::Storage::FileSystem::{
    GetFileAttributesW, SetFileAttributesW, FILE_FLAGS_AND_ATTRIBUTES,
  };

  use super::{ICO_NAME, INI_BODY, INI_NAME, SPACE_ICON};

  const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
  const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
  const INVALID_ATTRIBUTES: u32 = u32::MAX; // GetFileAttributesW 失败返回值

  pub fn apply(root: &Path) -> Result<(), String> {
    // 图标文件：字节一致跳过（幂等）；不一致覆写回品牌图标（专有文件名，单一写点）
    let ico = root.join(ICO_NAME);
    let same = fs::read(&ico).map(|b| b == SPACE_ICON).unwrap_or(false);
    if !same {
      fs::write(&ico, SPACE_ICON).map_err(|e| format!("空间图标写入失败：{e}"))?;
    }
    add_attr(&ico, FILE_ATTRIBUTE_HIDDEN)?;
    // desktop.ini：已存在不动（用户自定义优先——本文件可能承载 shell 其它键）
    let ini = root.join(INI_NAME);
    if !ini.exists() {
      fs::write(&ini, INI_BODY).map_err(|e| format!("desktop.ini 写入失败：{e}"))?;
      add_attr(&ini, FILE_ATTRIBUTE_HIDDEN)?;
    }
    // 文件夹只读位 = desktop.ini 生效前提（Windows 只认「只读或系统属性目录」的 desktop.ini）
    add_attr(root, FILE_ATTRIBUTE_READONLY)?;
    Ok(())
  }

  /// 属性位叠加（先读后加保留现有属性——std set_readonly 整体覆盖属性字，不可用）
  fn add_attr(path: &Path, attr: u32) -> Result<(), String> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
      let cur = GetFileAttributesW(PCWSTR(wide.as_ptr()));
      if cur == INVALID_ATTRIBUTES {
        return Err(format!("属性读取失败：{}", path.display()));
      }
      if cur & attr == 0 {
        SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(cur | attr))
          .map_err(|e| format!("属性设置失败（{}）：{e}", path.display()))?;
      }
    }
    Ok(())
  }
}

#[cfg(not(windows))]
mod imp {
  use std::path::Path;

  pub fn apply(_root: &Path) -> Result<(), String> {
    Ok(()) // 非 Windows 暂无文件夹图标机制
  }
}

#[cfg(all(test, windows))]
mod tests {
  use std::fs;
  use std::os::windows::fs::MetadataExt;

  use super::*;

  /// 落地三件套 + 幂等（ini 用户内容不动 / ico 覆写回品牌图）+ 属性位（文件夹只读 / 文件隐藏）
  #[test]
  fn apply_writes_icon_ini_and_attrs() {
    let dir = tempfile::tempdir().unwrap();
    apply(dir.path());
    assert_eq!(fs::read(dir.path().join(ICO_NAME)).unwrap(), SPACE_ICON);
    let ini = fs::read_to_string(dir.path().join(INI_NAME)).unwrap();
    assert!(ini.contains("IconResource=AniKit.ico,0"));
    assert!(fs::metadata(dir.path()).unwrap().permissions().readonly(), "文件夹只读位 = desktop.ini 生效前提");
    const HIDDEN: u32 = 0x2;
    assert!(fs::metadata(dir.path().join(ICO_NAME)).unwrap().file_attributes() & HIDDEN != 0, "图标文件隐藏位");

    // 幂等：用户改过 ini 不动；ico 被改覆写回；重复调用不出错
    fs::write(dir.path().join(INI_NAME), "custom").unwrap();
    fs::write(dir.path().join(ICO_NAME), b"x").unwrap();
    apply(dir.path());
    assert_eq!(fs::read_to_string(dir.path().join(INI_NAME)).unwrap(), "custom");
    assert_eq!(fs::read(dir.path().join(ICO_NAME)).unwrap(), SPACE_ICON);
  }
}
