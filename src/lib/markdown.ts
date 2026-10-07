// MarkDown 渲染实例（PRD 04-04 条 6 定案 markdown-it；排版 = styles/markdown.css 全局类）：
// 任务列表（GFM `- [ ]` / `- [x]`）＝ 我们自己的复选框（2026-09 用户定）——core rule 剥 `[ ]` 前缀、
// li 挂 .md-task、文本前注入 .kit-check-box 静态框（styles/checkbox.css 通用类，Checkbox.vue 同款）；
// 框带 data-task-line（源码行号，0 起），点击 toggle 回写由 PreviewText 事件代理接（lib/mdEdit.toggleTaskLine）。
// 代码块 = div.md-code 包装 + pre.kit-scroll-x 横滚 + 右上角 .md-copy 复制钮（文本编辑稿，PromptSection 同款口径）。
// 图片 = anikit 引用渲染真图（2026-10-08 S3，PRD 02-09：anikit://item/{id} 与 http://127.0.0.1:*/item/{id}
// 双形态归一 → convertFileSrc 缩略图，点击 = 内部跳转由 PreviewText 代理接 data-anikit-id）；
// 非 anikit 引用 / 无缩略图 = 占位文本（旧口径）。链接可点（同轮：anikit:// 内部跳转，http(s) 默认浏览器——
// validateLink 白名单放行 anikit 协议，markdown-it 默认只认 http/ftp/mailto 会掐死自定义协议 href）。
import MarkdownIt, { type MarkdownIt as MdIt, type StateCore, type Token } from "markdown-it";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { parseAnikitRef } from "./anikitLink";
import { getIcon } from "./icons";

const CHECK_SVG = getIcon("check-tiny").replace("<svg", '<svg class="kit-check-mark"');
const COPY_SVG = getIcon("copy");

// 代码块包装：div.md-code + 复制钮（v-html 语境无 v-tip，钮义自明不加 title）；换行不横滚（markdown.css pre-wrap）
// line = 源码起始行（0 起，分屏 typewriter 跟滚定位用，同 kitLineMap 口径）
function wrapCode(inner: string, line: number): string {
  const dataLine = line >= 0 ? ` data-line-start="${line}"` : "";
  return `<div class="md-code"${dataLine}>${inner}<button type="button" class="md-copy">${COPY_SVG}</button></div>`;
}

// 块级元素源码行映射（分屏 typewriter 跟滚定位）：nesting=1 的块级 open token 一律挂 data-line-start
function kitLineMap(state: StateCore): boolean {
  for (const t of state.tokens) {
    if (t.map && t.nesting === 1) t.attrSet("data-line-start", String(t.map[0]));
  }
  return false;
}

// 任务列表 core rule：list_item 首段 inline 以 `[ ] ` / `[x] ` 开头 → 剥前缀 + li.md-task + 注入复选框
function kitTaskList(state: StateCore): boolean {
  const tokens = state.tokens;
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i];
    if (t.type !== "list_item_open") continue;
    // 本 item 首段 inline（结构：list_item_open → paragraph_open → inline；空 item 无 inline 防爆）
    let inline: Token | null = null;
    for (let j = i + 1; j < tokens.length; j++) {
      const x = tokens[j];
      if (x.type === "inline") {
        inline = x;
        break;
      }
      if (x.type === "list_item_close" || x.type === "list_item_open") break;
    }
    const first = inline?.children?.[0];
    if (!inline || !first || first.type !== "text") continue;
    const m = /^\[([ xX])\](?:\s+|$)/.exec(first.content);
    if (!m) continue;
    const checked = m[1] !== " ";
    first.content = first.content.slice(m[0].length);
    const line = t.map ? t.map[0] : -1;
    t.attrJoin("class", "md-task");
    const box = new state.Token("html_inline", "", 0);
    box.content = `<span class="kit-check-box${checked ? " on" : ""}" data-task-line="${line}">${CHECK_SVG}</span>`;
    inline.children = [box, ...(inline.children ?? [])];
  }
  return false;
}

/** anikit 图片 URL（2026-10-08 S3）：缩略图 convertFileSrc + ?v=modifiedAt 版本戳（AssetCard 同口径）；
 *  资产不在库 / 无缩略图 = null 落占位文本（库侧前端不持 originPath，不回退源文件） */
function anikitImageUrl(assetId: string): string | null {
  const p = useContentStore().placements.find((x) => x.assetId === assetId);
  if (!p?.thumbPath) return null;
  return `${convertFileSrc(p.thumbPath)}?v=${encodeURIComponent(p.modifiedAt ?? "")}`;
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** 创建 markdown-it 渲染实例（PreviewText 单例；渲染无状态可多组件共享） */
export function createMd(): MdIt {
  const md = new MarkdownIt({ html: false, linkify: true, breaks: false });
  // anikit:// 协议放行（默认 validateLink 只认 http/https/ftp/mailto，自定义协议 href 会被掐空）
  const validate0 = md.validateLink.bind(md);
  md.validateLink = (url: string) => url.startsWith("anikit://") || validate0(url);
  // 图片 = anikit 引用渲染真图（点击 = 内部跳转，PreviewText 代理接 data-anikit-id）；其余占位文本
  md.renderer.rules.image = (tokens, idx) => {
    const t = tokens[idx];
    const alt = t.content || "未命名";
    const fallback = `<span class="md-img">[图片 ${escapeHtml(alt)}]</span>`;
    const ref = parseAnikitRef(String(t.attrGet("src") ?? ""));
    if (!ref || ref.kind !== "item") return fallback;
    const url = anikitImageUrl(ref.id);
    if (!url) return fallback;
    return `<img class="md-asset" src="${escapeHtml(url)}" alt="${escapeHtml(alt)}" data-anikit-id="${ref.id}" loading="lazy">`;
  };
  md.core.ruler.after("inline", "kit-task-list", kitTaskList);
  md.core.ruler.after("inline", "kit-line-map", kitLineMap);
  const fence0 = md.renderer.rules.fence!;
  md.renderer.rules.fence = (t, i, o, e, s) => wrapCode(fence0(t, i, o, e, s), t[i].map?.[0] ?? -1);
  const codeBlock0 = md.renderer.rules.code_block!;
  md.renderer.rules.code_block = (t, i, o, e, s) => wrapCode(codeBlock0(t, i, o, e, s), t[i].map?.[0] ?? -1);
  return md;
}
