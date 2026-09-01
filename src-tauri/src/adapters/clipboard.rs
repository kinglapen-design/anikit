//! 系统剪贴板适配器（规范第 2 条；8.3 发送至 AN）：Win32 私有格式写入。
//! Animate 元件剪贴板格式 = 注册格式 "Flash 11 Picture"（内容为 XFL 包字节，
//! spike/clipboard-probe 实证）；arboard 只支持标准格式（文本/图片），私有格式直调 Win32。
//! 业务语义（fla → 载荷转换）在 services/send_stage，本层只管字节写入。

#[cfg(windows)]
pub use imp::write_flash_picture;

#[cfg(not(windows))]
pub fn write_flash_picture(_data: &[u8]) -> Result<(), String> {
  Err("当前平台不支持发送至 Animate".to_string())
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
}
