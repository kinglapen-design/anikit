// 文本资产命令封装（5.6，规范第 1 条）：read_text_asset / write_text_asset 统一出口。
// 类型以后端序列化结构为唯一来源（src-tauri/src/commands/text.rs；serde camelCase）。
import { invoke } from "@tauri-apps/api/core";

// read_text_asset 返回（txt/md 全文；UTF-8 优先 GBK 回退，2MB 上限）
export interface TextContent {
  content: string;
}

// write_text_asset 返回（保存后同步 PlacementView：文件大小 + 库内变更时刻）
export interface WriteTextResult {
  size: number;
  modifiedAt: string;
}

// 读取文本内容（非文本 / 超上限抛中文文案）
export async function readTextAsset(assetId: string): Promise<TextContent> {
  return await invoke<TextContent>("read_text_asset", { assetId });
}

// 保存文本内容（APP 内编辑防抖自动落盘；编辑不进 undo）
export async function writeTextAsset(assetId: string, content: string): Promise<WriteTextResult> {
  return await invoke<WriteTextResult>("write_text_asset", { assetId, content });
}
