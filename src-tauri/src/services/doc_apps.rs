//! 新建文档软件检测（1.3，07 新增菜单"新建文档"组，2026-08-11 用户定）：
//! 每次打开 UI 前端调 detect_doc_apps——按扩展名查 Windows 文件关联（AssocQueryStringW
//! ASSOCSTR_EXECUTABLE）判定软件是否安装；已安装 → 复用 shell_thumb 提取**主程序 exe 图标**
//!（IShellItemImageFactory，品牌图标 = An/Ps/Ae 方块观感）编码 PNG data URL 供菜单直接渲染。
//! 未安装（关联查询失败）→ installed = false，前端删除对应新建项。

use serde::Serialize;

/// 单类文档的软件检测结果（icon = PNG data URL，未安装/取图标失败为 None）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocAppInfo {
  pub id: &'static str,
  pub installed: bool,
  pub icon: Option<String>,
}

/// 检测清单：文档类型 id → 关联扩展名
const PROBES: [(&str, &str); 7] = [
  ("animate", ".fla"),
  ("photoshop", ".psd"),
  ("afterEffects", ".aep"),
  ("illustrator", ".ai"),
  ("word", ".docx"),
  ("excel", ".xlsx"),
  ("text", ".txt"),
];

/// 菜单图标 data URL 的取图边长（18 盒显示，2x 屏取 32 保清晰）
const ICON_EDGE: u32 = 32;

pub fn detect_doc_apps() -> Vec<DocAppInfo> {
  PROBES
    .iter()
    .map(|(id, ext)| {
      let exe = imp::associated_exe(ext);
      let icon = exe
        .as_deref()
        .and_then(|p| crate::adapters::shell_thumb::format_icon(std::path::Path::new(p), ICON_EDGE))
        .and_then(png_data_url);
      DocAppInfo {
        id,
        installed: exe.is_some(),
        icon,
      }
    })
    .collect()
}

/// RgbaImage → PNG data URL（菜单项图标内联用，不落盘）
fn png_data_url(img: image::RgbaImage) -> Option<String> {
  use base64::Engine;
  let mut buf = std::io::Cursor::new(Vec::new());
  image::DynamicImage::ImageRgba8(img)
    .write_to(&mut buf, image::ImageFormat::Png)
    .ok()?;
  Some(format!(
    "data:image/png;base64,{}",
    base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
  ))
}

#[cfg(windows)]
mod imp {
  use windows::core::PCWSTR;
  use windows::core::HSTRING;
  use windows::Win32::UI::Shell::{AssocQueryStringW, ASSOCF_NONE, ASSOCSTR_EXECUTABLE};

  /// 扩展名 → 关联主程序 exe 绝对路径；无关联（软件未安装）返回 None
  pub(super) fn associated_exe(ext: &str) -> Option<String> {
    let hext = HSTRING::from(ext);
    unsafe {
      // 先查所需缓冲长度（含终止 NUL）
      let mut len: u32 = 0;
      let hr = AssocQueryStringW(
        ASSOCF_NONE,
        ASSOCSTR_EXECUTABLE,
        PCWSTR::from_raw(hext.as_ptr()),
        PCWSTR::null(),
        None,
        &mut len,
      );
      // 无关联返回 HRESULT 错误（如 0x80070483）；长度 0/1 也视为无
      if hr.is_err() || len <= 1 {
        return None;
      }
      let mut buf = vec![0u16; len as usize];
      let hr = AssocQueryStringW(
        ASSOCF_NONE,
        ASSOCSTR_EXECUTABLE,
        PCWSTR::from_raw(hext.as_ptr()),
        PCWSTR::null(),
        Some(windows::core::PWSTR::from_raw(buf.as_mut_ptr())),
        &mut len,
      );
      if hr.is_err() {
        return None;
      }
      let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
      let path = String::from_utf16_lossy(&buf[..end]);
      if path.is_empty() {
        return None;
      }
      // 未知扩展名关联到 Windows 占位程序 OpenWith.exe（"打开方式"）——视为无真实关联（未安装）
      let file_name = path.rsplit(['\\', '/']).next().unwrap_or(&path);
      if file_name.eq_ignore_ascii_case("openwith.exe") {
        return None;
      }
      Some(path)
    }
  }
}

/// 非 Windows 存根：全部未安装（mac 方案随对应平台阶段再做）
#[cfg(not(windows))]
mod imp {
  pub(super) fn associated_exe(_ext: &str) -> Option<String> {
    None
  }
}

#[cfg(all(test, windows))]
mod tests {
  use super::*;

  #[test]
  fn text_doc_always_has_association_on_windows() {
    // 记事本是 Windows 自带：.txt 关联必然存在
    let apps = detect_doc_apps();
    let text = apps.iter().find(|a| a.id == "text").unwrap();
    assert!(text.installed, ".txt 应有系统关联");
    assert!(text.icon.is_some(), "记事本图标应可提取");
  }

  #[test]
  fn unknown_ext_has_no_association() {
    // 未知扩展名关联到 OpenWith.exe 占位程序，应判为无关联
    assert!(imp::associated_exe(".anikit-nonexistent-ext").is_none());
  }
}
