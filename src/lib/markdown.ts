// MarkDown 渲染实例（PRD 04-04 条 6 定案 markdown-it；排版 = styles/markdown.css 全局类）：
// 任务列表（GFM `- [ ]` / `- [x]`）＝ 我们自己的复选框（2026-09 用户定）——core rule 剥 `[ ]` 前缀、
// li 挂 .md-task、文本前注入 .kit-check-box 静态框（styles/checkbox.css 通用类，Checkbox.vue 同款）；
// 框带 data-task-line（源码行号，0 起），点击 toggle 回写由 PreviewText 事件代理接（lib/mdEdit.toggleTaskLine）。
// 代码块 = div.md-code 包装 + pre.kit-scroll-x 横滚 + 右上角 .md-copy 复制钮（文本编辑稿，PromptSection 同款口径）。
// 图片引用 = 库内封闭路径解析不了 → 占位文本（09 待办）；链接本轮不可点（09 待办，CSS pointer-events 挡）。
import MarkdownIt, { type MarkdownIt as MdIt, type StateCore, type Token } from "markdown-it";
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

/** 创建 markdown-it 渲染实例（PreviewText 单例；渲染无状态可多组件共享） */
export function createMd(): MdIt {
  const md = new MarkdownIt({ html: false, linkify: true, breaks: false });
  // 图片引用解析不了（库内封闭）：渲染占位文本（09 待办：库内资产引用解析）
  md.renderer.rules.image = (tokens, idx) =>
    `<span class="md-img">[图片 ${tokens[idx].content || "未命名"}]</span>`;
  md.core.ruler.after("inline", "kit-task-list", kitTaskList);
  md.core.ruler.after("inline", "kit-line-map", kitLineMap);
  const fence0 = md.renderer.rules.fence!;
  md.renderer.rules.fence = (t, i, o, e, s) => wrapCode(fence0(t, i, o, e, s), t[i].map?.[0] ?? -1);
  const codeBlock0 = md.renderer.rules.code_block!;
  md.renderer.rules.code_block = (t, i, o, e, s) => wrapCode(codeBlock0(t, i, o, e, s), t[i].map?.[0] ?? -1);
  return md;
}
