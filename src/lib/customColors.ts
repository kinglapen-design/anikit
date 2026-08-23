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
