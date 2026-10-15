// MarkDown 大纲标题提取（2026-10-15 大纲轮，PRD 04_文件预览「md 大纲栏」）：
// 复用 markdown-it parse 产物——heading_open token 自带层级（tag h1-h6）与源码行号
//（map[0]，0 起，同 kitLineMap 给渲染块挂的 data-line-start 口径，跳转定位直接对齐）；
// 文本 = inline 子流纯文本拼接（text / code_inline 内容），行内格式标记（** ` []() 等）不进大纲。
// PreviewText 渲染与大纲共用同一次 parse（tokens 双消费），不另起解析。
import type { Token } from "markdown-it";

export interface MdOutlineItem {
  level: number; // 1-6（heading_open 的 tag）
  text: string; // 纯文本（行内格式剥标记）
  line: number; // 源码起始行（0 起）
}

export function extractMdOutline(tokens: Token[]): MdOutlineItem[] {
  const out: MdOutlineItem[] = [];
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i];
    if (t.type !== "heading_open") continue;
    const inline = tokens[i + 1];
    let text = "";
    if (inline?.type === "inline") {
      for (const c of inline.children ?? []) {
        if (c.type === "text" || c.type === "code_inline") text += c.content;
      }
    }
    out.push({ level: Number(t.tag.slice(1)) || 1, text: text.trim(), line: t.map?.[0] ?? 0 });
  }
  return out;
}
