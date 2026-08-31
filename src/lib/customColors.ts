// 自定义颜色清单（2026-08-18，3.3 / 05 稿 §4 更改颜色网格面板 + tokens.css 自定义颜色 9 组）：
// tint = token 名（--custom-{tint}，Folder.color 存储值）；子级视觉跟随 = --custom-{tint}-bg
//（底色版本，用户定——不写子级数据）。顺序按稿（§4 一行 红/橙/黄绿/绿、二行 蓝紫/紫/粉/桃红/蓝，
// 蓝色在末位——与 tokens.css 声明序不同，勿按声明序重排）。
export interface CustomColorDef {
  tint: string; // --custom-{tint}（Folder.color 存储值）
  label: string; // 中文名（token 1.6 节命名）
  hex: string; // 主档色值（tokens.css --custom-{tint}；6.1 档案徽标底色 = 本档直存 hex，Badge.color CSS 直用）
}

export const CUSTOM_COLORS: CustomColorDef[] = [
  { tint: "red", label: "红色", hex: "#402122" },
  { tint: "orange", label: "橙色", hex: "#352719" },
  { tint: "yellow-green", label: "黄绿", hex: "#2c2b16" },
  { tint: "green", label: "绿色", hex: "#1c2e20" },
  { tint: "blue-violet", label: "蓝紫色", hex: "#252841" },
  { tint: "purple", label: "紫色", hex: "#2f263d" },
  { tint: "pink", label: "粉色", hex: "#392238" },
  { tint: "peach", label: "桃红", hex: "#3c222a" },
  { tint: "blue", label: "蓝色", hex: "#1a2c37" },
];

// 旧九色名 → 阶段七新九色名（2026-08-30 左栏轮过渡映射）：存量 Folder.color 为旧体系
// 暗底色名（--custom-{tint}），新树容器（TreeRowItem tint / 缩进线 tint）吃新名
//（--n-custom-{tint}-1/-2 亮暗对，TwoToneIcon data-tint 集合）。
// 新体系无 orange / blue-violet：就近归 yellow / purple。数据层迁移（连同 02自定义 图标
// 文件干映射，命名对照.md 02 节注）归独立待办，本表常驻至迁移完成。
const LEGACY_TINT_MAP: Record<string, string> = {
  red: "red",
  orange: "yellow",
  "yellow-green": "yellowgreen",
  green: "green",
  "blue-violet": "purple",
  purple: "purple",
  pink: "pink",
  peach: "peach",
  blue: "blue",
};

/** 旧体系自定义色名映射为新九色名；未知名（含空）原样返回（TwoToneIcon 无匹配 data-tint 时回默认双色） */
export function toNewTint(tint: string | undefined): string {
  if (!tint) return "";
  return LEGACY_TINT_MAP[tint] ?? tint;
}

// 文件夹色块行十色（2026-09-01 新稿「右键菜单-更改颜色」，用户定案）：常态-1 = 默认位（清色 null）
// + 九色 -1 档按 Token.md 1.6 声明序；显示色 = 新 token -1 档，存储值 = 新 tint 名
//（旧消费端 TreeItem / LinkManageDialog 经 toNewTint 转正，数据层迁移待办完成后 LEGACY_TINT_MAP 退役）。
export const FOLDER_SWATCHES: { tint: string | null; hex: string; label: string }[] = [
  { tint: null, hex: "var(--n-custom-1)", label: "默认" },
  { tint: "red", hex: "var(--n-custom-red-1)", label: "红色" },
  { tint: "blue", hex: "var(--n-custom-blue-1)", label: "蓝色" },
  { tint: "yellow", hex: "var(--n-custom-yellow-1)", label: "黄色" },
  { tint: "yellowgreen", hex: "var(--n-custom-yellowgreen-1)", label: "黄绿" },
  { tint: "green", hex: "var(--n-custom-green-1)", label: "绿色" },
  { tint: "cyan", hex: "var(--n-custom-cyan-1)", label: "青色" },
  { tint: "purple", hex: "var(--n-custom-purple-1)", label: "紫色" },
  { tint: "pink", hex: "var(--n-custom-pink-1)", label: "粉色" },
  { tint: "peach", hex: "var(--n-custom-peach-1)", label: "桃红" },
];
