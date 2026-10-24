//! 对外媒体端点的图片源解析。by-name 链接先于自动导入产生，查询需覆盖导入途中的短暂未命中。

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tokio::time::{Instant, sleep};

use crate::core::library::Library;
use crate::core::models::AssetMeta;
use crate::services::md_bundle::DIRECT_IMAGE_EXTS;
use crate::services::{asset_menu, views};

const BY_NAME_WAIT: Duration = Duration::from_secs(10);
const BY_NAME_POLL: Duration = Duration::from_millis(100);

pub type ImageSource = (PathBuf, &'static str);

#[derive(Debug)]
pub enum ByNameError {
  Lock,
  NoLibrary,
  NotFound,
}

/// 未命中时每 100ms 短锁重查，最多等 10 秒；锁不跨等待，文件读取由 HTTP 层在锁外完成。
pub async fn by_name_image(library: &Mutex<Option<Library>>, name: &str) -> Result<Option<ImageSource>, ByNameError> {
  let deadline = Instant::now() + BY_NAME_WAIT;
  loop {
    let found = {
      let guard = library.lock().map_err(|_| ByNameError::Lock)?;
      let lib = guard.as_ref().ok_or(ByNameError::NoLibrary)?;
      lib.index.latest_by_origin_name(name, &lib.recycle).map(|meta| image_source(lib, &meta.id, meta))
    };
    match found {
      Some(source) => return Ok(source),
      None if Instant::now() >= deadline => return Err(ByNameError::NotFound),
      None => sleep(BY_NAME_POLL).await,
    }
  }
}

/// 直渲扩展名走 origin 字节；其余走 thumbAssetId 伴生代理缩略图（同 views::placement_view 口径）。
pub fn image_source(lib: &Library, id: &str, meta: &AssetMeta) -> Option<ImageSource> {
  if DIRECT_IMAGE_EXTS.contains(&meta.ext.to_ascii_lowercase().as_str()) {
    asset_menu::real_origin_file(lib, id).ok().map(|p| (p, mime_of(&meta.ext)))
  } else {
    let thumb_source = meta
      .thumb_asset_id
      .as_deref()
      .and_then(|i| lib.index.assets.get(i))
      .filter(|t| t.hidden == Some(true))
      .unwrap_or(meta);
    views::thumb_path(lib, thumb_source, &lib.format_icons).map(|p| (PathBuf::from(p), "image/webp"))
  }
}

fn mime_of(ext: &str) -> &'static str {
  match ext.to_ascii_lowercase().as_str() {
    "png" => "image/png",
    "jpg" | "jpeg" => "image/jpeg",
    "gif" => "image/gif",
    "webp" => "image/webp",
    "bmp" => "image/bmp",
    "avif" => "image/avif",
    _ => "application/octet-stream",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::{library, space};
  use crate::services::import::{ImportOptions, SequenceMode, Source, Structure};
  use crate::services::import_task::run_import;
  use std::sync::Arc;
  use std::sync::atomic::AtomicBool;

  #[tokio::test]
  async fn by_name_waits_for_import_and_returns_real_image() {
    let dir = tempfile::tempdir().unwrap();
    let mut space = space::create(dir.path(), "空间", false).unwrap();
    let lib = library::create(&mut space, "库").unwrap();
    let root = lib.root.clone();
    let assets_dir = lib.assets_dir.clone();
    let library = Arc::new(Mutex::new(Some(lib)));
    let source_dir = tempfile::tempdir().unwrap();
    let source = source_dir.path().join("迟到.png");
    image::RgbImage::from_pixel(1, 1, image::Rgb([255, 0, 0])).save(&source).unwrap();
    let expected = std::fs::read(&source).unwrap();

    let importer_library = library.clone();
    let importer_source = source.clone();
    let importer = std::thread::spawn(move || {
      std::thread::sleep(Duration::from_millis(300));
      let opts = ImportOptions {
        structure: Structure::Keep,
        source: Source::Copy,
        target_folder: None,
        paths: vec![importer_source],
        imported_from: None,
        sequence_mode: SequenceMode::Scatter,
      };
      let outcome = run_import(&importer_library, root, assets_dir, opts, |_| {}, &AtomicBool::new(false));
      assert_eq!(outcome.imported, 1);
      assert!(outcome.failed.is_empty());
    });

    let (path, mime) = by_name_image(&library, "迟到.png").await.unwrap().unwrap();
    importer.join().unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(std::fs::read(path).unwrap(), expected);
  }
}
