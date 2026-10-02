//! 储存空间文件夹品牌图标（2026-10-03 用户定，Eagle `.library` 文件夹同款思路——品牌感与
//! 辨识度）：Windows 文件夹自定义图标机制 = desktop.ini + **文件夹只读属性位**（desktop.ini
//! 生效前提；目录只读位无写保护语义，Explorer 自注「仅应用于文件夹中的文件」）。
//! **图标文件单份全局随软件**（2026-10-03 二轮，用户定——初版落空间根被否「多建几个空间
//! 每个都要一份 ico」）：`%APPDATA%\AniKit\space-folder.ico`（全局配置目录，config.json 同
//! 目录），desktop.ini 用旧式 `IconFile` 键 = 支持**环境变量展开**（`IconResource` 新式键
//! 不展开——社区既定行为，XP 起沿用至今），ini 内容恒定与用户无关，空间搬迁 / 换机 / 换
//! Windows 用户全部免维护；无 AniKit 环境打开空间 = 默认黄文件夹（优雅降级）。每个空间
//! 落盘的只有 desktop.ini 一个隐藏文件 + 文件夹只读位。
//! 幂等：全局 ico 字节一致跳过（不一致覆写——版本升级随 exe 换新）；desktop.ini 不存在写、
//! 内容含「AniKit」指纹（我们两代模板）覆写换新、用户自定义不动；空间根旧 `AniKit.ico`
//!（初版方案遗物，字节 = 嵌入图 = 我们写的）顺带清理。失败静默 log——图标是装饰，不阻塞开空间。
//! 非 Windows 暂不支持（PRD 目标平台 Windows 优先）。

use std::path::Path;

/// 空间文件夹图标（设计稿 `design/LOGO/02_Final_PNG/AniKit-library-folder-clean-v4-1024.png`
/// 离线转 ICO：宽长方形款（2026-10-03 v4 换稿——v3 正方形与资源管理器默认图标并排显空），
/// 内容 alpha>8 trim 后等比贴 92% 宽正方形画布，16/24/32/48/64/256 六尺寸；
/// 换图标 = 重转覆盖同路径，exe 重编随包；已落地靠「字节不一致覆写」幂等换新）
const SPACE_ICON: &[u8] = include_bytes!("../../icons/space-folder.ico");

const ICO_NAME: &str = "space-folder.ico";
const INI_NAME: &str = "desktop.ini";
/// 空间根初版图标遗物名（方案一落地文件，二轮起清理）
const LEGACY_ICO_NAME: &str = "AniKit.ico";
/// IconFile 旧式键 + %APPDATA%（Explorer 读取时展开环境变量；IconResource 不展开故不用）。
/// 字面路径与 core::config 的 dirs::config_dir()/AniKit 一致（Windows = Roaming %APPDATA%）
const INI_BODY: &str = "[.ShellClassInfo]\r\nIconFile=%APPDATA%\\AniKit\\space-folder.ico\r\nIconIndex=0\r\nInfoTip=AniKit 储存空间\r\n";
/// desktop.ini 指纹：内容含此 = 我们写的（两代模板 InfoTip/IconResource 均含），可覆写
const INI_FINGERPRINT: &str = "AniKit";

/// 落地空间文件夹品牌图标（activate_space 调用：创建 / 打开同覆盖，现有空间惰性补写）
pub fn apply(space_root: &Path) {
  if let Err(e) = imp::apply(space_root) {
    log::warn!("空间文件夹图标落地失败（静默跳过）：{e}");
  }
}

/// 核心逻辑与真实全局路径解耦（测试注入 tempdir 用——直测 apply 会写真实 %APPDATA%）
#[cfg(all(test, windows))]
fn apply_with(space_root: &Path, icon_path: &Path) -> Result<(), String> {
  imp::apply_with(space_root, icon_path)
}

/// 全局图标路径（%APPDATA%\AniKit\space-folder.ico，config.json 同目录）
fn global_icon_path() -> Result<std::path::PathBuf, String> {
  dirs::config_dir()
    .map(|d| d.join("AniKit").join(ICO_NAME))
    .ok_or_else(|| "无法定位系统配置目录".to_string())
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

  use super::{global_icon_path, INI_BODY, INI_FINGERPRINT, INI_NAME, LEGACY_ICO_NAME, SPACE_ICON};

  const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
  const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
  const INVALID_ATTRIBUTES: u32 = u32::MAX; // GetFileAttributesW 失败返回值

  pub fn apply(root: &Path) -> Result<(), String> {
    apply_with(root, &global_icon_path()?)
  }

  pub fn apply_with(root: &Path, ico: &Path) -> Result<(), String> {
    // ① 全局图标 ensure（单份随软件）：字节一致跳过，不一致覆写（版本升级随 exe 换新）
    let same = fs::read(ico).map(|b| b == SPACE_ICON).unwrap_or(false);
    if !same {
      if let Some(dir) = ico.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("图标目录创建失败：{e}"))?;
      }
      fs::write(ico, SPACE_ICON).map_err(|e| format!("全局图标写入失败：{e}"))?;
    }
    // ② desktop.ini：不存在写；含「AniKit」指纹（我们的）覆写换新；用户自定义不动
    let ini = root.join(INI_NAME);
    match fs::read_to_string(&ini) {
      Err(_) => write_ini(&ini)?,
      Ok(body) if body.contains(INI_FINGERPRINT) => write_ini(&ini)?,
      Ok(_) => {}
    }
    // ③ 空间根初版图标遗物清理（字节 = 嵌入图 = 我们写的；用户换过内容不动）
    let legacy = root.join(LEGACY_ICO_NAME);
    if fs::read(&legacy).map(|b| b == SPACE_ICON).unwrap_or(false) {
      let _ = fs::remove_file(&legacy);
    }
    // ④ 文件夹只读位 = desktop.ini 生效前提（Windows 只认「只读或系统属性目录」的 desktop.ini）
    add_attr(root, FILE_ATTRIBUTE_READONLY)?;
    Ok(())
  }

  fn write_ini(ini: &Path) -> Result<(), String> {
    fs::write(ini, INI_BODY).map_err(|e| format!("desktop.ini 写入失败：{e}"))?;
    add_attr(ini, FILE_ATTRIBUTE_HIDDEN)
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

  /// 全流程：全局 ico 落地覆写 / ini 指纹分流（我们的覆写、用户的不动）/ 遗物清理 / 属性位
  ///（apply_with 注入 tempdir 作图标位——直测 apply 会写真实 %APPDATA%）
  #[test]
  fn apply_global_icon_ini_and_cleanup() {
    let appdata = tempfile::tempdir().unwrap();
    let ico = appdata.path().join("AniKit").join(ICO_NAME);
    let space = tempfile::tempdir().unwrap();
    let run = || apply_with(space.path(), &ico);

    // 首次落地：全局 ico 写入 + ini 新写 + 只读位
    run().unwrap();
    assert_eq!(fs::read(&ico).unwrap(), SPACE_ICON, "全局图标落地（随软件单份）");
    let body = fs::read_to_string(space.path().join(INI_NAME)).unwrap();
    assert!(body.contains("IconFile=%APPDATA%\\AniKit\\space-folder.ico"), "IconFile 环境变量款");
    assert!(fs::metadata(space.path()).unwrap().permissions().readonly(), "文件夹只读位 = 生效前提");
    const HIDDEN: u32 = 0x2;
    assert!(fs::metadata(space.path().join(INI_NAME)).unwrap().file_attributes() & HIDDEN != 0, "ini 隐藏位");

    // 幂等分流：我们写的 ini（含指纹）被改旧也覆写回；用户自定义 ini 不动
    fs::write(space.path().join(INI_NAME), "IconResource=AniKit.ico,0").unwrap();
    run().unwrap();
    assert!(fs::read_to_string(space.path().join(INI_NAME)).unwrap().contains("IconFile=%APPDATA%"), "我们的旧 ini 覆写换新");
    fs::write(space.path().join(INI_NAME), "custom").unwrap();
    run().unwrap();
    assert_eq!(fs::read_to_string(space.path().join(INI_NAME)).unwrap(), "custom", "用户 ini 不动");

    // 遗物清理：字节 = 嵌入图的旧 AniKit.ico 删除；用户换过内容的不动
    fs::write(space.path().join(LEGACY_ICO_NAME), SPACE_ICON).unwrap();
    run().unwrap();
    assert!(!space.path().join(LEGACY_ICO_NAME).exists(), "初版遗物（我们写的）清理");
    fs::write(space.path().join(LEGACY_ICO_NAME), b"user").unwrap();
    run().unwrap();
    assert!(space.path().join(LEGACY_ICO_NAME).exists(), "用户改过内容不动");

    // 全局 ico 被改（版本旧）覆写回
    fs::write(&ico, b"old").unwrap();
    run().unwrap();
    assert_eq!(fs::read(&ico).unwrap(), SPACE_ICON);
  }
}
