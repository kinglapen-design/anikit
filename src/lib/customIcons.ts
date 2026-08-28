// 自定义图标库清单（2026-08-14 区块子系统：区块管理弹窗"默认图标"下拉数据源）：
// 文件 = assets/icons/05_custom-icon/*.svg，引用值 = 文件干（lib/icons.ts glob 自动注册，
// AppIcon/getIcon 直接按名引用）；中文标签出自 assets/icons/命名对照.md。
// 3.3 更改图标网格面板（05 稿 §3/§4）复用本清单。
// 2026-08-28：05_custom-icon 已随旧图标归档 assets/icons/旧版/ 并退出 glob，
// 本清单文件干暂不可解析（报"图标不存在"）；阶段七按新 02自定义 集重写 + 存量存储值迁移。
export interface CustomIconDef {
  name: string; // 文件干（Folder.icon / Block.icon 存储值）
  label: string; // 中文名（下拉列表显示）
}

export const CUSTOM_ICONS: CustomIconDef[] = [
  { name: "animal", label: "动物" },
  { name: "audio", label: "音频" },
  { name: "building", label: "建筑" },
  { name: "character", label: "角色" },
  { name: "clothing", label: "服饰" },
  { name: "digital", label: "数码" },
  { name: "emoji", label: "表情" },
  { name: "facility", label: "设施" },
  { name: "food", label: "食物" },
  { name: "furniture", label: "家具" },
  { name: "game", label: "游戏" },
  { name: "monster", label: "怪物" },
  { name: "project", label: "项目" },
  { name: "scene", label: "场景" },
  { name: "text", label: "文本" },
  { name: "tool", label: "工具" },
  { name: "vehicle", label: "载具" },
  { name: "video", label: "视频" },
  { name: "weapon", label: "武器" },
];
