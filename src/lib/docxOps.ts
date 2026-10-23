// docx 预览命令封装（规范第 1 条）：
// 预览链路（2026-10-10 mammoth 换引擎，PRD 04-04 条 6）= read_docx_binary 拿原始字节（原始二进制 IPC，
// 不经 JSON）→ docxToHtml 前端 mammoth.js 转 HTML（表格带框线 / 图片内嵌 data URI）；
// 工作空间文本卡内容 = read_docx_by_path 按路径段落提取（卡片不渲染 HTML，不吃 mammoth，链路不动）。
// 类型以后端序列化结构为唯一来源（src-tauri/src/services/docx.rs；serde camelCase）。
import { invoke } from "@tauri-apps/api/core";
import mammoth from "mammoth";

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

// 读取 docx 原始字节（预览 mammoth 链路；非 docx / 损坏 / 超 64MB 抛中文文案）
export async function readDocxBinary(assetId: string): Promise<ArrayBuffer> {
  const r = await invoke<ArrayBuffer | number[]>("read_docx_binary", { assetId });
  if (r instanceof ArrayBuffer) return r;
  return new Uint8Array(r).buffer; // 桩 / 非常规通道回退（原始 IPC 正常路径 = ArrayBuffer）
}

// 读取 docx 段落结构（库内文本卡正文懒加载——卡片要轻文本不吃 mammoth，2026-10-10 换引擎时实证存活）
export async function readDocxAsset(assetId: string): Promise<DocxContent> {
  return await invoke<DocxContent>("read_docx_asset", { assetId });
}

// docx 字节 → HTML（mammoth 默认图片转 data URI 内嵌；浮动布局图片 / 文本框降级顺序内嵌为已知口径）
export async function docxToHtml(buf: ArrayBuffer): Promise<string> {
  const r = await mammoth.convertToHtml({ arrayBuffer: buf });
  return r.value;
}

// 按真实路径读 docx 预览内容（2026-09-26 工作空间文本卡 docx 支路；越界 / 非 docx 抛中文文案）
export async function readDocxByPath(absPath: string): Promise<DocxContent> {
  return await invoke<DocxContent>("read_docx_by_path", { absPath });
}

// 按真实路径读 docx 原始字节（2026-10-23 工作空间文档预览放行：预览 mammoth 链路按路径版；
// 越界 / 非 docx / 损坏 / 超 64MB 抛中文文案）
export async function readDocxBinaryByPath(absPath: string): Promise<ArrayBuffer> {
  const r = await invoke<ArrayBuffer | number[]>("read_docx_binary_by_path", { absPath });
  if (r instanceof ArrayBuffer) return r;
  return new Uint8Array(r).buffer; // 桩 / 非常规通道回退（同 readDocxBinary）
}
