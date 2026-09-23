//! 颜色提取服务（6.1 颜色解析，PRD 04_文件预览 05-1 / 03_数据链接 02）：从缩略图提主色存
//! metadata.colors（#RRGGBB 按占比降序），详情 pinned 颜色 bar 显示。
//! 透明通道口径（2026-08-23 用户定）：alpha < ALPHA_MIN 的像素不参与统计——PNG 透明区 RGB
//! 常为黑/白杂色，混入会污染主色；半透明像素按原始 RGB 计入（不做预乘）。
//! 提取时机：导入 / 重生成缩略图时随缩略图管线算（320px 小图亚毫秒级）；存量资产打开详情时
//! 懒回填（ensure_asset_colors，被动补数不刷 modifiedAt、不进撤销）。
//! 取色源 = 卡片 / 详情显示图同源（2026-09-23 用户定）：无自身缩略图的工程资产（fla 等）
//! 经伴生缩略图代理（8.1 thumb_asset_id → hidden 资产）取其伴生 WebP 的缩略图取色——
//! 显示的是哪张图，主色就取自哪张。

use image::DynamicImage;

use crate::core::json_io::write_json;
use crate::core::library::Library;
use crate::core::paths;
use crate::services::thumbnail;

/// 存储上限（PRD：最多支持显示十个颜色）；详情 bar 槽位 7 个（01_主窗口稿），前端 slice(0, 7)
pub const COLOR_STORE_COUNT: usize = 10;

/// 透明通道口径：alpha 低于此值的像素不参与（近乎不可见；16 ≈ 6% 不透明度）
const ALPHA_MIN: u8 = 16;

/// 主色去重最小 RGB 距离²：小于此视为同色合并（56 ≈ 可分辨色档；同族明暗色各自成席，同 Eagle）
const MIN_DIST_SQ: i32 = 56 * 56;

/// 从图片提主色：4-4-4 RGB 桶直方图（4096 桶，桶内均值抗相近色散席）→ 按像素数降序
/// 贪心选取，与已选色距离 < MIN_DIST_SQ 的桶跳过。返回 #RRGGBB 大写 hex（占比降序，最多
/// COLOR_STORE_COUNT 个）；全透明 / 空图返回空数组。
pub fn extract(img: &DynamicImage) -> Vec<String> {
  let rgba = img.to_rgba8();
  // (像素数, Σr, Σg, Σb)
  let mut cells = [(0u32, 0u64, 0u64, 0u64); 4096];
  for p in rgba.pixels() {
    let [r, g, b, a] = p.0;
    if a < ALPHA_MIN {
      continue;
    }
    let idx = ((r as usize) >> 4) << 8 | ((g as usize) >> 4) << 4 | ((b as usize) >> 4);
    let e = &mut cells[idx];
    e.0 += 1;
    e.1 += r as u64;
    e.2 += g as u64;
    e.3 += b as u64;
  }
  let mut ranked: Vec<(u32, u8, u8, u8)> = cells
    .iter()
    .filter(|c| c.0 > 0)
    .map(|&(n, sr, sg, sb)| (n, (sr / n as u64) as u8, (sg / n as u64) as u8, (sb / n as u64) as u8))
    .collect();
  ranked.sort_by_key(|c| std::cmp::Reverse(c.0));

  let mut picked: Vec<(u8, u8, u8)> = Vec::new();
  for &(_, r, g, b) in &ranked {
    if picked.len() >= COLOR_STORE_COUNT {
      break;
    }
    let dup = picked.iter().any(|&(pr, pg, pb)| {
      let (dr, dg, db) = (r as i32 - pr as i32, g as i32 - pg as i32, b as i32 - pb as i32);
      dr * dr + dg * dg + db * db < MIN_DIST_SQ
    });
    if !dup {
      picked.push((r, g, b));
    }
  }
  picked.iter().map(|&(r, g, b)| format!("#{r:02X}{g:02X}{b:02X}")).collect()
}

/// 存量懒回填：metadata.colors 为空且有缩略图 → 解码缩略图提取并落盘（先落盘再同步索引，
/// 规范第 7 条）。**不刷 modified_at**（被动补数不算库内变更，防 3.5 排序键漂移）、不进撤销栈
/// （系统补数非用户写操作；后续用户写操作快照自然带上回填值）。全程 best-effort：任何一步
/// 失败仅日志，下次打开详情重试，不影响详情读取。
/// 取色源走伴生缩略图代理（8.1，同 views::placement_view 口径：thumb_asset_id 指向的 hidden
/// 资产优先于自身缩略图）——fla 等工程资产显示的就是伴生 WebP 的图，主色与其同源
/// （2026-09-23 用户定）；落盘恒写资产**自身** info 的 metadata.json，不污染伴生资产。
pub fn ensure_asset_colors(lib: &mut Library, asset_id: &str) {
  let Some(meta) = lib.index.assets.get(asset_id) else { return };
  if !meta.colors.is_empty() {
    return;
  }
  let source = meta
    .thumb_asset_id
    .as_deref()
    .and_then(|id| lib.index.assets.get(id))
    .filter(|t| t.hidden == Some(true))
    .unwrap_or(meta);
  let Some(thumb) = source.thumbnail.clone() else { return };
  let thumb_path = paths::info_dir(&lib.assets_dir, &source.id).join(&thumb);
  if !thumb_path.is_file() {
    return;
  }
  let Some(img) = thumbnail::decode_by_signature(&thumb_path) else {
    log::warn!("颜色回填：缩略图解码失败（{asset_id}）");
    return;
  };
  let colors = extract(&img);
  if colors.is_empty() {
    return; // 全透明图无有效主色——不写墓碑，下回重试（320px 小图开销可忽略）
  }
  let mut meta = meta.clone();
  meta.colors = colors;
  let dir = paths::info_dir(&lib.assets_dir, asset_id);
  if let Err(e) = write_json(&dir.join("metadata.json"), &meta) {
    log::warn!("颜色回填落盘失败（{asset_id}）：{e}");
    return;
  }
  lib.index.assets.insert(asset_id.to_string(), meta);
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use crate::core::models::AssetMeta;
  use image::{Rgba, RgbaImage};

  fn solid(w: u32, h: u32, px: Rgba<u8>) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, px))
  }

  #[test]
  fn extract_pure_color_single() {
    let colors = extract(&solid(64, 64, Rgba([200, 30, 30, 255])));
    assert_eq!(colors, vec!["#C81E1E"], "纯色图只出一色（大写 hex）");
  }

  /// 透明通道口径（2026-08-23 用户定）：透明像素的颜色（这里是黑）不得混入主色
  #[test]
  fn extract_skips_transparent_pixels() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(64, 64, |x, _| {
      if x < 32 {
        Rgba([0, 0, 0, 0]) // 左半全透明（RGB 为黑，混进来会抢头席）
      } else {
        Rgba([200, 30, 30, 255])
      }
    }));
    let colors = extract(&img);
    assert_eq!(colors, vec!["#C81E1E"], "全透明像素不计，黑色不得出现");
  }

  /// alpha 阈值边界：< 16 跳过，≥ 16 计入
  #[test]
  fn extract_alpha_threshold() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(64, 64, |x, _| {
      if x < 32 {
        Rgba([0, 0, 0, 15]) // 低于阈值：跳过
      } else {
        Rgba([200, 30, 30, 16]) // 达阈值：计入
      }
    }));
    let colors = extract(&img);
    assert_eq!(colors, vec!["#C81E1E"], "alpha 15 的黑不得混入");
  }

  #[test]
  fn extract_orders_by_dominance_and_dedups() {
    // 大红 9/16 + 近似红 3/16 + 蓝 1/4：纯红席位数压过近似红在首席；距离 < 56 的近似红合并
    let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(64, 64, |x, y| {
      if y < 48 {
        if x % 4 == 3 { Rgba([190, 34, 32, 255]) } else { Rgba([200, 30, 30, 255]) }
      } else {
        Rgba([30, 30, 200, 255])
      }
    }));
    let colors = extract(&img);
    assert_eq!(colors.len(), 2, "近似红合并一席 + 蓝一席：{colors:?}");
    assert_eq!(colors[0], "#C81E1E", "占比高的纯红在首席：{colors:?}");
    assert_eq!(colors[1], "#1E1EC8");
  }

  #[test]
  fn extract_caps_at_store_count() {
    // 16 档灰阶渐变 > 存储上限：截到 COLOR_STORE_COUNT
    let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(256, 1, |x, _| {
      let v = (x / 16 * 16) as u8;
      Rgba([v, v, v, 255])
    }));
    let colors = extract(&img);
    assert!(colors.len() <= COLOR_STORE_COUNT, "上限 {COLOR_STORE_COUNT}：实得 {}", colors.len());
    assert!(!colors.is_empty());
  }

  #[test]
  fn extract_all_transparent_empty() {
    let colors = extract(&solid(32, 32, Rgba([9, 9, 9, 0])));
    assert!(colors.is_empty(), "全透明图无有效主色");
  }

  /// 存量懒回填：空 colors + 有缩略图 → 落盘 + 索引同步 + 不刷 modifiedAt
  #[test]
  fn ensure_asset_colors_backfills_without_touching_modified_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    let meta: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-A", "name": "AN-A", "originName": "AN-A.png", "ext": "png", "size": 100,
      "importedAt": "2026-08-23T00:00:00+08:00", "modifiedAt": "2026-08-23T00:00:00+08:00",
      "thumbnail": "AN-A_thumbnail.webp"
    }))
    .unwrap();
    let info_dir = paths::info_dir(&lib.assets_dir, "AN-A");
    std::fs::create_dir_all(&info_dir).unwrap();
    write_json(&info_dir.join("metadata.json"), &meta).unwrap();
    // 缩略图：半透明黑 + 实红
    let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(64, 64, |x, _| {
      if x < 32 { Rgba([0, 0, 0, 0]) } else { Rgba([200, 30, 30, 255]) }
    }));
    thumbnail::save_webp(&img, &info_dir.join("AN-A_thumbnail.webp")).unwrap();
    lib.index.assets.insert("AN-A".into(), meta);

    ensure_asset_colors(&mut lib, "AN-A");
    let after = &lib.index.assets["AN-A"];
    // 有损 webp 往返 ±1 色偏：断言一席红系（黑未混入），不咬死精确 hex
    assert_eq!(after.colors.len(), 1, "索引回填一席（透明黑不混入）：{:?}", after.colors);
    assert!(after.colors[0].starts_with("#C8"), "红系主色：{:?}", after.colors);
    assert_eq!(after.modified_at, "2026-08-23T00:00:00+08:00", "被动回填不刷排序键");
    let disk: AssetMeta = read_json(&info_dir.join("metadata.json")).unwrap();
    assert_eq!(disk.colors, after.colors, "落盘与索引一致");

    // 幂等：已有 colors 不再动
    ensure_asset_colors(&mut lib, "AN-A");
    // 无缩略图引用 / 不存在的资产：静默返回
    ensure_asset_colors(&mut lib, "AN-不存在");
  }

  /// 伴生缩略图代理（2026-09-23 用户定）：fla 工程无自身缩略图 → 经 thumb_asset_id 取 hidden
  /// 伴生资产的 webp 缩略图取色；落盘写工程自身 info 的 metadata.json，不污染伴生资产
  #[test]
  fn ensure_asset_colors_backfills_from_companion_thumb() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    let fla: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-F", "name": "AN-F", "originName": "AN-F.fla", "ext": "fla", "size": 100,
      "importedAt": "2026-08-23T00:00:00+08:00", "modifiedAt": "2026-08-23T00:00:00+08:00",
      "thumbAssetId": "AN-T"
    }))
    .unwrap();
    let webp: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-T", "name": "AN-T", "originName": "AN-T.webp", "ext": "webp", "size": 100,
      "importedAt": "2026-08-23T00:00:00+08:00", "modifiedAt": "2026-08-23T00:00:00+08:00",
      "thumbnail": "AN-T_thumbnail.webp", "hidden": true
    }))
    .unwrap();
    // 伴生资产的缩略图文件（纯蓝）；工程自身 info 只有 metadata.json（无缩略图）
    let thumb_dir = paths::info_dir(&lib.assets_dir, "AN-T");
    std::fs::create_dir_all(&thumb_dir).unwrap();
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([30, 30, 200, 255])));
    thumbnail::save_webp(&img, &thumb_dir.join("AN-T_thumbnail.webp")).unwrap();
    let fla_dir = paths::info_dir(&lib.assets_dir, "AN-F");
    std::fs::create_dir_all(&fla_dir).unwrap();
    write_json(&fla_dir.join("metadata.json"), &fla).unwrap();
    lib.index.assets.insert("AN-F".into(), fla);
    lib.index.assets.insert("AN-T".into(), webp);

    ensure_asset_colors(&mut lib, "AN-F");
    let after = &lib.index.assets["AN-F"];
    // 有损 webp 往返 ±1 色偏：断言一席蓝系，不咬死精确 hex
    assert_eq!(after.colors.len(), 1, "伴生 webp 主色回填一席：{:?}", after.colors);
    assert!(after.colors[0].starts_with("#1E"), "蓝系主色：{:?}", after.colors);
    assert_eq!(after.modified_at, "2026-08-23T00:00:00+08:00", "被动回填不刷排序键");
    let disk: AssetMeta = read_json(&fla_dir.join("metadata.json")).unwrap();
    assert_eq!(disk.colors, after.colors, "落盘 = 工程自身 metadata");
    // 伴生资产不被污染：自身 colors 保持空、不补写它的 metadata.json
    assert!(lib.index.assets["AN-T"].colors.is_empty());
    assert!(!thumb_dir.join("metadata.json").exists(), "不得写伴生资产的 metadata");

    // 伴生资产未被 hidden 打标时不代理（同 views::placement_view 口径：非 hidden 不代理，
    // 防「还原为素材」途中脏引用取错图；失效引用由开库自检剔除）——工程保持无色，下回重试
    let mut plain = lib.index.assets["AN-T"].clone();
    plain.hidden = None;
    lib.index.assets.insert("AN-T".into(), plain);
    let mut fla2: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-G", "name": "AN-G", "originName": "AN-G.fla", "ext": "fla", "size": 100,
      "importedAt": "2026-08-23T00:00:00+08:00", "modifiedAt": "2026-08-23T00:00:00+08:00",
      "thumbAssetId": "AN-T"
    }))
    .unwrap();
    fla2.colors = Vec::new();
    lib.index.assets.insert("AN-G".into(), fla2);
    ensure_asset_colors(&mut lib, "AN-G");
    assert!(lib.index.assets["AN-G"].colors.is_empty(), "非 hidden 引用不代理取色");
  }
}
