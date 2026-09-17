//! 系统剪贴板适配器（规范第 2 条；8.3 发送至 AN）：Win32 私有格式写入 + 标准格式读写补充。
//! Animate 元件剪贴板格式 = 注册格式 "Flash 11 Picture"（内容为 XFL 包字节，
//! spike/clipboard-probe 实证）；arboard 只支持标准格式（文本/图片），私有格式直调 Win32。
//! 2026-09-17 实机走查补：
//!   - read_image_dib_fallback：arboard get_image 只认 PNG / CF_DIBV5 两格式，大量软件
//!     （QQ·微信截图、Win32 老程序）只写 CF_DIB——兜底直读（「外部剪贴板导入以前可以现在不行」根因）；
//!   - write_files_clipboard：单次开剪贴板写 CF_HDROP 文件列表 + 可选位图（PNG + CF_DIBV5），
//!     支撑「复制文件」同步写系统剪贴板（复制图片可粘到 APP 外，Eagle 同款）。
//!
//! 业务语义（fla → 载荷转换）在 services/send_stage，本层只管字节读写。

#[cfg(windows)]
pub use imp::{read_image_dib_fallback, write_files_clipboard, write_flash_picture};

#[cfg(not(windows))]
pub fn write_flash_picture(_data: &[u8]) -> Result<(), String> {
  Err("当前平台不支持发送至 Animate".to_string())
}

#[cfg(not(windows))]
pub fn read_image_dib_fallback() -> Result<image::RgbaImage, String> {
  Err("当前平台不支持剪贴板读取".to_string())
}

#[cfg(not(windows))]
pub fn write_files_clipboard(
  _paths: &[&std::path::Path],
  _image: Option<&image::RgbaImage>,
) -> Result<(), String> {
  Err("当前平台不支持剪贴板写入".to_string())
}

#[cfg(windows)]
mod imp {
  use windows::core::PCWSTR;
  use windows::Win32::Foundation::{GlobalFree, HANDLE};
  use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
  };
  use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

  /// 写入 "Flash 11 Picture" 私有格式（XFL 包字节）。成功后堆句柄所有权移交系统。
  /// OpenClipboard 竞争（其他程序占用）重试 3 次 × 50ms；最终失败报「剪贴板被占用」。
  pub fn write_flash_picture(data: &[u8]) -> Result<(), String> {
    let name: Vec<u16> = "Flash 11 Picture".encode_utf16().chain(std::iter::once(0)).collect();
    let fmt = unsafe { RegisterClipboardFormatW(PCWSTR(name.as_ptr())) };
    if fmt == 0 {
      return Err("注册剪贴板格式失败".to_string());
    }
    let mut opened = false;
    for _ in 0..3 {
      if unsafe { OpenClipboard(None) }.is_ok() {
        opened = true;
        break;
      }
      std::thread::sleep(std::time::Duration::from_millis(50));
    }
    if !opened {
      return Err("剪贴板被占用，请重试".to_string());
    }
    let r = write_inner(fmt, data);
    unsafe { let _ = CloseClipboard(); }
    r
  }

  fn write_inner(fmt: u32, data: &[u8]) -> Result<(), String> {
    unsafe {
      EmptyClipboard().map_err(|e| format!("清空剪贴板失败：{e}"))?;
      let h = GlobalAlloc(GMEM_MOVEABLE, data.len()).map_err(|e| format!("内存分配失败：{e}"))?;
      let ptr = GlobalLock(h);
      if ptr.is_null() {
        let _ = GlobalFree(Some(h));
        return Err("内存锁定失败".to_string());
      }
      std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
      let _ = GlobalUnlock(h);
      // 成功后句柄所有权移交系统（禁止再 GlobalFree）；失败必须自己释放防泄漏
      if let Err(e) = SetClipboardData(fmt, Some(HANDLE(h.0))) {
        let _ = GlobalFree(Some(h));
        return Err(format!("写入剪贴板失败：{e}"));
      }
      Ok(())
    }
  }

  // —— 2026-09-17 实机走查补：CF_DIB 兜底读 + CF_HDROP/位图写入 ——

  use std::os::windows::ffi::OsStrExt;
  use std::path::Path;
  use windows::Win32::Graphics::Gdi::{BITMAPV5HEADER, BI_BITFIELDS};
  use windows::Win32::System::DataExchange::GetClipboardData;
  use windows::Win32::System::Memory::GlobalSize;
  use windows::Win32::UI::Shell::DROPFILES;

  const CF_DIB: u32 = 8;
  const CF_DIBV5: u32 = 17;
  const CF_HDROP: u32 = 15;

  fn open_with_retry() -> Result<(), String> {
    for _ in 0..3 {
      if unsafe { OpenClipboard(None) }.is_ok() {
        return Ok(());
      }
      std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err("剪贴板被占用，请重试".to_string())
  }

  /// 全局内存打包一字节串并 SetClipboardData（成功后句柄所有权移交系统）
  fn set_bytes(fmt: u32, bytes: &[u8]) -> Result<(), String> {
    unsafe {
      let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|e| format!("内存分配失败：{e}"))?;
      let ptr = GlobalLock(h);
      if ptr.is_null() {
        let _ = GlobalFree(Some(h));
        return Err("内存锁定失败".to_string());
      }
      std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
      let _ = GlobalUnlock(h);
      if let Err(e) = SetClipboardData(fmt, Some(HANDLE(h.0))) {
        let _ = GlobalFree(Some(h));
        return Err(format!("写入剪贴板失败：{e}"));
      }
      Ok(())
    }
  }

  /// arboard 不读的 CF_DIB 兜底：QQ/微信截图等只写 DIB 的软件的图片也能导入。
  /// 读 = OpenClipboard 拿原始字节（不解码不开 arboard 句柄，与 get_image 先后调用互不干扰）。
  pub fn read_image_dib_fallback() -> Result<image::RgbaImage, String> {
    open_with_retry()?;
    let r = (|| unsafe {
      let h = GetClipboardData(CF_DIB).map_err(|_| "剪贴板里没有图片".to_string())?;
      let hg = windows::Win32::Foundation::HGLOBAL(h.0);
      let size = GlobalSize(hg);
      let ptr = GlobalLock(hg);
      if ptr.is_null() {
        return Err("剪贴板内存锁定失败".to_string());
      }
      let data = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
      let _ = GlobalUnlock(hg);
      Ok(data)
    })();
    unsafe {
      let _ = CloseClipboard();
    }
    decode_dib(&r?)
  }

  /// DIB 字节（BITMAPINFOHEADER 起、无文件头）→ RGBA（纯函数，单测可测）
  fn decode_dib(data: &[u8]) -> Result<image::RgbaImage, String> {
    let decoder = image::codecs::bmp::BmpDecoder::new_without_file_header(std::io::Cursor::new(data))
      .map_err(|_| "剪贴板图片数据无效".to_string())?;
    image::DynamicImage::from_decoder(decoder)
      .map(|i| i.to_rgba8())
      .map_err(|e| format!("剪贴板图片解码失败：{e}"))
  }

  /// 单次开剪贴板写：CF_HDROP 文件列表 + 可选位图（PNG 在前兼容性更好 + CF_DIBV5，arboard 同口径）
  pub fn write_files_clipboard(paths: &[&Path], image: Option<&image::RgbaImage>) -> Result<(), String> {
    if paths.is_empty() && image.is_none() {
      return Err("没有可写入的内容".to_string());
    }
    open_with_retry()?;
    let r = (|| {
      unsafe { EmptyClipboard() }.map_err(|e| format!("清空剪贴板失败：{e}"))?;
      if !paths.is_empty() {
        set_bytes(CF_HDROP, &build_hdrop_bytes(paths))?;
      }
      if let Some(img) = image {
        let png_fmt = register_format("PNG")?;
        set_bytes(png_fmt, &encode_png(img)?)?;
        set_bytes(CF_DIBV5, &build_dibv5_bytes(img))?;
      }
      Ok(())
    })();
    unsafe {
      let _ = CloseClipboard();
    }
    r
  }

  fn register_format(name: &str) -> Result<u32, String> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let fmt = unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) };
    if fmt == 0 {
      return Err("注册剪贴板格式失败".to_string());
    }
    Ok(fmt)
  }

  /// DROPFILES + 宽字符路径串（逐个 \0 分隔、末尾双 \0），纯函数（单测可测）
  fn build_hdrop_bytes(paths: &[&Path]) -> Vec<u8> {
    let header_size = std::mem::size_of::<DROPFILES>();
    let mut wide: Vec<u16> = Vec::new();
    for p in paths {
      wide.extend(p.as_os_str().encode_wide().chain(std::iter::once(0)));
    }
    wide.push(0);
    let mut out = vec![0u8; header_size + wide.len() * 2];
    unsafe {
      let df = out.as_mut_ptr() as *mut DROPFILES;
      (*df).pFiles = header_size as u32;
      (*df).pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
      (*df).fNC = windows::core::BOOL(0);
      (*df).fWide = windows::core::BOOL(1);
      std::ptr::copy_nonoverlapping(wide.as_ptr(), out.as_mut_ptr().add(header_size) as *mut u16, wide.len());
    }
    out
  }

  fn encode_png(img: &image::RgbaImage) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut buf = std::io::Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new(&mut buf)
      .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8)
      .map_err(|e| format!("剪贴板 PNG 编码失败：{e}"))?;
    Ok(buf.into_inner())
  }

  /// BITMAPV5HEADER + BGRA 像素（bottom-up 行序翻转），纯函数（单测可测；arboard add_cf_dibv5 同构）
  fn build_dibv5_bytes(img: &image::RgbaImage) -> Vec<u8> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let row = w * 4;
    let mut pixels = vec![0u8; row * h];
    for (y, dst) in pixels.chunks_exact_mut(row).enumerate() {
      let src = &img.as_raw()[(h - 1 - y) * row..(h - y) * row];
      dst.copy_from_slice(src);
    }
    for px in pixels.chunks_exact_mut(4) {
      px.swap(0, 2); // RGBA → BGRA
    }
    let header_size = std::mem::size_of::<BITMAPV5HEADER>();
    let mut out = vec![0u8; header_size + pixels.len()];
    unsafe {
      let hd = out.as_mut_ptr() as *mut BITMAPV5HEADER;
      (*hd).bV5Size = header_size as u32;
      (*hd).bV5Width = w as i32;
      (*hd).bV5Height = h as i32;
      (*hd).bV5Planes = 1;
      (*hd).bV5BitCount = 32;
      (*hd).bV5Compression = BI_BITFIELDS;
      (*hd).bV5RedMask = 0x00ff0000;
      (*hd).bV5GreenMask = 0x0000ff00;
      (*hd).bV5BlueMask = 0x000000ff;
      (*hd).bV5AlphaMask = 0xff000000;
      (*hd).bV5CSType = 0x7352_4742; // 'sRGB'（LCS_sRGB）
      std::ptr::copy_nonoverlapping(pixels.as_ptr(), out.as_mut_ptr().add(header_size), pixels.len());
    }
    out
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    #[test]
    fn hdrop_packs_wide_paths_double_nul_terminated() {
      let a = Path::new("E:\\库\\a.png");
      let b = Path::new("E:\\库\\b.psd");
      let bytes = build_hdrop_bytes(&[a, b]);
      let header_size = std::mem::size_of::<DROPFILES>();
      let df = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const DROPFILES) };
      let (p_files, f_wide) = (df.pFiles, df.fWide.0); // 打包结构体字段先按值拷贝（assert 取引用会踩 E0793）
      assert_eq!(p_files as usize, header_size, "pFiles = 头尺寸");
      assert_eq!(f_wide, 1, "宽字符标记");
      let wide: &[u16] = unsafe {
        std::slice::from_raw_parts(
          bytes.as_ptr().add(header_size) as *const u16,
          (bytes.len() - header_size) / 2,
        )
      };
      let text = String::from_utf16_lossy(wide);
      assert!(text.starts_with("E:\\库\\a.png\u{0}E:\\库\\b.psd\u{0}"), "路径逐个 NUL 分隔：{text:?}");
      assert!(wide.ends_with(&[0, 0]), "末尾双 NUL 终止");
    }

    #[test]
    fn dibv5_packs_header_and_flipped_bgra_pixels() {
      // 2×2：上排 红/绿，下排 蓝/白（RGBA）
      let mut raw = Vec::new();
      raw.extend_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255]); // 上排
      raw.extend_from_slice(&[0, 0, 255, 255, 255, 255, 255, 255]); // 下排
      let img = image::RgbaImage::from_raw(2, 2, raw).unwrap();
      let bytes = build_dibv5_bytes(&img);
      let header_size = std::mem::size_of::<BITMAPV5HEADER>();
      let hd = unsafe { &*(bytes.as_ptr() as *const BITMAPV5HEADER) };
      assert_eq!(hd.bV5Size as usize, header_size);
      assert_eq!((hd.bV5Width, hd.bV5Height, hd.bV5BitCount), (2, 2, 32));
      assert_eq!(hd.bV5AlphaMask, 0xff000000, "alpha 掩码（透明不丢）");
      let px = &bytes[header_size..];
      assert_eq!(px.len(), 16);
      // bottom-up：第一行 = 源下排（蓝/白）且 BGRA
      assert_eq!(&px[0..4], &[255, 0, 0, 255], "源蓝 (0,0,255) → BGRA 不变序");
      assert_eq!(&px[4..8], &[255, 255, 255, 255], "源白 → BGRA");
      // 第二行 = 源上排（红/绿）：红 RGBA(255,0,0) → BGRA(0,0,255)
      assert_eq!(&px[8..12], &[0, 0, 255, 255], "源红 → BGRA");
      assert_eq!(&px[12..16], &[0, 255, 0, 255], "源绿 → BGRA");
    }

    #[test]
    fn dib_decodes_bitinfoheader_bottom_up() {
      // 手工 2×2 24bpp DIB：BITMAPINFOHEADER + bottom-up 行（每行 4 字节对齐补 2 字节）
      let mut dib = Vec::new();
      dib.extend_from_slice(&40i32.to_le_bytes()); // biSize
      dib.extend_from_slice(&2i32.to_le_bytes()); // 宽
      dib.extend_from_slice(&2i32.to_le_bytes()); // 高（正 = bottom-up）
      dib.extend_from_slice(&1u16.to_le_bytes()); // planes
      dib.extend_from_slice(&24u16.to_le_bytes()); // bpp
      dib.extend_from_slice(&0i32.to_le_bytes()); // BI_RGB
      dib.extend_from_slice(&16i32.to_le_bytes()); // sizeImage
      dib.extend_from_slice(&[0u8; 16]); // 分辨率/色表字段
      // 下行（bottom-up 第一行）：蓝、白（BGR）
      dib.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
      // 上行：红、绿
      dib.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
      let img = decode_dib(&dib).expect("合法 DIB 应解码");
      assert_eq!((img.width(), img.height()), (2, 2));
      assert_eq!(img.get_pixel(0, 0).0, [255, 0, 0, 255], "左上 = 红");
      assert_eq!(img.get_pixel(1, 0).0, [0, 255, 0, 255], "右上 = 绿");
      assert_eq!(img.get_pixel(0, 1).0, [0, 0, 255, 255], "左下 = 蓝");
      assert!(decode_dib(&[0u8; 8]).is_err(), "垃圾数据报错不 panic");
    }
  }
}


