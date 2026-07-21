//! 导入服务：文件 / 文件夹 → 封闭式素材库的完整管线（PRD 01-3、05-5）。
//!
//! 约定（本阶段新增，PRD 外补充）：
//! - Placement.folder 必须有值；根目录用保留 ID "root"（ROOT_FOLDER_ID）表示，
//!   Folder 列表中不放 root 节点。
//! - Keep 模式：被导入的根文件夹本身也作为层级起点建 Folder 节点。
//!
//! 纪律：metadata.json 由本模块写（每资源单文件单写点）；folders.json 不在此直接写，
//! 全部处理完统一走 core::library::save_folders 一次性落盘（单一写点归 core）。

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::json_io::write_json;
use crate::core::library::{self, Library, ASSTES_DIR};
use crate::core::models::{new_asset_id, AssetMeta, Folder, Placement, DATA_VERSION};
use crate::services::thumbnail;

/// 根目录的保留文件夹 ID：Placement.folder 用它表示根，Folder 列表不放 root 节点
pub const ROOT_FOLDER_ID: &str = "root";

/// 层级模式：Flatten 剔除层级平铺；Keep 保留源目录层级建文件夹节点
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Structure {
  #[serde(rename = "flatten")]
  Flatten,
  #[serde(rename = "keep")]
  Keep,
}

/// 来源模式：Copy 复制入库；Move 剪切入库（rename 优先，跨盘回退 复制+删除）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
  #[serde(rename = "copy")]
  Copy,
  #[serde(rename = "move")]
  Move,
}

/// 导入参数（target_folder 为目标文件夹 ID，None 表示根目录）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
  pub structure: Structure,
  pub source: Source,
  pub target_folder: Option<String>,
}

/// 单文件失败记录（reason 为中文人话，不给堆栈）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedItem {
  pub path: String,
  pub reason: String,
}

/// 导入结果汇总：新导入数、查重链接数、失败明细
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
  pub imported: u32,
  pub linked: u32,
  pub failed: Vec<FailedItem>,
}

/// 待导入文件：源路径 + Keep 模式目录链（Flatten 恒为空）
struct PendingFile {
  path: PathBuf,
  /// 相对被导入根文件夹的目录链（含根文件夹名本身），如 ["参考", "人物"]
  chain: Vec<String>,
}

/// 导入一批路径：逐文件建档 / 链接，单文件失败收集不中断，全部处理完一次性落盘 folders.json。
pub fn import_paths(lib: &mut Library, paths: &[PathBuf], opts: &ImportOptions) -> ImportOutcome {
  let target = opts.target_folder.clone().unwrap_or_else(|| ROOT_FOLDER_ID.to_string());
  let mut outcome = ImportOutcome { imported: 0, linked: 0, failed: Vec::new() };

  for file in collect(paths, opts.structure, &mut outcome.failed) {
    match import_one(lib, &file, &target, opts) {
      Ok(true) => outcome.linked += 1,
      Ok(false) => outcome.imported += 1,
      Err(reason) => {
        outcome.failed.push(FailedItem { path: file.path.to_string_lossy().into_owned(), reason })
      }
    }
  }

  // 统一落盘（写操作先落盘再更新内存：内存索引已在逐文件落 metadata 后同步，
  // folders.json 是批量尾部单次写，失败只记日志不中断——内存中数据仍在，下次导入会重试写）
  if let Err(e) = library::save_folders(lib) {
    log::error!("folders.json 落盘失败：{e}");
  }
  log::info!(
    "导入完成：新导入 {}、链接 {}、失败 {}",
    outcome.imported,
    outcome.linked,
    outcome.failed.len()
  );
  outcome
}

/// 展开源：文件直收；文件夹递归收集全部文件（自写递归，不引 walkdir）；
/// 路径不存在 / 不可访问直接进 failed。
fn collect(paths: &[PathBuf], structure: Structure, failed: &mut Vec<FailedItem>) -> Vec<PendingFile> {
  let mut out = Vec::new();
  for path in paths {
    if path.is_file() {
      out.push(PendingFile { path: path.clone(), chain: Vec::new() });
    } else if path.is_dir() {
      let chain = root_chain(path, structure);
      walk(path, chain, &mut out, failed);
    } else {
      failed.push(FailedItem {
        path: path.to_string_lossy().into_owned(),
        reason: "路径不存在或无法访问".to_string(),
      });
    }
  }
  out
}

/// Keep 模式目录链起点：被导入的根文件夹名本身（Flatten 为空链）
fn root_chain(dir: &Path, structure: Structure) -> Vec<String> {
  if structure != Structure::Keep {
    return Vec::new();
  }
  dir.file_name().map(|s| vec![s.to_string_lossy().into_owned()]).unwrap_or_default()
}

/// 递归收集文件夹下全部文件；子文件夹向目录链追加一级；文件夹读不了进 failed 继续
fn walk(dir: &Path, chain: Vec<String>, out: &mut Vec<PendingFile>, failed: &mut Vec<FailedItem>) {
  let entries = match fs::read_dir(dir) {
    Ok(e) => e,
    Err(e) => {
      failed
        .push(FailedItem { path: dir.to_string_lossy().into_owned(), reason: format!("文件夹读取失败：{e}") });
      return;
    }
  };
  for entry in entries.filter_map(|e| e.ok()) {
    let path = entry.path();
    if path.is_file() {
      out.push(PendingFile { path, chain: chain.clone() });
    } else if path.is_dir() {
      let mut sub = chain.clone();
      sub.push(entry.file_name().to_string_lossy().into_owned());
      walk(&path, sub, out, failed);
    }
  }
}

/// 导入单个文件：hash 查重命中建链接 Placement（不复制实体），未命中走完整建档。
/// 返回 Ok(true) 链接 / Ok(false) 新导入；Err 为中文失败原因。
fn import_one(lib: &mut Library, file: &PendingFile, target: &str, opts: &ImportOptions) -> Result<bool, String> {
  let hash = sha256_file(&file.path).map_err(|e| format!("文件读取失败：{e}"))?;

  // hash 查重：先取出已有资源 id 结束借用，再建链接 Placement（不复制实体）
  let linked_id = lib.index.assets.values().find(|a| a.hash == hash).map(|a| a.id.clone());
  if let Some(existing_id) = linked_id {
    let folder_id = resolve_folder(lib, target, &file.chain, opts)?;
    add_placement(lib, existing_id, folder_id, file_name(&file.path), false);
    return Ok(true);
  }

  let asset = build_asset(lib, file, &hash, opts)?;
  let folder_id = resolve_folder(lib, target, &file.chain, opts)?;
  add_placement(lib, asset.id.clone(), folder_id, file_name(&file.path), true);
  lib.index.assets.insert(asset.id.clone(), asset);
  Ok(false)
}

/// 单文件建档：建 info 文件夹 → 实体入库 → 探测宽高 → 缩略图 → 写 metadata.json
fn build_asset(lib: &Library, file: &PendingFile, hash: &str, opts: &ImportOptions) -> Result<AssetMeta, String> {
  let id = new_asset_id();
  let info_dir = lib.root.join(ASSTES_DIR).join(format!("{id}.info"));
  fs::create_dir_all(&info_dir).map_err(|e| format!("info 文件夹创建失败：{e}"))?;

  let origin_name = file_name(&file.path);
  let dest = info_dir.join(&origin_name);
  let size = fs::metadata(&file.path).map(|m| m.len()).unwrap_or(0);
  store_file(&file.path, &dest, opts.source)?;

  let (width, height) = thumbnail::probe(&dest).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None));
  let thumb = thumbnail::generate(&info_dir, &dest, file_stem(&file.path).as_str());
  let meta = make_meta(&id, file, &origin_name, size, hash, width, height, thumb);
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  Ok(meta)
}

/// 实体入库：Copy 复制；Move 优先 rename（同盘零拷贝），失败回退 复制+删除源（跨盘场景）
fn store_file(src: &Path, dest: &Path, source: Source) -> Result<(), String> {
  match source {
    Source::Copy => {
      fs::copy(src, dest).map_err(|e| format!("文件复制失败：{e}"))?;
    }
    Source::Move => {
      if fs::rename(src, dest).is_err() {
        fs::copy(src, dest).map_err(|e| format!("文件移动失败：{e}"))?;
        fs::remove_file(src).map_err(|e| format!("源文件删除失败：{e}"))?;
      }
    }
  }
  Ok(())
}

/// 组装 metadata.json 内容（PRD 01-2.3：只存资源固有属性，位置信息在 folders.json）
fn make_meta(
  id: &str,
  file: &PendingFile,
  origin_name: &str,
  size: u64,
  hash: &str,
  width: Option<u32>,
  height: Option<u32>,
  thumbnail: Option<String>,
) -> AssetMeta {
  // RFC3339 本地时间，与 PRD 示例一致（精确到秒，不带小数）
  let now = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  AssetMeta {
    version: DATA_VERSION,
    id: id.to_string(),
    name: file_stem(&file.path),
    origin_name: origin_name.to_string(),
    ext: file.path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default(),
    size,
    hash: hash.to_string(),
    width,
    height,
    duration: None,
    rating: 0,
    links: Vec::new(),
    imported_at: now.clone(),
    modified_at: now,
    tags: Vec::new(),
    note: String::new(),
    thumbnail,
    fps: None,
    frame_count: None,
    thumb_frame: None,
    r#type: None,
    source: None,
    start: None,
    end: None,
  }
}

/// 解析 Placement 落地文件夹：Keep 模式按目录链逐级 get-or-create，落链条末级
fn resolve_folder(lib: &mut Library, target: &str, chain: &[String], opts: &ImportOptions) -> Result<String, String> {
  if target != ROOT_FOLDER_ID && !lib.index.folders.contains_key(target) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  let mut parent = target.to_string();
  if opts.structure == Structure::Keep {
    for name in chain {
      parent = get_or_create_folder(lib, &parent, name);
    }
  }
  Ok(parent)
}

/// 同级同名文件夹复用；不存在则新建（order 取同级末尾），索引同步插入
fn get_or_create_folder(lib: &mut Library, parent: &str, name: &str) -> String {
  // root 保留 ID 在 Folder 表里对应 parent = null
  let parent_key = if parent == ROOT_FOLDER_ID { None } else { Some(parent.to_string()) };
  if let Some(f) = lib.folders.folders.iter().find(|f| f.parent == parent_key && f.name == name) {
    return f.id.clone();
  }
  let order = lib
    .folders
    .folders
    .iter()
    .filter(|f| f.parent == parent_key)
    .map(|f| f.order)
    .max()
    .map(|m| m + 1)
    .unwrap_or(0);
  let folder = Folder { id: new_folder_id(), name: name.to_string(), parent: parent_key, order };
  let id = folder.id.clone();
  lib.folders.folders.push(folder.clone());
  lib.index.folders.insert(id.clone(), folder);
  id
}

/// 加 Placement：同文件夹显示名唯一化（冲突追加 -序号），order 取文件夹内末尾，索引同步
fn add_placement(lib: &mut Library, asset_id: String, folder_id: String, name: String, origin: bool) {
  let existing: HashSet<String> = lib
    .index
    .placements_by_folder
    .get(&folder_id)
    .map(|v| v.iter().map(|p| p.name.clone()).collect())
    .unwrap_or_default();
  let order = lib.index.placements_by_folder.get(&folder_id).map(|v| v.len()).unwrap_or(0) as i32;
  let placement =
    Placement { asset: asset_id, folder: folder_id.clone(), name: library::unique_name(&existing, &name), order, origin };
  lib.folders.items.push(placement.clone());
  lib.index.placements_by_asset.entry(placement.asset.clone()).or_default().push(placement.clone());
  lib.index.placements_by_folder.entry(folder_id).or_default().push(placement);
}

/// 流式 sha256（64KB 缓冲，大文件不全量读入内存），输出 "sha256:{hex}"
fn sha256_file(path: &Path) -> std::io::Result<String> {
  let mut file = fs::File::open(path)?;
  let mut hasher = Sha256::new();
  let mut buf = [0u8; 64 * 1024];
  loop {
    let n = file.read(&mut buf)?;
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
  }
  Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// 生成文件夹 ID：`F` + 时间戳（yyMMddHHmm，时间有序）+ 3 位随机字符（防同秒冲突）。
/// 参照 new_asset_id 风格；假数据里的 F01/F02 是人工示例，实际生成用此规则。
fn new_folder_id() -> String {
  const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  use rand::Rng;
  let mut rng = rand::thread_rng();
  let suffix: String = (0..3).map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char).collect();
  format!("F{}-{suffix}", chrono::Local::now().format("%y%m%d%H%M"))
}

/// Placement 显示名 / metadata.originName：含扩展名的文件名
fn file_name(path: &Path) -> String {
  path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// metadata.name：去扩展名的文件名
fn file_stem(path: &Path) -> String {
  path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::models::FoldersFile;
  use crate::core::library::FOLDERS_FILE;

  /// 造一个空测试库（临时目录随用例结束自动清理）
  fn make_lib() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    (dir, lib)
  }

  fn opts(structure: Structure, source: Source) -> ImportOptions {
    ImportOptions { structure, source, target_folder: None }
  }

  /// 库内 info 文件夹数量
  fn info_dir_count(lib: &Library) -> usize {
    fs::read_dir(lib.root.join(ASSTES_DIR)).unwrap().count()
  }

  #[test]
  fn flatten_imports_folder_tree_flat() {
    let (dir, mut lib) = make_lib();
    let src = dir.path().join("素材");
    fs::create_dir_all(src.join("子目录")).unwrap();
    fs::write(src.join("a.txt"), "aaa").unwrap();
    fs::write(src.join("子目录/b.txt"), "bbb").unwrap();

    let out = import_paths(&mut lib, &[src], &opts(Structure::Flatten, Source::Copy));
    assert_eq!((out.imported, out.linked, out.failed.len()), (2, 0, 0));
    // 平铺：不建任何 Folder，两条 Placement 都落根目录
    assert!(lib.folders.folders.is_empty());
    assert_eq!(lib.index.placements_by_folder[ROOT_FOLDER_ID].len(), 2);
  }

  #[test]
  fn keep_builds_folder_nodes_from_source_tree() {
    let (dir, mut lib) = make_lib();
    let src = dir.path().join("参考");
    fs::create_dir_all(src.join("人物")).unwrap();
    fs::write(src.join("封面.png"), "x").unwrap();
    fs::write(src.join("人物/走路.fla"), "y").unwrap();

    let out = import_paths(&mut lib, &[src], &opts(Structure::Keep, Source::Copy));
    assert_eq!(out.imported, 2);
    // 层级含根文件夹本身：参考（顶层）→ 人物（子级）
    let top = lib.folders.folders.iter().find(|f| f.name == "参考" && f.parent.is_none()).unwrap();
    let sub = lib.folders.folders.iter().find(|f| f.name == "人物").unwrap();
    assert_eq!(sub.parent.as_deref(), Some(top.id.as_str()));
    // 链条末级落 Placement
    assert_eq!(lib.index.placements_by_folder[&sub.id].len(), 1);
    assert_eq!(lib.index.placements_by_folder[&top.id].len(), 1);
  }

  #[test]
  fn copy_keeps_source_move_removes_source() {
    let (dir, mut lib) = make_lib();
    let f1 = dir.path().join("复制.txt");
    let f2 = dir.path().join("剪切.txt");
    fs::write(&f1, "1").unwrap();
    fs::write(&f2, "2").unwrap();

    import_paths(&mut lib, &[f1.clone()], &opts(Structure::Flatten, Source::Copy));
    assert!(f1.exists(), "复制后源文件应仍在");

    import_paths(&mut lib, &[f2.clone()], &opts(Structure::Flatten, Source::Move));
    assert!(!f2.exists(), "剪切后源文件应消失");
  }

  #[test]
  fn same_hash_links_without_duplicating_entity() {
    let (dir, mut lib) = make_lib();
    let f1 = dir.path().join("甲.txt");
    let f2 = dir.path().join("乙.txt");
    fs::write(&f1, "相同内容").unwrap();
    fs::write(&f2, "相同内容").unwrap();

    let out = import_paths(&mut lib, &[f1, f2], &opts(Structure::Flatten, Source::Copy));
    assert_eq!((out.imported, out.linked), (1, 1));
    // 实体只复制一次：info 文件夹唯一，Placement 两条且只有一条 origin
    assert_eq!(info_dir_count(&lib), 1);
    let origins = lib.folders.items.iter().filter(|p| p.origin).count();
    assert_eq!((lib.folders.items.len(), origins), (2, 1));
  }

  #[test]
  fn duplicate_display_name_gets_counter_suffix() {
    let (dir, mut lib) = make_lib();
    let d1 = dir.path().join("源一");
    let d2 = dir.path().join("源二");
    fs::create_dir_all(&d1).unwrap();
    fs::create_dir_all(&d2).unwrap();
    fs::write(d1.join("男人.fla"), "内容一").unwrap();
    fs::write(d2.join("男人.fla"), "内容二").unwrap();

    let out = import_paths(&mut lib, &[d1, d2], &opts(Structure::Flatten, Source::Copy));
    assert_eq!(out.imported, 2);
    let names: Vec<&str> = lib.folders.items.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"男人.fla"));
    assert!(names.contains(&"男人-1.fla"));
  }

  #[test]
  fn mixed_files_and_folder_and_bad_path() {
    let (dir, mut lib) = make_lib();
    let single = dir.path().join("单文件.txt");
    fs::write(&single, "s").unwrap();
    let folder = dir.path().join("夹");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("内.txt"), "i").unwrap();
    let missing = dir.path().join("不存在.txt");

    let out = import_paths(&mut lib, &[single, folder, missing], &opts(Structure::Keep, Source::Copy));
    assert_eq!(out.imported, 2);
    assert_eq!(out.failed.len(), 1);
    assert!(out.failed[0].reason.contains("不存在"));
  }

  #[test]
  fn import_writes_metadata_and_persists_folders() {
    let (dir, mut lib) = make_lib();
    let png = dir.path().join("图.PNG");
    image::RgbaImage::from_pixel(64, 32, image::Rgba([0, 255, 0, 255])).save(&png).unwrap();

    let out = import_paths(&mut lib, &[png], &opts(Structure::Flatten, Source::Copy));
    assert_eq!(out.imported, 1);

    // metadata.json：宽高探测 + 缩略图 + ext 小写不含点
    let asset = lib.index.assets.values().next().unwrap();
    assert_eq!((asset.width, asset.height), (Some(64), Some(32)));
    assert_eq!(asset.ext, "png");
    assert!(asset.thumbnail.is_some());
    assert!(asset.hash.starts_with("sha256:"));
    let info = lib.root.join(ASSTES_DIR).join(format!("{}.info", asset.id));
    assert!(info.join("metadata.json").is_file());
    assert!(info.join(asset.thumbnail.as_ref().unwrap()).is_file());

    // folders.json 已落盘且版本合法
    let back: FoldersFile = read_json(&lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(back.items.len(), 1);
    assert!(back.items[0].origin);
  }
}
