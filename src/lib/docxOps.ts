// docx 预览命令封装（R6 新功能验证首例，PRD 04-04 条 6，规范第 1 条）：read_docx_asset 统一出口。
// 类型以后端序列化结构为唯一来源（src-tauri/src/services/docx.rs；serde camelCase）。
import { invoke } from "@tauri-apps/api/core";

// 文本 run（粗 / 斜为 run 级标记）
export interface DocxRun {
  text: string;
  bold: boolean;
  italic: boolean;
}

// 段落（heading = 0 正文 / 1-6 标题级；list = 列表项，前端加项目符号）
export interface DocxParagraph {
  heading: number;
  list: boolean;
  runs: DocxRun[];
}

// 提取结果（truncated = 文本超 2MB 截断，UI 提示「仅显示前一部分」）
export interface DocxContent {
  paragraphs: DocxParagraph[];
  truncated: boolean;
}

// 读取 docx 预览内容（非 docx / 损坏 / 超大抛中文文案）
export async function readDocxAsset(assetId: string): Promise<DocxContent> {
  return await invoke<DocxContent>("read_docx_asset", { assetId });
}

// 按真实路径读 docx 预览内容（2026-09-26 工作空间文本卡 docx 支路；越界 / 非 docx 抛中文文案）
export async function readDocxByPath(absPath: string): Promise<DocxContent> {
  return await invoke<DocxContent>("read_docx_by_path", { absPath });
}
