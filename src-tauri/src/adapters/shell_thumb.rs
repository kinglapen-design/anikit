//! Shell 格式图标适配器（PRD 04-03 保底，4.2）：IShellItemImageFactory 取系统格式图标。
//! 只取图标（SIIGBF_ICONONLY），产物按格式缓存于 cache/thumbs/{ext}.webp，**不进 info**——
//! info 内放 shell 图会污染：后期补伴生缩略图（如 fla 的 WEBP）后两份并存（2026-07-22 用户定案）。
//! mac 端方案保留：Quick Look（QLThumbnailGenerator），暂不做（4.2 定案）。

use std::path::Path;

use image::RgbaImage;

/// 系统格式图标尺寸（见方，带 alpha）
pub const FORMAT_ICON_EDGE: u32 = 256;

/// 取文件的系统格式图标；系统无关联图标 / 调用失败返回 None（能力探测语义，不算吞错误）
pub fn format_icon(path: &Path, edge: u32) -> Option<RgbaImage> {
  imp::format_icon(path, edge)
}

#[cfg(windows)]
mod imp {
  use std::cell::RefCell;
  use std::mem::size_of;
  use std::path::Path;

  use image::RgbaImage;
  use windows::core::{HSTRING, PCWSTR};
  use windows::Win32::Foundation::{S_FALSE, SIZE};
  use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
  };
  use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
  use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
  };

  /// 线程退出时配对 CoUninitialize（仅当本线程 COM 由我们初始化）
  struct ComGuard {
    owned: bool,
  }

  impl Drop for ComGuard {
    fn drop(&mut self) {
      if self.owned {
        unsafe { CoUninitialize() };
      }
    }
  }

  thread_local! {
    /// 每线程一次的 COM 初始化结果（None = 未尝试；Err = 失败不再重试）
    static COM: RefCell<Option<Result<ComGuard, ()>>> = const { RefCell::new(None) };
  }

  /// STA 初始化（shell 第三方 handler 对 STA 兼容性最好）；S_FALSE = 已被别人初始化，借用不 uninit
  fn ensure_com() -> bool {
    COM.with(|c| {
      let mut slot = c.borrow_mut();
      if slot.is_none() {
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        *slot = Some(if hr.is_ok() {
          Ok(ComGuard { owned: hr != S_FALSE })
        } else {
          log::warn!("COM 初始化失败（{hr:?}），格式图标不可用");
          Err(())
        });
      }
      matches!(&*slot, Some(Ok(_)))
    })
  }

  /// HBITMAP 析构 guard（GDI 句柄不配对释放会泄漏）
  struct Bitmap(HBITMAP);

  impl Drop for Bitmap {
    fn drop(&mut self) {
      unsafe {
        let _ = DeleteObject(HGDIOBJ((self.0).0));
      }
    }
  }

  pub(super) fn format_icon(path: &Path, edge: u32) -> Option<RgbaImage> {
    if !ensure_com() {
      return None;
    }
    let hpath = HSTRING::from(path.as_os_str());
    let factory: IShellItemImageFactory =
      unsafe { SHCreateItemFromParsingName(PCWSTR::from_raw(hpath.as_ptr()), None) }
        .map_err(|e| log::warn!("Shell 项创建失败（{}）：{e}", path.display()))
        .ok()?;
    let size = SIZE { cx: edge as i32, cy: edge as i32 };
    let hbm = unsafe { factory.GetImage(size, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK) }
      .map_err(|e| log::warn!("系统图标获取失败（{}）：{e}", path.display()))
      .ok()?;
    hbitmap_to_rgba(Bitmap(hbm))
  }

  /// HBITMAP → RGBA：GetDIBits 顶向下 32bpp，BGRA 转 RGBA 并解除预乘 alpha
  fn hbitmap_to_rgba(bmp: Bitmap) -> Option<RgbaImage> {
    let mut info = BITMAP::default();
    let ok = unsafe { GetObjectW(HGDIOBJ((bmp.0).0), size_of::<BITMAP>() as i32, Some(&mut info as *mut _ as _)) };
    if ok == 0 {
      log::warn!("图标位图信息读取失败");
      return None;
    }
    let (w, h) = (info.bmWidth, info.bmHeight.abs());
    if w <= 0 || h <= 0 {
      return None;
    }
    let mut bmi = BITMAPINFO {
      bmiHeader: BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h, // 负 = 顶向下行序
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
      },
      ..Default::default()
    };
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let hdc = unsafe { CreateCompatibleDC(None) };
    if hdc.is_invalid() {
      return None;
    }
    let lines =
      unsafe { GetDIBits(hdc, bmp.0, 0, h as u32, Some(buf.as_mut_ptr() as _), &mut bmi, DIB_RGB_COLORS) };
    unsafe {
      let _ = DeleteDC(hdc);
    }
    if lines == 0 {
      log::warn!("图标位图像素读取失败");
      return None;
    }
    for px in buf.chunks_exact_mut(4) {
      let (b, g, r, a) = (px[0], px[1], px[2], px[3]);
      px[0] = unpremul(r, a);
      px[1] = unpremul(g, a);
      px[2] = unpremul(b, a);
    }
    RgbaImage::from_raw(w as u32, h as u32, buf)
  }

  /// 预乘 alpha 还原（四舍五入，封顶 255；a=0 时颜色无意义归零）
  fn unpremul(c: u8, a: u8) -> u8 {
    if a == 0 {
      return 0;
    }
    ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8
  }
}

/// 非 Windows 存根：mac 走 Quick Look（QLThumbnailGenerator），PRD 04-03，暂不做
#[cfg(not(windows))]
mod imp {
  use std::path::Path;

  use image::RgbaImage;

  pub(super) fn format_icon(_path: &Path, _edge: u32) -> Option<RgbaImage> {
    None
  }
}

#[cfg(all(test, windows))]
mod tests {
  use super::*;

  #[test]
  fn format_icon_returns_bitmap_for_unknown_ext() {
    let dir = tempfile::tempdir().unwrap();
    let fla = dir.path().join("工程.fla");
    std::fs::write(&fla, "fake fla bytes").unwrap();

    let img = format_icon(&fla, FORMAT_ICON_EDGE).expect("未知扩展名也应返回系统默认图标");
    assert!(img.width() > 0 && img.height() > 0);
  }

  #[test]
  fn format_icon_returns_none_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let ghost = dir.path().join("不存在.fla");
    assert!(format_icon(&ghost, FORMAT_ICON_EDGE).is_none());
  }
}
