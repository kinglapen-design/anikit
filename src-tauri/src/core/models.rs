//! 数据模型：与 PRD 01_资源与数据 / 06_数据 的 JSON 结构一一对应。
//! 所有数据文件都必须携带 "version" 字段（PRD 06-05）；字段命名 camelCase，不造同义词。

use serde::{Deserialize, Serialize};

/// 当前软件支持的数据文件版本（PRD 06-05）：
/// 文件版本低于它则逐级迁移，高于它则拒绝写入（防止新版写过的库被旧版破坏）。
pub const DATA_VERSION: u32 = 1;

/// 生成资源 ID：`AN` + 时间戳（yyMMddHHmm，时间有序）+ 5 位随机字符（防同秒冲突）。
/// ID 不承载语义，语义全部在 metadata（PRD 01-2.1）。
pub fn new_asset_id() -> String {
  // 去掉易混淆字符 0/O/1/I，肉眼抄录不出错
  const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  use rand::Rng;
  let mut rng = rand::thread_rng();
  let suffix: String = (0..5).map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char).collect();
  format!("AN{}-{suffix}", chrono::Local::now().format("%y%m%d%H%M"))
}

fn is_false(v: &bool) -> bool {
  !*v
}

// ---------- metadata.json（assets/{ID}.info/，单资源元数据） ----------

/// 只存资源固有属性；放置位置（Placement）在 folders.json，不冗余（PRD 01-2.3）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMeta {
  pub version: u32,
  pub id: String,
  pub name: String,
  /// 磁盘原始文件名（含拓展名）；真实路径不存绝对路径，由 ID 推导：assets/{ID}.info/{originName}
  pub origin_name: String,
  pub ext: String,
  pub size: u64,
  /// 导入查重用（sha256:xxxx）
  #[serde(default)]
  pub hash: String,
  /// 图片/视频/音频导入时提取，fla 等工程文件为 null（PRD 示例显式保留 null）
  #[serde(default)]
  pub width: Option<u32>,
  #[serde(default)]
  pub height: Option<u32>,
  #[serde(default)]
  pub duration: Option<f64>,
  /// 音频 BPM（只读文件内嵌标签 TBPM/bpm，无标签为 null——不做自动节拍检测，2026-07-26 用户定）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bpm: Option<f64>,
  /// 0-5 星评分
  #[serde(default)]
  pub rating: u8,
  #[serde(default)]
  pub links: Vec<String>,
  pub imported_at: String,
  /// 导入渠道（"inbox" = 收件箱导入，含自动与手动；常规导入 None）——自动导入撞 hash 自动合并的
  /// 判定依据（PRD 01-2.3 注）；手动导入虽同标 inbox 渠道，但 manual=true 关闭合并、走正常重复提示
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub imported_from: Option<String>,
  pub modified_at: String,
  #[serde(default)]
  pub tags: Vec<String>,
  #[serde(default)]
  pub note: String,
  /// 逐帧笔记条数冗余（5.2 展示层：卡片角标 / 详情区块免读 notes.json）；
  /// set_note 同步维护，0 记 None 不写字段（None = 无笔记，比 Some(0) 干净）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub note_count: Option<u32>,
  /// 缩略图文件名（与 metadata 同目录）
  #[serde(default)]
  pub thumbnail: Option<String>,
  // 序列帧（PRD 01-2.5）与视频（5.3 帧化改造：导入探测 / get_media_source 存量自愈写入）；
  // 其他普通资源无此三字段
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub fps: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub frame_count: Option<u32>,
  /// 封面帧文件名（frames/ 内的归一名 标题-序号.扩展名；2026-07-22 用户定案：默认首帧文件名）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thumb_frame: Option<String>,
  // 视频虚拟片段（PRD 01-2.6）：type 为 "segment" 时 source/start/end 有效
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub r#type: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub start: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub end: Option<f64>,
}

// ---------- space.json（储存空间标志与元数据，PRD 01_储存空间-02） ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceFile {
  pub version: u32,
  pub name: String,
  /// 创建时间（RFC3339 本地时区，与 config 的 lastOpenedAt 同格式）
  pub created_at: String,
  /// 自动导入开关（空间级设置，2026-08-12 用户定：inbox 空间级共用）；
  /// default 保证旧 space.json（无此字段）可读
  #[serde(default)]
  pub auto_import: bool,
  /// 资源库自定义顺序（菜单抓手拖动排序持久化，PRD 01_储存空间-3.2：新库排尾）；
  /// default 保证旧 space.json（无此字段）可读
  #[serde(default)]
  pub library_order: Vec<String>,
}

impl SpaceFile {
  pub fn new(name: &str, auto_import: bool) -> Self {
    Self {
      version: DATA_VERSION,
      name: name.to_string(),
      created_at: chrono::Local::now().to_rfc3339(),
      auto_import,
      library_order: Vec::new(),
    }
  }
}

// ---------- folders.json（文件夹树 + 全部 Placement） ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldersFile {
  pub version: u32,
  #[serde(default)]
  pub folders: Vec<Folder>,
  #[serde(default)]
  pub items: Vec<Placement>,
}

impl FoldersFile {
  pub fn new() -> Self {
    Self { version: DATA_VERSION, folders: Vec::new(), items: Vec::new() }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub parent: Option<String>,
  #[serde(default)]
  pub order: i32,
}

/// Placement：哪个资源、在哪个文件夹、显示什么名称、排在第几（PRD 01-2.4）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
  pub asset: String,
  pub folder: String,
  pub name: String,
  #[serde(default)]
  pub order: i32,
  /// 导入时创建的第一条 Placement 为原始（PRD 01-3.6）；链接 Placement 无此字段
  #[serde(default, skip_serializing_if = "is_false")]
  pub origin: bool,
}

// ---------- tags.json（标签定义与群组，结构随阶段 4 细化） ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagsFile {
  pub version: u32,
  #[serde(default)]
  pub tags: Vec<Tag>,
  #[serde(default)]
  pub groups: Vec<TagGroup>,
}

impl TagsFile {
  pub fn new() -> Self {
    Self { version: DATA_VERSION, tags: Vec::new(), groups: Vec::new() }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagGroup {
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub order: i32,
}

// ---------- pins.json（快捷入口，ID 引用） ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinsFile {
  pub version: u32,
  #[serde(default)]
  pub pins: Vec<Pin>,
}

impl PinsFile {
  pub fn new() -> Self {
    Self { version: DATA_VERSION, pins: Vec::new() }
  }
}

/// target 为 ID：文件夹（F 开头）或资源（AN 开头），按 ID 寻址避免改名死链（PRD 01-3.4）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
  pub target: String,
  #[serde(default)]
  pub order: i32,
}

// ---------- recycle.json（回收站记录） ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecycleFile {
  pub version: u32,
  #[serde(default)]
  pub items: Vec<RecycleItem>,
}

impl RecycleFile {
  pub fn new() -> Self {
    Self { version: DATA_VERSION, items: Vec::new() }
  }
}

/// 移入回收站不做物理移动（PRD 06-06）：记录删除前全部 Placement，还原时按记录恢复
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecycleItem {
  pub asset: String,
  #[serde(default)]
  pub placements: Vec<Placement>,
  pub deleted_at: String,
}

// ---------- recent.json（最近使用，上限 500 条） ----------

/// LRU 上限：超出淘汰最旧（6.1 第二轮启用）
pub const RECENT_LIMIT: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentFile {
  pub version: u32,
  #[serde(default)]
  pub items: Vec<RecentItem>,
}

impl RecentFile {
  pub fn new() -> Self {
    Self { version: DATA_VERSION, items: Vec::new() }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentItem {
  pub asset: String,
  /// 最近一次"使用"时间（打开成功等入口写入，RFC3339 本地秒）
  pub viewed_at: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn asset_id_format() {
    let id = new_asset_id();
    assert!(id.starts_with("AN"));
    assert_eq!(id.len(), 2 + 10 + 1 + 5); // AN + 时间戳 + - + 5 位随机字符
    let suffix = &id[13..];
    assert!(suffix.chars().all(|c| c.is_ascii_alphanumeric() && !matches!(c, '0' | 'O' | '1' | 'I')));
  }

  #[test]
  fn parses_prd_metadata_example() {
    // PRD 01-2.3 的 metadata.json 示例（version 字段按 PRD 06-05 要求携带）
    let json = r#"{
      "version": 1,
      "id": "AN2607171718-X7K2Q",
      "name": "人物",
      "originName": "人物.fla",
      "ext": "fla",
      "size": 1234567,
      "hash": "sha256:xxxx",
      "width": null,
      "height": null,
      "duration": null,
      "rating": 0,
      "links": [],
      "importedAt": "2026-07-17T17:18:00+08:00",
      "modifiedAt": "2026-07-17T17:18:00+08:00",
      "tags": ["男人", "走路"],
      "note": "注释与描述",
      "thumbnail": "人物_thumbnail.webp"
    }"#;
    let meta: AssetMeta = serde_json::from_str(json).unwrap();
    assert_eq!(meta.id, "AN2607171718-X7K2Q");
    assert_eq!(meta.origin_name, "人物.fla");
    assert_eq!(meta.width, None);
    assert_eq!(meta.tags, vec!["男人", "走路"]);
    // 往返序列化不丢字段
    let text = serde_json::to_string(&meta).unwrap();
    let back: AssetMeta = serde_json::from_str(&text).unwrap();
    assert_eq!(meta, back);
  }

  #[test]
  fn parses_prd_folders_example() {
    // PRD 01-2.4 的 folders.json 示例必须原样可解析
    let json = r#"{
      "version": 1,
      "folders": [
        { "id": "F01", "name": "人物",   "parent": null, "order": 0 },
        { "id": "F02", "name": "动画01", "parent": null, "order": 1 }
      ],
      "items": [
        { "asset": "AN2607171718-X7K2Q", "folder": "F01", "name": "男人.fla", "order": 0, "origin": true },
        { "asset": "AN2607171718-X7K2Q", "folder": "F02", "name": "张三.fla", "order": 0 }
      ]
    }"#;
    let file: FoldersFile = serde_json::from_str(json).unwrap();
    assert_eq!(file.folders.len(), 2);
    assert!(file.items[0].origin);
    assert!(!file.items[1].origin);
    // 链接 Placement 序列化不出 origin 字段，保持文件干净
    let text = serde_json::to_string(&file.items[1]).unwrap();
    assert!(!text.contains("origin"));
  }
}
