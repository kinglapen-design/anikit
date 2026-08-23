//! 导入管线原语与公开类型：扫描展开、sha256、实体入库、metadata 建档、文件夹 / Placement 登记。
//! 本模块不持锁、不编排任务；执行编排（分段锁 / 事件 / 取消 / 落盘节流）在 import_task。
//!
//! 约定（本阶段新增，PRD 外补充）：
//! - Placement.folder 必须有值；根目录用保留 ID "root"（ROOT_FOLDER_ID）表示，
//!   Folder 列表中不放 root 节点。
//! - Keep 模式：被导入的根文件夹本身也作为层级起点建 Folder 节点。
//!
//! 纪律：metadata.json 由 build_asset 写（每资源单文件单写点，路径由 ID 推导无需共享状态）；
//! folders.json 不在此写，统一由 import_task 经 core::library::save_folders 节流落盘。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::json_io::{read_json, write_json};
use crate::core::library::{self, Library};
use crate::core::models::{AssetMeta, Folder, Placement, DATA_VERSION};
use crate::core::paths;
use crate::services::thumbnail;
use crate::services::views::PlacementView;

/// 根目录的保留文件夹 ID（定义在 core/models.rs，此处转出保既有引用路径）
pub use crate::core::models::ROOT_FOLDER_ID;

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

/// 序列帧模式（4.1）：Pack 检测到的序列帧打包为一个资源；Scatter 逐文件导入（默认，
/// 剪贴板 / 收件箱导入恒 Scatter）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SequenceMode {
  #[serde(rename = "pack")]
  Pack,
  #[default]
  #[serde(rename = "scatter")]
  Scatter,
}

/// 导入参数（target_folder 为目标文件夹 ID，None 表示根目录；paths 为展开前原始路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
  pub structure: Structure,
  pub source: Source,
  pub target_folder: Option<String>,
  #[serde(default)]
  pub paths: Vec<PathBuf>,
  /// 导入渠道标记（写入 metadata.importedFrom）："inbox" = 收件箱导入（含自动与手动），
  /// 服务快速访问"已导入"区块口径与摘标语义；撞 hash 一律走正常重复提示（冲突挂起），
  /// 不再区分渠道——同渠道静默合并 2026-08-13 用户定案废除（原服务 RECOVER 场景，已随
  /// 工程文件不自动导入而消解）
  #[serde(default)]
  pub imported_from: Option<String>,
  /// 序列帧处理模式（仅 UI 导入弹窗给 Pack；收件箱 / 剪贴板恒 Scatter）
  #[serde(default)]
  pub sequence_mode: SequenceMode,
}

/// 单文件失败记录（reason 为中文人话，不给堆栈）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedItem {
  pub path: String,
  pub reason: String,
}

/// 序列帧源信息（4.1）：冲突挂起项携带，前端原样回传给 resolve_conflicts，
/// "保留两个"决策按此重新打包建档（帧清单按序号升序）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceSource {
  /// 基础名（资产标题）
  pub name: String,
  pub ext: String,
  /// 序号位宽（入库补齐用）
  pub width: u32,
  pub frames: Vec<SequenceFrameSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceFrameSource {
  pub path: PathBuf,
  pub number: u32,
}

/// hash 冲突挂起项（PRD 10_重复添加提示：导入中不询问，finished 后由前端逐项弹窗决策）。
/// 直接携带弹窗左右对照（已存在 vs 新文件）全部显示数据，前端无需再发查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictItem {
  /// 源文件绝对路径
  pub source_path: PathBuf,
  /// 源文件名（含扩展名）；序列帧冲突为 `标题.扩展名`
  pub source_name: String,
  /// 源文件字节数
  pub source_size: u64,
  /// 源文件图片宽高（非图片 None）
  pub source_w: Option<u32>,
  pub source_h: Option<u32>,
  /// 源文件预览缓存路径（webview 直渲不了的格式经后端解码生成，cache/tmp；图片格式 None 走前端直读源文件）
  pub source_thumb_path: Option<String>,
  /// 落点文件夹 ID（"root" 或 F 开头；keep 模式为目录链末级）
  pub target_folder: String,
  /// 落点文件夹显示名（"root" → "根目录"）
  pub target_folder_name: String,
  /// 库内 hash 相同的已有资源
  pub existing_asset_id: String,
  /// 已有资源显示名（其 origin Placement 名；无 origin 则任一 Placement 名）
  pub existing_name: String,
  pub existing_w: Option<u32>,
  pub existing_h: Option<u32>,
  pub existing_size: u64,
  pub existing_ext: String,
  /// 已有资源缩略图绝对路径（无则 None）
  pub existing_thumb_path: Option<String>,
  /// origin Placement 所在文件夹显示名（root → "根目录"）
  pub existing_folder_name: String,
  /// 序列帧冲突的组信息（普通文件冲突无此字段；前端原样回传供"保留两个"重新打包）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sequence: Option<SequenceSource>,
  /// 本批次的导入渠道标记（inbox 等；前端原样回传——"保留两个"建档的新资产继承该标记，
  /// 否则自动导入冲突的新资产丢标不进"已导入"视图，2026-08-13 用户实测"以为文件丢失"）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub imported_from: Option<String>,
}

/// 导入结果汇总：新导入数、冲突挂起、失败明细
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
  pub imported: u32,
  pub conflicts: Vec<ConflictItem>,
  pub failed: Vec<FailedItem>,
}

/// 导入进度事件（工作线程 → 前端 Channel；serde tag="type" 小驼峰判别）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ImportEvent {
  /// 扫描完成，总数已知
  Started { total: u32 },
  /// keep 模式层级已建好（前端重拉文件夹树）；flatten 不发
  Folders,
  /// 每导入一个推一个（流式：到一个显示一个）；冲突挂起项不发 Asset
  /// （Box：PlacementView 较大，避免枚举整体膨胀——serde 序列化形状不变）
  Asset { placement: Box<PlacementView> },
  /// 全部处理完（含取消中断），数量为已实际处理数；conflicts 待 resolve_conflicts 决策
  Finished { imported: u32, conflicts: Vec<ConflictItem>, failed: Vec<FailedItem> },
}

/// 待导入文件：源路径 + Keep 模式目录链（Flatten 恒为空）
pub(crate) struct PendingFile {
  pub path: PathBuf,
  /// 相对被导入根文件夹的目录链（含根文件夹名本身），如 ["参考", "人物"]
  pub chain: Vec<String>,
}

/// 展开源：文件直收；文件夹递归收集全部文件（自写递归，不引 walkdir）；
/// 路径不存在 / 不可访问直接进 failed。纯文件系统操作，不碰共享状态（锁外调用）。
pub(crate) fn collect(paths: &[PathBuf], structure: Structure, failed: &mut Vec<FailedItem>) -> Vec<PendingFile> {
  let mut out = Vec::new();
  for path in paths {
    if path.is_file() {
      out.push(PendingFile { path: path.clone(), chain: Vec::new() });
    } else if path.is_dir() {
      let chain = root_chain(path, structure);
      walk(path, chain, structure, &mut out, failed);
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

/// 递归收集文件夹下全部文件；Keep 模式子文件夹向目录链追加一级（Flatten 链恒为空）；
/// 文件夹读不了进 failed 继续
fn walk(
  dir: &Path,
  chain: Vec<String>,
  structure: Structure,
  out: &mut Vec<PendingFile>,
  failed: &mut Vec<FailedItem>,
) {
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
      if structure == Structure::Keep {
        sub.push(entry.file_name().to_string_lossy().into_owned());
      }
      walk(&path, sub, structure, out, failed);
    }
  }
}

/// 单文件建档（锁外重活）：建 info 文件夹 → 实体入库 → 探测宽高 → 缩略图 → 写 metadata.json。
/// 路径全由新 ID 推导（core::paths 单点），不依赖共享状态；
/// 使用预分配的 asset id（并行管线在阶段 2 查重时统一分配），并携带导入渠道标记（inbox 导入写 metadata.importedFrom）
pub(crate) fn build_asset_with_id(
  assets_dir: &Path,
  file: &PendingFile,
  hash: &str,
  asset_id: &str,
  source: Source,
  imported_from: Option<&str>,
) -> Result<AssetMeta, String> {
  let info_dir = paths::info_dir(assets_dir, asset_id);
  fs::create_dir_all(&info_dir).map_err(|e| format!("info 文件夹创建失败：{e}"))?;

  let origin_name = file_name(&file.path);
  let dest = info_dir.join(&origin_name);
  let size = fs::metadata(&file.path).map(|m| m.len()).unwrap_or(0);
  store_file(&file.path, &dest, source)?;

  // 5.1：动图（GIF/WEBP 动图）→ 视频 → 音频依次探测（按扩展名门禁，普通文件零开销）；
  // sidecar 缺失 / 解码失败全部 None，退化为现状（格式图标保底），导入不失败
  let anim = crate::services::anim::probe(&dest);
  let video = crate::services::media::probe_video(&dest);
  let audio = crate::services::media::probe_audio(&dest);
  let (width, height) = if let Some(a) = anim {
    (Some(a.width), Some(a.height))
  } else if let Some(v) = video {
    (Some(v.width), Some(v.height))
  } else if audio.is_some() || crate::services::media::is_audio(&dest) {
    // 音频宽高 = 波形缩略图尺寸（布局用，2026-07-26）：与探测解耦——ffprobe 探测失败
    // （duration null）但波形成功时，卡片也按宽比例布局，不落 1:1 方盒 cover 裁切
    (Some(crate::services::media::WAVEFORM_WIDTH), Some(crate::services::media::WAVEFORM_HEIGHT))
  } else {
    thumbnail::probe(&dest).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None))
  };
  let thumb = thumbnail::generate(&info_dir, &dest, file_stem(&file.path).as_str());
  let mut meta = make_meta(asset_id, file, &origin_name, size, hash, width, height, thumb.as_ref().map(|t| t.name.clone()), imported_from);
  meta.thumb_width = thumb.as_ref().map(|t| t.width);
  meta.thumb_height = thumb.as_ref().map(|t| t.height);
  meta.duration = anim.map(|a| a.duration_secs).or(video.map(|v| v.duration)).or(audio.map(|a| a.duration));
  meta.frame_count = anim.map(|a| a.frame_count);
  meta.bpm = audio.and_then(|a| a.bpm);
  // 视频帧率（5.3 帧化改造，2026-07-28 用户定：帧数优先显示）；序列帧 fps 仍由 sequence 管线写
  meta.fps = video.and_then(|v| v.fps);
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  Ok(meta)
}

/// 改名原语（6.1 rename_placement 用，operations/duplicate.rs）：磁盘 originName 与
/// 词干缩略图（{stem}_thumbnail.webp/png）同步改名，metadata 更新（originName / name /
/// thumbnail / modifiedAt）并写盘。**序列帧资产**：origin 是 frames/ 内封面帧（根无实体），
/// 改走 sequence::rename_frames 整组帧文件改词干（thumb_frame / origin_name 同步）。
/// 全部在锁外执行（单资源单文件）；
/// 调用方负责锁内索引同步与撤销登记（renames 成对反转即 DiskOp::Rename）。
pub(crate) struct OriginRename {
  pub meta: AssetMeta,
  /// 实际执行的磁盘改名（旧路径, 新路径），按执行顺序（撤销登记用；空 = 新旧同名未改）
  pub renames: Vec<(PathBuf, PathBuf)>,
}

pub(crate) fn rename_origin(assets_dir: &Path, asset_id: &str, new_name: &str) -> Result<OriginRename, String> {
  let info_dir = paths::info_dir(assets_dir, asset_id);
  let meta_path = info_dir.join("metadata.json");
  let mut meta: AssetMeta = read_json(&meta_path).map_err(|e| e.to_string())?;
  let mut renames = Vec::new();
  if let Some(frames_dir) = crate::services::sequence::frames_dir_if_sequence(assets_dir, asset_id, &meta) {
    // 序列帧：origin 是 frames/ 内封面帧（info 根无实体），整组帧文件同步改词干
    crate::services::sequence::rename_frames(&frames_dir, &mut meta, new_name, &mut renames)?;
  } else if meta.origin_name != new_name {
    let old_origin = info_dir.join(&meta.origin_name);
    let new_origin = info_dir.join(new_name);
    if old_origin.exists() {
      fs::rename(&old_origin, &new_origin).map_err(|e| format!("文件改名失败：{e}"))?;
      renames.push((old_origin, new_origin));
    }
    meta.origin_name = new_name.to_string();
    meta.name = Path::new(new_name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
  }
  // 词干缩略图同步改名并重新登记（普通 / 序列帧共用；新旧同名自动跳过）
  if let Some(thumb) = meta.thumbnail.clone() {
    let stem = Path::new(new_name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = Path::new(&thumb).extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "webp".into());
    let new_thumb = format!("{stem}_thumbnail.{ext}");
    let old_thumb = info_dir.join(&thumb);
    if thumb != new_thumb && old_thumb.exists() {
      let new_thumb_path = info_dir.join(&new_thumb);
      fs::rename(&old_thumb, &new_thumb_path).map_err(|e| format!("缩略图改名失败：{e}"))?;
      renames.push((old_thumb, new_thumb_path));
      meta.thumbnail = Some(new_thumb);
    }
  }
  meta.modified_at = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  write_json(&meta_path, &meta).map_err(|e| e.to_string())?;
  Ok(OriginRename { meta, renames })
}

/// 实体入库：Copy 复制；Move 优先 rename（同盘零拷贝），失败回退 复制+删除源（跨盘场景）
pub(crate) fn store_file(src: &Path, dest: &Path, source: Source) -> Result<(), String> {
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

/// 源文件磁盘创建时间（RFC3339 本地秒，与 imported_at 同格式）：读取失败 / 平台不支持为 None。
/// 须在入库（store_file 复制）前对源路径调用——池内副本的 birth time 是复制时刻，无意义
pub(crate) fn source_created_at(path: &Path) -> Option<String> {
  let t = fs::metadata(path).ok()?.created().ok()?;
  Some(chrono::DateTime::<chrono::Local>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
}

/// 组装 metadata.json 内容（PRD 01-2.3：只存资源固有属性，位置信息在 folders.json）
#[allow(clippy::too_many_arguments)]
fn make_meta(
  id: &str,
  file: &PendingFile,
  origin_name: &str,
  size: u64,
  hash: &str,
  width: Option<u32>,
  height: Option<u32>,
  thumbnail: Option<String>,
  imported_from: Option<&str>,
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
    thumb_width: None,
    thumb_height: None,
    duration: None,
    bpm: None,
    rating: 0,
    links: Vec::new(),
    imported_at: now.clone(),
    imported_from: imported_from.map(|s| s.to_string()),
    // 剪贴板导入的源 = cache/ 临时文件（刚创建），created 即建档时刻，语义自洽
    created_at: source_created_at(&file.path),
    modified_at: now,
    tags: Vec::new(),
    note: String::new(),
    descriptions: Vec::new(),
    note_count: None,
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

/// 纯查找：按目录链定位 Placement 落地文件夹（keep 层级已由 import_task 批量建好）
pub(crate) fn find_folder(lib: &Library, target: &str, chain: &[String]) -> Option<String> {
  let mut parent = target.to_string();
  for name in chain {
    parent = find_child_id(lib, &parent, name)?;
  }
  Some(parent)
}

/// 同级（parent）下按名查文件夹 ID；root 保留 ID 对应 parent = null
fn find_child_id(lib: &Library, parent: &str, name: &str) -> Option<String> {
  let parent_key = if parent == ROOT_FOLDER_ID { None } else { Some(parent) };
  lib.folders.folders.iter().find(|f| f.parent.as_deref() == parent_key && f.name == name).map(|f| f.id.clone())
}

/// keep 模式批量建层级：目录链逐级 get-or-create（同级同名复用）；返回是否新建了节点
pub(crate) fn ensure_chain(lib: &mut Library, target: &str, chain: &[String]) -> bool {
  let mut created = false;
  let mut parent = target.to_string();
  for name in chain {
    match find_child_id(lib, &parent, name) {
      Some(id) => parent = id,
      None => {
        parent = create_folder(lib, &parent, name);
        created = true;
      }
    }
  }
  created
}

/// 新建文件夹（order 取同级末尾），索引同步插入
pub(crate) fn create_folder(lib: &mut Library, parent: &str, name: &str) -> String {
  let parent_key = if parent == ROOT_FOLDER_ID { None } else { Some(parent.to_string()) };
  let order = lib
    .folders
    .folders
    .iter()
    .filter(|f| f.parent == parent_key)
    .map(|f| f.order)
    .max()
    .map(|m| m + 1)
    .unwrap_or(0);
  let folder = Folder { id: new_folder_id(), name: name.to_string(), parent: parent_key, block: None, icon: None, kind: None, color: None, groups: None, order };
  let id = folder.id.clone();
  lib.folders.folders.push(folder.clone());
  lib.index.folders.insert(id.clone(), folder);
  id
}

/// 加 Placement（**前插**，2026-07-25 用户定：新导入的素材排在最前，不用下拉滚轮找）：
/// 同文件夹显示名唯一化（冲突追加 -序号）；front_anchor 记录各文件夹本批已前插数——
/// 文件夹视图（placements_by_folder）按锚点段首前插、批内保序；
/// 全部视图（folders.items）全局前插：插入位 = 本批已插总数（各文件夹锚点之和），
/// 异文件夹混批也按到达序从头部依次后移（2026-08-20 复验修复：原按目标文件夹段首定位，
/// 目标段不在头部时全部视图呈中间插入）；新批次锚点从 0 起排在旧批次之前；
/// order 向负生长（仅作相对键、当前无消费者，7.1 排序落地时统一重排），索引同步
pub(crate) fn add_placement(
  lib: &mut Library,
  asset_id: String,
  folder_id: String,
  name: String,
  origin: bool,
  front_anchor: &mut HashMap<String, usize>,
) -> Placement {
  add_placement_impl(lib, asset_id, folder_id, name, origin, front_anchor, true)
}

/// 同 add_placement 但**不做同名单一化**（2026-08-14 用户定：冲突"保留两个"允许同名——
/// 库本就允许同名文件存在，自动加序号删除，要什么名字用户自己改）
pub(crate) fn add_placement_keep_name(
  lib: &mut Library,
  asset_id: String,
  folder_id: String,
  name: String,
  origin: bool,
  front_anchor: &mut HashMap<String, usize>,
) -> Placement {
  add_placement_impl(lib, asset_id, folder_id, name, origin, front_anchor, false)
}

#[allow(clippy::too_many_arguments)]
fn add_placement_impl(
  lib: &mut Library,
  asset_id: String,
  folder_id: String,
  name: String,
  origin: bool,
  front_anchor: &mut HashMap<String, usize>,
  unique: bool,
) -> Placement {
  let display = if unique {
    let existing: HashSet<String> = lib
      .index
      .placements_by_folder
      .get(&folder_id)
      .map(|v| v.iter().map(|p| p.name.clone()).collect())
      .unwrap_or_default();
    library::unique_name(&existing, &name)
  } else {
    name
  };
  // items 全局前插（全部视图 = items 顺序）：插入位 = 本批此前已插总数（各文件夹锚点之和），
  // 异文件夹混批也按到达序从头部依次后移；新批次锚点清零、整批排在旧批次之前
  let pos: usize = front_anchor.values().sum();
  let batch = front_anchor.entry(folder_id.clone()).or_insert(0);
  let order = lib
    .index
    .placements_by_folder
    .get(&folder_id)
    .and_then(|v| v.first())
    .map(|f| f.order - 1)
    .unwrap_or(0);
  let placement = Placement { asset: asset_id, folder: folder_id.clone(), name: display, order, origin };
  lib.folders.items.insert(pos, placement.clone());
  lib.index.placements_by_asset.entry(placement.asset.clone()).or_default().push(placement.clone());
  lib.index.placements_by_folder.entry(folder_id).or_default().insert(*batch, placement.clone());
  *batch += 1;
  placement
}

/// 流式 sha256（64KB 缓冲，大文件不全量读入内存），输出 "sha256:{hex}"（锁外调用）
pub(crate) fn sha256_file(path: &Path) -> std::io::Result<String> {
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
pub(crate) fn new_folder_id() -> String {
  const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  use rand::Rng;
  let mut rng = rand::thread_rng();
  let suffix: String = (0..3).map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char).collect();
  format!("F{}-{suffix}", chrono::Local::now().format("%y%m%d%H%M"))
}

/// Placement 显示名 / metadata.originName：含扩展名的文件名
pub(crate) fn file_name(path: &Path) -> String {
  path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// metadata.name：去扩展名的文件名
fn file_stem(path: &Path) -> String {
  path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn add_placement_inserts_front_with_batch_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    let mut anchor = HashMap::new();
    // 同批三个：批内保序（到达顺序即显示顺序）
    add_placement(&mut lib, "A1".into(), "root".into(), "a.png".into(), true, &mut anchor);
    add_placement(&mut lib, "A2".into(), "root".into(), "b.png".into(), true, &mut anchor);
    add_placement(&mut lib, "A3".into(), "root".into(), "c.png".into(), true, &mut anchor);
    let names: Vec<&str> =
      lib.index.placements_by_folder["root"].iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["a.png", "b.png", "c.png"], "文件夹视图批内保序");
    let items: Vec<&str> = lib.folders.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(items, ["a.png", "b.png", "c.png"], "全部视图批内保序");
    // 新批次（新锚点）：整批排在旧批次之前
    let mut anchor2 = HashMap::new();
    add_placement(&mut lib, "A4".into(), "root".into(), "z.png".into(), true, &mut anchor2);
    let names: Vec<&str> =
      lib.index.placements_by_folder["root"].iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["z.png", "a.png", "b.png", "c.png"], "新批次排最前");
  }

  #[test]
  fn add_placement_all_view_front_regardless_of_segment_position() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    // 批1 导入 FY，批2 导入 FX（全局前插把 FY 段挤到后面），批3 再导入 FY——
    // 旧实现按"FY 段首"定位把 y2 插到全部视图中间；修后任何批次都排视图最前
    add_placement(&mut lib, "Y1".into(), "FY".into(), "y1.png".into(), true, &mut HashMap::new());
    let mut anchor = HashMap::new();
    add_placement(&mut lib, "X1".into(), "FX".into(), "x1.png".into(), true, &mut anchor);
    add_placement(&mut lib, "X2".into(), "FX".into(), "x2.png".into(), true, &mut anchor);
    add_placement(&mut lib, "Y2".into(), "FY".into(), "y2.png".into(), true, &mut HashMap::new());
    let items: Vec<&str> = lib.folders.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(items, ["y2.png", "x1.png", "x2.png", "y1.png"], "全部视图：新批次排视图最前");
    let fy: Vec<&str> = lib.index.placements_by_folder["FY"].iter().map(|p| p.name.as_str()).collect();
    assert_eq!(fy, ["y2.png", "y1.png"], "文件夹视图：段内新批次在前（语义不变）");
    // 异文件夹混批：同一锚点跨文件夹也按到达序从头部依次后移
    let mut anchor3 = HashMap::new();
    add_placement(&mut lib, "M1".into(), "FX".into(), "m1.png".into(), true, &mut anchor3);
    add_placement(&mut lib, "M2".into(), "FY".into(), "m2.png".into(), true, &mut anchor3);
    let items: Vec<&str> = lib.folders.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(&items[..2], ["m1.png", "m2.png"], "混批按到达序全局保序");
  }

  #[test]
  fn import_event_json_shape_is_stable() {
    // 前端 TS 类型按此形状写，序列化契约变更会打爆这个用例
    let started = serde_json::to_value(ImportEvent::Started { total: 3 }).unwrap();
    assert_eq!(started, serde_json::json!({ "type": "started", "total": 3 }));
    let folders = serde_json::to_value(ImportEvent::Folders).unwrap();
    assert_eq!(folders, serde_json::json!({ "type": "folders" }));

    let view = PlacementView {
      asset_id: "AN-X".into(),
      folder_id: "root".into(),
      name: "a.png".into(),
      w: Some(1),
      h: None,
      thumb_w: None,
      thumb_h: None,
      ext: "png".into(),
      thumb_path: None,
      thumb_kind: None,
      origin: true,
      frame_count: None,
      animated: false,
      duration: None,
      bpm: None,
      imported_from: None,
      size: 0,
      created_at: None,
      modified_at: "2026-08-19T00:00:00+08:00".into(),
      imported_at: "2026-08-19T00:00:00+08:00".into(),
      note_count: None,
      thumb_frame: None,
      fps: None,
    };
    let asset = serde_json::to_value(ImportEvent::Asset { placement: Box::new(view) }).unwrap();
    assert_eq!(
      asset,
      serde_json::json!({
        "type": "asset",
        "placement": {
          "assetId": "AN-X", "folderId": "root", "name": "a.png",
          "w": 1, "h": null, "thumbW": null, "thumbH": null, "ext": "png", "thumbPath": null, "thumbKind": null, "origin": true,
          "frameCount": null, "animated": false, "duration": null, "bpm": null, "importedFrom": null,
          "size": 0, "createdAt": null,
          "modifiedAt": "2026-08-19T00:00:00+08:00", "importedAt": "2026-08-19T00:00:00+08:00",
          "noteCount": null, "thumbFrame": null, "fps": null
        }
      })
    );

    let finished = serde_json::to_value(ImportEvent::Finished {
      imported: 1,
      conflicts: vec![ConflictItem {
        source_path: PathBuf::from("p"),
        source_name: "新图.png".into(),
        source_size: 100,
        source_w: Some(10),
        source_h: None,
        source_thumb_path: None,
        target_folder: ROOT_FOLDER_ID.into(),
        target_folder_name: "根目录".into(),
        existing_asset_id: "AN-X".into(),
        existing_name: "旧图.png".into(),
        existing_w: None,
        existing_h: None,
        existing_size: 200,
        existing_ext: "png".into(),
        existing_thumb_path: None,
        existing_folder_name: "根目录".into(),
        sequence: None,
        imported_from: None,
      }],
      failed: vec![FailedItem { path: "q".into(), reason: "r".into() }],
    })
    .unwrap();
    assert_eq!(
      finished,
      serde_json::json!({
        "type": "finished", "imported": 1,
        "conflicts": [{
          "sourcePath": "p", "sourceName": "新图.png", "sourceSize": 100,
          "sourceW": 10, "sourceH": null, "sourceThumbPath": null,
          "targetFolder": "root", "targetFolderName": "根目录",
          "existingAssetId": "AN-X", "existingName": "旧图.png",
          "existingW": null, "existingH": null, "existingSize": 200,
          "existingExt": "png", "existingThumbPath": null, "existingFolderName": "根目录"
        }],
        "failed": [{ "path": "q", "reason": "r" }]
      })
    );
  }
}
