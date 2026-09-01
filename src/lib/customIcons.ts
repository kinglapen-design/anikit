// 自定义图标库清单（2026-08-14 区块子系统：区块管理弹窗"默认图标"下拉数据源）：
// 文件 = assets/icons/02自定义/*.svg（双色集，#BFBFBF 亮层 / #666666 暗层原值），引用值 = 文件干
//（lib/icons.ts glob 自动注册，AppIcon/getIcon 直接按名引用）；中文标签出自 assets/icons/命名对照.md。
// 3.3 更改图标网格面板（05 稿 §3/§4）复用本清单。
// 2026-09-01（7.1 二十五轮）：30 个新双色图标接入，旧 19 名清单废除（folder-closed/folder-open
// 为基础件不入本清单）；顺序 = 命名对照.md 02自定义 表序。
// 2026-09-01（同日走查）：archive 移出清单（29 个）——档案图标档案专属不可选（TreeRowItem
// 按 kind === "archive" 强制挂载，勿作文件夹自定义值；存量 Folder/Block.icon = "archive"
// 由后端 fix_custom_icons 白名单同批剔除回退 None）。
// 存量 Folder.icon / Block.icon 旧文件干迁移已落地 = 后端开库自检 fix_custom_icons
//（core/library.rs：就近映射 animal→dog / audio→music / video→film / vehicle→car / monster→slime，
// 对不上的旧名与任意未知名回退 None——2026-09-01 用户定；其新名白名单与本清单同源）。
export interface CustomIconDef {
  name: string; // 文件干（Folder.icon / Block.icon 存储值）
  label: string; // 中文名（下拉列表显示）
}

export const CUSTOM_ICONS: CustomIconDef[] = [
  { name: "inbox", label: "收件箱" },
  { name: "window", label: "窗口" },
  { name: "briefcase", label: "公文包" },
  { name: "medal", label: "勋章" },
  { name: "card", label: "卡片" },
  { name: "slime", label: "史莱姆" },
  { name: "layers", label: "图层" },
  { name: "image", label: "图片" },
  { name: "chat", label: "对话" },
  { name: "layout", label: "布局" },
  { name: "film", label: "影片" },
  { name: "house", label: "房屋" },
  { name: "tag", label: "标签" },
  { name: "building", label: "楼房" },
  { name: "car", label: "汽车" },
  { name: "browser", label: "浏览器" },
  { name: "flame", label: "火焰" },
  { name: "thumbs-up", label: "点赞" },
  { name: "dog", label: "狗狗" },
  { name: "camera", label: "相机" },
  { name: "kanban", label: "看板" },
  { name: "vegetable", label: "蔬菜" },
  { name: "emoji", label: "表情" },
  { name: "character", label: "角色" },
  { name: "palette", label: "调色盘" },
  { name: "shopping-basket", label: "购物篮" },
  { name: "music", label: "音乐" },
  { name: "plane", label: "飞机" },
  { name: "tableware", label: "餐具" },
];
