//! assets 池路径推导单点（PRD 01_储存空间「assets 池访问纪律」，2026-08-12 定案）：
//! 素材平铺为 `assets/{ID}.info` 不分桶；`ID → info 路径`的推导只经本模块函数，
//! 所有调用点禁止自行拼路径——为未来可能的分桶改造留单点改造口。

use std::path::{Path, PathBuf};

/// 统一素材池目录名（空间级；旧库根的 asstes/ 拼写遗留随储存空间层正名）
pub const ASSETS_DIR: &str = "assets";

/// 空间根 → assets 池目录
pub fn assets_dir(space_root: &Path) -> PathBuf {
  space_root.join(ASSETS_DIR)
}

/// 资源 ID → info 文件夹（`assets/{ID}.info`）
pub fn info_dir(assets_dir: &Path, id: &str) -> PathBuf {
  assets_dir.join(format!("{id}.info"))
}

/// 资源 ID → info 内 metadata.json
pub fn metadata_file(assets_dir: &Path, id: &str) -> PathBuf {
  info_dir(assets_dir, id).join("metadata.json")
}

/// 资源 ID → info 内 frames/ 目录（序列帧 / 动图解包逐帧高清图）
pub fn frames_dir(assets_dir: &Path, id: &str) -> PathBuf {
  info_dir(assets_dir, id).join("frames")
}

/// 资源 ID + 磁盘原始文件名 → 实体文件（`assets/{ID}.info/{originName}`）
pub fn origin_file(assets_dir: &Path, id: &str, origin_name: &str) -> PathBuf {
  info_dir(assets_dir, id).join(origin_name)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn derives_info_paths_from_id() {
    let assets = Path::new("D:/空间/assets");
    assert_eq!(assets_dir(Path::new("D:/空间")), assets.to_path_buf());
    assert_eq!(info_dir(assets, "AN2607211305-X7K2Q"), assets.join("AN2607211305-X7K2Q.info"));
    assert_eq!(
      metadata_file(assets, "AN-1"),
      assets.join("AN-1.info").join("metadata.json")
    );
    assert_eq!(frames_dir(assets, "AN-1"), assets.join("AN-1.info").join("frames"));
    assert_eq!(origin_file(assets, "AN-1", "人物.fla"), assets.join("AN-1.info").join("人物.fla"));
  }
}
