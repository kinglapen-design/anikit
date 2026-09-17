//! 数据模型：与 PRD 01_资源与数据 / 06_数据 的 JSON 结构一一对应。
//! 所有数据文件都必须携带 "version" 字段（PRD 06-05）；字段命名 camelCase，不造同义词。

use serde::{Deserialize, Serialize};

/// 当前软件支持的数据文件版本（PRD 06-05）：
/// 文件版本低于它则逐级迁移，高于它则拒绝写入（防止新版写过的库被旧版破坏）。
pub const DATA_VERSION: u32 = 1;

/// 根目录的保留文件夹 ID：Placement.folder 用它表示根，Folder 列表不放 root 节点
///（2026-08-13 自 services/import.rs 上移 core——core/library.rs 开库自检清 root 链接需要用，
/// core 不反向依赖 services；services/import.rs 以 pub use 转出口径不变）
pub const ROOT_FOLDER_ID: &str = "root";

/// 默认区块「目录」的保留 ID：不可删（可改名）；Folder.block = None 语义上即属「目录」根级，
/// 未分类（root Placement）同在「目录」区块根级——零数据迁移（2026-08-14 区块子系统）
pub const DEFAULT_BLOCK_ID: &str = "dir";

/// 档案形态标记（2026-08-15 档案子系统一轮，PRD 02_文件整理 03）：Folder.kind = Some(ARCHIVE_KIND)
/// 即档案文件夹（文件夹 + 详细数据，数据存库级 archives.json）；档案内不允许子级文件夹
pub const ARCHIVE_KIND: &str = "archive";

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
  /// 缩略图实际像素尺寸（5.4，2026-08-23 用户定）：卡片自适应/瀑布流布局比例跟缩略图走——
  /// 自定义缩略图后布局按新图比例自适应，width/height 恒为源文件尺寸不动；刷新缩略图从 origin
  /// 重生成即回退。生成缩略图时写入；存量数据 None（前端回退 width/height，比例一致无感知）。不升 DATA_VERSION
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thumb_width: Option<u32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thumb_height: Option<u32>,
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
  /// 导入渠道（"inbox" = 收件箱导入，含自动与手动；常规导入 None）——服务快速访问
  /// "已导入"区块口径与摘标语义（移动出根目录即摘标转正）；撞 hash 一律走正常重复提示，
  /// 同渠道静默合并 2026-08-13 用户定案废除（原服务 RECOVER 场景，随工程文件不自动导入消解）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub imported_from: Option<String>,
  /// 源文件磁盘创建时间（RFC3339，3.5 排序键）：导入时记源文件 birth time（池内复制会刷新
  /// 文件自身时间，故须在入库前读源）；剪贴板临时文件 / 新建文档 = 建档时刻；副本 / 克隆继承
  /// 源资产不刷新；存量数据 None（前端排序排尾）。不升 DATA_VERSION
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub created_at: Option<String>,
  /// 库内数据最后变更时刻（导入 / 解包 / 设封面 / 重命名等写操作刷新；3.5 排序"修改时间"取此——
  /// 2026-08-19 用户定：用库内变更时刻，非源文件 mtime 冻结值）
  pub modified_at: String,
  #[serde(default)]
  pub tags: Vec<String>,
  /// 描述（标题 + 内容组合，逐条添加，6.1，PRD 03_数据链接 02/06）；default 空数组，不升 DATA_VERSION
  #[serde(default)]
  pub descriptions: Vec<DescItem>,
  #[serde(default)]
  pub note: String,
  /// 逐帧笔记条数冗余（5.2 展示层：卡片角标 / 详情区块免读 notes.json）；
  /// set_note 同步维护，0 记 None 不写字段（None = 无笔记，比 Some(0) 干净）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub note_count: Option<u32>,
  /// 缩略图文件名（与 metadata 同目录）
  #[serde(default)]
  pub thumbnail: Option<String>,
  /// 主色（6.1 颜色解析，PRD 04_文件预览 05-1）：缩略图提取最多 10 个 #RRGGBB（按占比降序；
  /// alpha < 16 透明像素不计——2026-08-23 用户定：透明通道颜色不混入）。生成 / 重生成缩略图时
  /// 写入；存量打开详情时懒回填（services::color::ensure_asset_colors）。default 空数组，不升 DATA_VERSION
  #[serde(default)]
  pub colors: Vec<String>,
  // 序列帧（PRD 01-2.5）与视频（5.3 帧化改造：导入探测 / get_media_source 存量自愈写入）；
  // 其他普通资源无此三字段
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub fps: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub frame_count: Option<u32>,
  /// 封面帧文件名（frames/ 内的归一名 标题-序号.扩展名；2026-07-22 用户定案：默认首帧文件名）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thumb_frame: Option<String>,
  /// hidden 标记（PRD 02-15，8.1）：图片被设为工程/档案的缩略图后打标——所有普通视图统一过滤，
  /// 仅快速访问「缩略图」视图按资产显示一次；不复制、不搬迁、不删链接（链接位置原样保留只是不显示，
  /// 「还原为素材」取消标记即恢复）。default None 不写字段，不升 DATA_VERSION
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hidden: Option<bool>,
  /// 伴生缩略图引用（PRD 02-15 thumbAssetId，8.1）：工程资产（fla 等）指向其 hidden 缩略图资产——
  /// 卡片缩略图 / 布局尺寸经此引用取目标资产的图（PlacementView 组装时代理）；一个 hidden 资产
  /// 可被多个工程引用。开库自检剔除失效引用；「还原为素材」时同步清除所有指向它的引用
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thumb_asset_id: Option<String>,
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
  /// 区块表（2026-08-14 区块子系统）：侧栏顶级文件夹的分组；default 保证旧 folders.json（无此字段）可读，
  /// 开库自检（core/library.rs fix_blocks）负责注入默认区块「目录」
  #[serde(default)]
  pub blocks: Vec<Block>,
  #[serde(default)]
  pub folders: Vec<Folder>,
  #[serde(default)]
  pub items: Vec<Placement>,
}

impl FoldersFile {
  pub fn new() -> Self {
    Self {
      version: DATA_VERSION,
      // 新库自带默认区块（与开库自检同一口径，省一次自愈落盘）
      blocks: vec![Block { id: DEFAULT_BLOCK_ID.to_string(), name: "目录".to_string(), icon: None, order: 0 }],
      folders: Vec::new(),
      items: Vec::new(),
    }
  }
}

/// 区块（2026-08-14 区块子系统）：侧栏顶级文件夹的分组；order 为显示顺序（排序时连续重写 0..n）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
  pub id: String,
  pub name: String,
  /// 默认图标（自定义图标名）：新建顶级文件夹继承到 Folder.icon；None = 无默认图标，不写字段
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
  #[serde(default)]
  pub order: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub parent: Option<String>,
  /// 所属区块 ID（仅顶级文件夹有意义；None / "dir" = 默认区块「目录」，2026-08-14 区块子系统）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub block: Option<String>,
  /// 自定义图标（新建顶级文件夹时继承所在区块默认图标；None = 默认文件夹图标，不写字段）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
  /// 节点形态（2026-08-15 档案子系统一轮）：Some("archive") = 档案文件夹；None = 普通文件夹，不写字段
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub kind: Option<String>,
  /// 自定义颜色（2026-08-18，3.3）：色档名（red/orange/...，对应前端 token --custom-{色}）；
  /// 仅本节点持有——子级视觉跟随 = 该色底色版本（--custom-{色}-bg），不写子级数据（用户定）；
  /// None = 未自定义，不写字段
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub color: Option<String>,
  /// 分组规则（2026-08-19，3.5 二轮，PRD 02-04）：按文件夹各自生效的内容区分组显示；
  /// 数组序 = 定义顺序（内容区组间序）；None / 空 = 不分组（不写字段，不升 DATA_VERSION）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub groups: Option<Vec<GroupDef>>,
  #[serde(default)]
  pub order: i32,
}

/// 分组定义（PRD 02-04）：名称 + 规则集；组内多规则 AND 全命中才算归入；
/// **rules 为空 = 自由组（2026-09-17，PRD 02-04 无规则空组）**：完全无规则、手动归组，
/// members 为 assetId 有序列表（移动语义唯一归属；与规则组不混态）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDef {
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub rules: Vec<GroupRule>,
  /// 自由组成员（assetIds；None / 缺省 = 无成员不写字段，不升 DATA_VERSION）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub members: Option<Vec<String>>,
}

/// 分组规则：kind = "ext"（扩展名多选）/ "type"（类型多选）/ "name"（分词列表，词间 AND）/
/// "importDate" / "modifyDate"（日期分档，多档 = OR）/ "tags"（标签词列表，词间 AND，2026-09-17）
/// ——均存值集合；类型清单前端口径（图片 / 动图 / 视频 / 音频 / 文本 / 文档 / 工程 / 其他，2026-08-19 用户定 8 类）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRule {
  pub kind: String,
  #[serde(default)]
  pub values: Vec<String>,
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

// ---------- archives.json（档案数据，2026-08-15 档案子系统一轮，PRD 03_数据链接 06） ----------
// 档案节点在 folders.json 标记形态（Folder.kind = "archive"）；头像 / 徽标 / 属性 / 描述存本文件（单一写点）

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchivesFile {
  pub version: u32,
  #[serde(default)]
  pub archives: Vec<ArchiveData>,
}

impl ArchivesFile {
  pub fn new() -> Self {
    Self { version: DATA_VERSION, archives: Vec::new() }
  }
}

/// 一个档案的详细数据（folder_id 对应 folders.json 中 kind = "archive" 的 Folder）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveData {
  pub folder_id: String,
  /// 徽标（档案卡片左上角；None = 不设置，不写字段）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub badge: Option<Badge>,
  /// 头像（即档案卡片封面，二者同步；None = 默认索引子项第一张图，不写字段）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub avatar: Option<Avatar>,
  /// 属性（短文本并列，"打词"能力，PRD 03_数据链接 02）
  #[serde(default)]
  pub props: Vec<String>,
  /// 描述（标题 + 内容组合，逐条添加）
  #[serde(default)]
  pub descriptions: Vec<DescItem>,
  /// 链接（详情 pinned 链接输入框数据源，2026-08-15 任务 A 三项；default 空数组）
  #[serde(default)]
  pub links: Vec<String>,
  /// 创建时间（RFC3339 本地时间，同 AssetMeta 口径；create_archive / convert_to_archive 写入；
  /// 旧数据缺省 = 空串不写字段，基础信息"创建日期"行不显示）
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub created_at: String,
}

impl ArchiveData {
  /// 默认空记录（get_archive_data 无记录时返回）
  pub fn new(folder_id: &str) -> Self {
    Self {
      folder_id: folder_id.to_string(),
      badge: None,
      avatar: None,
      props: Vec::new(),
      descriptions: Vec::new(),
      links: Vec::new(),
      created_at: String::new(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Badge {
  pub text: String,
  pub color: String,
}

/// 头像（即档案卡片封面，二者同步）：covers/ 目录内独立存的烘培封面文件 + 取景框参数
///（2026-08-23 用户定：本地选图独立存**不入库**为资产，偏离原「源资产 ID」口径——
/// 资产引用变体随主窗口选择模式落地时再扩；None = 默认索引子项第一张图，不写字段）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avatar {
  /// 封面文件名（库根 covers/ 内；带时间戳生成不覆盖写——undo 回滚 archives.json 即恢复旧封面显示，
  /// 旧文件留盘待清理，见 09 待办）
  pub file: String,
  /// 取景框参数（封面设置面板：滑块缩放 + 拖动位移；None = 未调整，不写字段）
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub crop: Option<Crop>,
}

/// 取景框参数：图片在框内的偏移与缩放
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Crop {
  pub x: f64,
  pub y: f64,
  pub scale: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescItem {
  pub title: String,
  pub content: String,
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

// ---------- relations.json（关系 / 音色，空间级，6.2 选择模式一轮，PRD 03_数据链接 06） ----------
// 关系可跨库（库 A 的角色配库 B 的音频）故存空间根而非库内；ref 携库名 + 显示名快照
//（跨库目标未打开 / 目标失效时兜底显示）。单一写点 = services/relations.rs。

/// 关系 kind：free = 自由关系（6.2 一轮）；voice = 音色（6.2 链接音频轮，PRD 03 数据链接 02；
/// 恒单向 使用者→音频，from 不得为音频 asset——2026-08-24 用户定方案 1，防"音频的音频"递归）
pub const RELATION_KIND_FREE: &str = "free";
pub const RELATION_KIND_VOICE: &str = "voice";

/// 生成关系 ID：`RL` + 时间戳（yyMMddHHmm）+ 5 位随机字符（同 new_asset_id 口径，ID 不承载语义）
pub fn new_relation_id() -> String {
  const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  use rand::Rng;
  let mut rng = rand::thread_rng();
  let suffix: String = (0..5).map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char).collect();
  format!("RL{}-{suffix}", chrono::Local::now().format("%y%m%d%H%M"))
}

/// 关系端点引用：lib = 库名；kind = "asset"（id = 资产 ID）/ "archive"（id = 档案 folderId）；
/// name = 显示名快照（写入时刻的资产名 / 档案名——跨库目标未打开或目标失效时兜底显示）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationRef {
  pub lib: String,
  pub kind: String,
  pub id: String,
  pub name: String,
}

/// 一条关系（双向显示 = 同一条记录的两个视角，PRD 03_数据链接 02）：
/// label = **to 对 from** 的称谓（雪牧.from 发起选雪天.to 输入"父亲" → 雪牧详情显示"雪天是父亲"）；
/// reverse_label = **from 对 to** 的称谓（"儿子"——选择模式备选区右框手动填写，2026-08-23 用户定：
/// 不做词典自动推导）；None / 空串 = 未填写（反向显示回退 label），不写字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
  pub id: String,
  pub from: RelationRef,
  pub to: RelationRef,
  pub kind: String,
  pub label: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reverse_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationsFile {
  pub version: u32,
  #[serde(default)]
  pub relations: Vec<Relation>,
}

impl RelationsFile {
  pub fn new() -> Self {
    Self { version: DATA_VERSION, relations: Vec::new() }
  }
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
    // 旧版 folders.json（无 blocks / Folder 无 block、icon 字段）serde default 读入（2026-08-14 区块子系统）
    assert!(file.blocks.is_empty());
    assert_eq!((file.folders[0].block.clone(), file.folders[0].icon.clone()), (None, None));
    // 链接 Placement 序列化不出 origin 字段，保持文件干净
    let text = serde_json::to_string(&file.items[1]).unwrap();
    assert!(!text.contains("origin"));
    // 旧版 Folder（无 kind 字段）serde default 读入；None 不写字段（2026-08-15 档案子系统）
    assert_eq!(file.folders[0].kind, None);
    assert!(!serde_json::to_string(&file.folders[0]).unwrap().contains("kind"));
    // 旧版 Folder（无 color 字段）serde default 读入；None 不写字段（2026-08-18，3.3）
    assert_eq!(file.folders[0].color, None);
    assert!(!serde_json::to_string(&file.folders[0]).unwrap().contains("color"));
  }

  /// 档案结构（2026-08-15 档案子系统一轮）：camelCase 序列化（踩坑 031 防线）、
  /// badge / avatar None 不写字段、缺省字段 default 读入、往返不丢
  #[test]
  fn archives_file_camel_case_and_defaults() {
    let file = ArchivesFile {
      version: 1,
      archives: vec![
        ArchiveData {
          folder_id: "F01".into(),
          badge: Some(Badge { text: "主角".into(), color: "#FF0000".into() }),
          avatar: Some(Avatar { file: "F01-20260823120000.webp".into(), crop: Some(Crop { x: 1.5, y: -2.0, scale: 2.0 }) }),
          props: vec!["走路".into()],
          descriptions: vec![DescItem { title: "设定".into(), content: "长文".into() }],
          links: vec!["https://ref".into()],
          created_at: "2026-08-15T10:00:00+08:00".into(),
        },
        ArchiveData::new("F02"),
      ],
    };
    let text = serde_json::to_string(&file).unwrap();
    assert!(text.contains("folderId"), "rename_all camelCase 生效：{text}");
    assert!(text.contains("file"), "Avatar 同口径：{text}");
    // 空记录：badge / avatar / createdAt 不写字段，props / descriptions / links 空数组照常写
    let bare = serde_json::to_string(&file.archives[1]).unwrap();
    assert_eq!(bare, r#"{"folderId":"F02","props":[],"descriptions":[],"links":[]}"#);
    // 缺省字段 default 读入（一轮旧数据 / 手编最小记录）
    let min: ArchiveData = serde_json::from_str(r#"{ "folderId": "F03" }"#).unwrap();
    assert_eq!(min, ArchiveData::new("F03"));
    // 往返不丢字段
    let back: ArchivesFile = serde_json::from_str(&text).unwrap();
    assert_eq!(file, back);
    // 档案 Folder：kind 写字段
    let folder: Folder =
      serde_json::from_str(r#"{ "id": "F01", "name": "雪牧", "parent": null, "kind": "archive", "order": 0 }"#).unwrap();
    assert_eq!(folder.kind.as_deref(), Some(ARCHIVE_KIND));
    assert!(serde_json::to_string(&folder).unwrap().contains(r#""kind":"archive""#));
  }
}
