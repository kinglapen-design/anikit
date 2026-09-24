// MarkDown 编辑操作（纯函数，UI 零业务逻辑——规范第 1 条命令接口层）：
// applyMdAction = 悬浮栏 MarkDown 工具栏 19 动作（FloatBarMarkdown emit 语义名 → 源码选区变换）；
// toggleTaskLine = 渲染态复选框点击按行号回写 `- [ ]` ↔ `- [x]`（data-task-line 由 lib/markdown 注入）。

export interface MdEditResult {
  text: string;
  start: number; // 新选区起（原文串索引）
  end: number; // 新选区止
}

// 行首块标记 = 标题 / 引用 / 列表（无序·有序·任务）任意叠层
const PREFIXES_RE = /^(\s*(?:#{1,6}\s+|>\s+|(?:[-*+]|\d+\.)\s+\[[ xX]\]\s+|(?:[-*+]|\d+\.)\s+))*/;
const stripPrefixes = (line: string): string => line.replace(PREFIXES_RE, "");

/** pos 所在行的区间 [起, 止)（止 = 换行符位置 / 串尾） */
function lineRangeAt(text: string, pos: number): [number, number] {
  const s = text.lastIndexOf("\n", Math.max(0, pos - 1)) + 1;
  let e = text.indexOf("\n", pos);
  if (e < 0) e = text.length;
  return [s, e];
}

/** 对选区覆盖的所有行（含部分行）逐行变换；新选区 = 首行起至末行止整段 */
function mapLines(text: string, start: number, end: number, fn: (line: string) => string): MdEditResult {
  const [ls] = lineRangeAt(text, start);
  const [, le] = lineRangeAt(text, end);
  const head = text.slice(0, ls);
  const tail = text.slice(le);
  const body = text
    .slice(ls, le)
    .split("\n")
    .map(fn)
    .join("\n");
  return { text: head + body + tail, start: ls, end: ls + body.length };
}

/** 选区包裹标记（再点 = 剥除 toggle） */
function wrapSel(text: string, start: number, end: number, mark: string): MdEditResult {
  const sel = text.slice(start, end);
  if (sel.length >= mark.length * 2 && sel.startsWith(mark) && sel.endsWith(mark)) {
    const inner = sel.slice(mark.length, -mark.length);
    const out = text.slice(0, start) + inner + text.slice(end);
    return { text: out, start, end: start + inner.length };
  }
  const out = text.slice(0, start) + mark + sel + mark + text.slice(end);
  return { text: out, start: start + mark.length, end: end + mark.length };
}

/** 选区替换为插入模板；cursorIn = 模板内应选中的片段（选中便于直接输入替换） */
function insertTpl(text: string, start: number, end: number, tpl: string, cursorIn?: string): MdEditResult {
  const out = text.slice(0, start) + tpl + text.slice(end);
  const at = cursorIn ? tpl.indexOf(cursorIn) : tpl.length;
  return { text: out, start: start + at, end: start + at + (cursorIn?.length ?? 0) };
}

/** 悬浮栏工具栏动作：作用于源码选区，返回新全文与新选区（未知动作原样返回） */
export function applyMdAction(source: string, start: number, end: number, action: string): MdEditResult {
  const same: MdEditResult = { text: source, start, end };
  const h = /^h([1-6])$/.exec(action);
  if (h) {
    const n = Number(h[1]);
    const levelRe = new RegExp(`^\\s*#{${n}}(?!#)(\\s|$)`);
    return mapLines(source, start, end, (l) =>
      levelRe.test(l) ? stripPrefixes(l) : `${"#".repeat(n)} ${stripPrefixes(l)}`,
    );
  }
  switch (action) {
    case "body":
      return mapLines(source, start, end, stripPrefixes);
    case "quote":
      return mapLines(source, start, end, (l) =>
        /^\s*>/.test(l) ? l.replace(/^(\s*)>\s?/, "$1") : l.replace(/^(\s*)/, "$1> "),
      );
    case "ul":
      return mapLines(source, start, end, (l) =>
        /^\s*[-*+]\s/.test(l) ? stripPrefixes(l) : `- ${stripPrefixes(l)}`,
      );
    case "ol":
      return mapLines(source, start, end, (l) =>
        /^\s*\d+\.\s/.test(l) ? stripPrefixes(l) : `1. ${stripPrefixes(l)}`,
      );
    case "task":
      return mapLines(source, start, end, (l) =>
        /^\s*[-*+]\s+\[[ xX]\]\s/.test(l) ? stripPrefixes(l) : `- [ ] ${stripPrefixes(l)}`,
      );
    case "bold":
      return wrapSel(source, start, end, "**");
    case "italic":
      return wrapSel(source, start, end, "*");
    case "strike":
      return wrapSel(source, start, end, "~~");
    case "code-inline":
      return wrapSel(source, start, end, "`");
    case "link": {
      const sel = source.slice(start, end) || "链接文字";
      return insertTpl(source, start, end, `[${sel}](url)`, "url");
    }
    case "code-block": {
      const sel = source.slice(start, end);
      if (sel) return insertTpl(source, start, end, `\`\`\`\n${sel}\n\`\`\``);
      return insertTpl(source, start, end, "```\n\n```", "\n");
    }
    case "table":
      return insertTpl(
        source,
        start,
        end,
        "\n\n| 列 1 | 列 2 | 列 3 |\n| --- | --- | --- |\n|  |  |  |\n",
        "列 1",
      );
    case "divider":
      return insertTpl(source, start, end, "\n\n---\n\n");
    default:
      return same;
  }
}

/** 渲染态任务复选框 toggle：按行号（0 起，lib/markdown 注入的 data-task-line）翻转 `[ ]` ↔ `[x]` */
export function toggleTaskLine(source: string, line: number): string {
  const lines = source.split("\n");
  if (line < 0 || line >= lines.length) return source;
  const l = lines[line];
  const i = l.search(/\[[ xX]\]/);
  if (i < 0) return source;
  lines[line] = `${l.slice(0, i + 1)}${l[i + 1] === " " ? "x" : " "}${l.slice(i + 2)}`;
  return lines.join("\n");
}
