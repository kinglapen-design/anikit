use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, Ordering}};

pub struct AnalysisControl {
  cancelled: Arc<AtomicBool>,
  paused: Mutex<bool>,
  wake: Condvar,
}

impl AnalysisControl {
  pub fn new() -> Self {
    Self { cancelled: Arc::new(AtomicBool::new(false)), paused: Mutex::new(false), wake: Condvar::new() }
  }

  pub fn cancel_flag(&self) -> Arc<AtomicBool> {
    self.cancelled.clone()
  }

  pub fn is_cancelled(&self) -> bool {
    self.cancelled.load(Ordering::Relaxed)
  }

  pub fn check(&self) -> Result<(), String> {
    {
      let mut paused = self.paused.lock().map_err(|e| e.to_string())?;
      while *paused {
        paused = self.wake.wait(paused).map_err(|e| e.to_string())?;
      }
    }
    if self.cancelled.load(Ordering::Relaxed) {
      return Err("分析已取消".into());
    }
    Ok(())
  }

  pub fn pause(&self) -> Result<(), String> {
    *self.paused.lock().map_err(|e| e.to_string())? = true;
    Ok(())
  }

  pub fn resume(&self) -> Result<(), String> {
    *self.paused.lock().map_err(|e| e.to_string())? = false;
    self.wake.notify_all();
    Ok(())
  }

  pub fn cancel(&self) -> Result<(), String> {
    self.cancelled.store(true, Ordering::Relaxed);
    *self.paused.lock().map_err(|e| e.to_string())? = false;
    self.wake.notify_all();
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pause_resume_and_cancel() {
    let control = AnalysisControl::new();
    control.pause().unwrap();
    control.resume().unwrap();
    assert!(control.check().is_ok());
    assert!(!control.cancel_flag().load(Ordering::Relaxed));
    control.cancel().unwrap();
    assert!(control.cancel_flag().load(Ordering::Relaxed));
    assert!(control.check().is_err());
  }
}
