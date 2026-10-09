// md 编辑器内核 = CodeMirror 6 + Live Preview 装饰层（2026-10-10 换代，PRD 04-04 条 6：
// 「不引入所见即所得」口径作废，用户定 Obsidian 式——光标外行内联渲染、光标行回落源码；
// 文档源仍是 md 字符串，textarea 内核与自实现 mdHistory 快照栈一并退役，撤销 = CM6 原生事务栈）。
// 工厂 createMdEditor = 唯一入口（PreviewText 调用）；纯装配层：排版数值全部走 token（styles/cm-md.css），
// anikit 图片 widget / 任务框 widget / 粘贴补全复用 lib/anikitLink 与 stores，UI 零业务逻辑。
// 装饰层口径：
// - 行级样式恒生效（标题字号 / 引用竖线 / 代码块卡片——光标在内排版不塌，Obsidian 同）；
// - 隐标记 / widget 替换仅光标外行（active 判定 = 装饰区间与任一选区相交，光标贴边也算在内）；
// - 任务框 / 图片 widget 例外恒渲染（框点击 toggle、图点击跳转；光标进图区间回落源码便于改 alt/url）；
// - 表格 / Setext 标题本轮不装饰（保源码态，待另轮）；块距 = md 空行自然撑开（非渲染态 flex gap 12）。

import { EditorState, Prec, type Extension, type Range } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  ViewPlugin,
  WidgetType,
  drawSelection,
  keymap,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting, syntaxTree } from "@codemirror/language";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { tags } from "@lezer/highlight";
import type { SyntaxNode } from "@lezer/common";
import { anikitThumbUrl, jumpToAnikitRef, parseAnikitRef, type AnikitRef } from "./anikitLink";
import { useContentStore } from "../stores/content";
import { getIcon } from "./icons";

const CHECK_SVG = getIcon("check-tiny").replace("<svg", '<svg class="kit-check-mark"');

export interface MdEditorOpts {
  parent: HTMLElement;
  doc: string;
  /** 文档变更（含撤销 / 工具栏 / 粘贴）→ draft 同步（防抖保存由调用方既有管线负责） */
  onDocChanged: (text: string) => void;
  /** 光标活动 / 输入（分屏 typewriter 跟滚触发点，= 旧 textarea keyup/click/input 口径） */
  onCursorActivity: () => void;
  /** 滚动（滚动保活记账 + 分屏联动） */
  onScroll: () => void;
  /** Esc / Ctrl+Enter / Ctrl+E = 回预览态 */
  onExitEdit: () => void;
}

export interface MdEditorHandle {
  view: EditorView;
  /** 外部 draft 变更回灌（分屏右栏复选框 toggle 等）：整文替换 + 光标钳位保留 */
  setDoc: (text: string) => void;
  /** 悬浮栏工具栏 19 动作执行点：源码变换结果整文替换 + 选区还原；每次点击独立撤销步（userEvent 带序号恒开新组） */
  applySource: (text: string, start: number, end: number, seq: number) => void;
  /** 光标落末尾 + 聚焦（进编辑态「接着写」口径） */
  focusEnd: () => void;
  destroy: () => void;
}

// —— 任务复选框 widget（替换 `[ ]` / `[x]` 标记 + 尾随空格；点击 = 按行回写，恒渲染不光标行回落） ——
class CheckboxWidget extends WidgetType {
  constructor(
    readonly checked: boolean,
    readonly from: number,
  ) {
    super();
  }
  override eq(w: CheckboxWidget): boolean {
    return w.checked === this.checked && w.from === this.from;
  }
  override toDOM(view: EditorView): HTMLElement {
    const s = document.createElement("span");
    s.className = `kit-check-box cm-lp-task${this.checked ? " on" : ""}`;
    s.innerHTML = CHECK_SVG;
    s.onmousedown = (e) => e.preventDefault(); // 点击不挪光标（框是开关不是文本）
    s.onclick = () => {
      // 位置随编辑漂移——以行重扫为准（toggleTaskLine 同口径）
      const line = view.state.doc.lineAt(Math.min(this.from, view.state.doc.length));
      const i = line.text.search(/\[[ xX]\]/);
      if (i < 0) return;
      const on = line.text[i + 1] === " ";
      view.dispatch({
        changes: { from: line.from + i, to: line.from + i + 3, insert: on ? "[x]" : "[ ]" },
        userEvent: "md.task",
      });
    };
    return s;
  }
}

// —— anikit 图片 widget（替换整个 ![alt](anikit://item/id)；光标进区间回落源码） ——
class ImageWidget extends WidgetType {
  constructor(
    readonly ref: AnikitRef,
    readonly alt: string,
    readonly url: string | null,
  ) {
    super();
  }
  override eq(w: ImageWidget): boolean {
    return w.ref.id === this.ref.id && w.url === this.url && w.alt === this.alt;
  }
  override toDOM(): HTMLElement {
    if (!this.url) {
      const s = document.createElement("span");
      s.className = "md-img";
      s.textContent = `[图片 ${this.alt || "未命名"}]`;
      return s;
    }
    const img = document.createElement("img");
    img.className = "md-asset";
    img.src = this.url;
    img.alt = this.alt;
    img.loading = "lazy";
    img.onmousedown = (e) => e.preventDefault();
    img.onclick = () => jumpToAnikitRef(this.ref);
    return img;
  }
}

// —— 分割线 widget（光标外行内容整段替换为 2px 通用/三级横线，稿 Line 7 同档） ——
class HrWidget extends WidgetType {
  override eq(): boolean {
    return true;
  }
  override toDOM(): HTMLElement {
    const d = document.createElement("div");
    d.className = "cm-lp-hr";
    return d;
  }
}

/** 装饰区间是否与任一选区相交（光标贴边 = 在内——回落源码便于编辑） */
function isActive(state: EditorState, from: number, to: number): boolean {
  return state.selection.ranges.some((r) => r.from <= to && r.to >= from);
}

/** 标记隐藏 = 零宽替换（仅在非 active 时调用方加入） */
const HIDE = Decoration.replace({});

// Live Preview 装饰构建：语法树（可见区）→ 行级样式恒挂 + 非 active 隐标记 / widget
function buildLpDeco(view: EditorView): DecorationSet {
  const { state } = view;
  const decos: Range<Decoration>[] = [];
  const hidden = new Set<string>(); // 嵌套标记去重（***粗斜*** 内外层同名 mark 不重复藏，防 replace 重叠报错）
  const lineDeco = (from: number, cls: string): void => {
    decos.push(Decoration.line({ class: cls }).range(from));
  };
  const hideMarks = (n: SyntaxNode, markName: string): void => {
    for (let c = n.firstChild; c; c = c.nextSibling) {
      if (c.name !== markName) continue;
      const key = `${c.from},${c.to}`;
      if (!hidden.has(key)) {
        hidden.add(key);
        decos.push(HIDE.range(c.from, c.to));
      }
    }
  };

  for (const vr of view.visibleRanges) {
    syntaxTree(state).iterate({
      from: vr.from,
      to: vr.to,
      enter(node) {
        const name = node.name;
        // —— 标题（ATX 六级）：行字号恒挂，# 标记 + 尾随空格光标外隐藏 ——
        const h = /^ATXHeading([1-6])$/.exec(name);
        if (h) {
          const n = Number(h[1]);
          const lFrom = state.doc.lineAt(node.from).from;
          lineDeco(lFrom, `cm-lp-h${n}`);
          if (!isActive(state, node.from, node.to)) {
            const mark = node.node.getChild("HeaderMark");
            if (mark) {
              let to = mark.to;
              if (state.sliceDoc(to, to + 1) === " ") to++; // 连空格一起藏（稿标题无前导空格）
              decos.push(HIDE.range(mark.from, to));
            }
          }
          return true; // 继续下探——标题内行内格式（** 等）标记照藏
        }
        switch (name) {
          // —— 引用：行竖线恒挂，> 标记光标行外隐藏 ——
          case "Blockquote": {
            const fromLine = state.doc.lineAt(node.from);
            const toLine = state.doc.lineAt(Math.min(node.to, state.doc.length));
            for (let n = fromLine.number; n <= toLine.number; n++) {
              lineDeco(state.doc.line(n).from, "cm-lp-quote");
            }
            return true; // 继续下探 QuoteMark 子节点
          }
          case "QuoteMark": {
            const l = state.doc.lineAt(node.from);
            if (!isActive(state, l.from, l.to)) {
              let to = node.to;
              if (state.sliceDoc(to, to + 1) === " ") to++;
              const key = `${node.from},${to}`;
              if (!hidden.has(key)) {
                hidden.add(key);
                decos.push(HIDE.range(node.from, to));
              }
            }
            return false;
          }
          // —— 行内格式：样式恒挂，标记光标外隐藏（不下探拦截——嵌套格式内层标记照藏） ——
          case "StrongEmphasis":
            decos.push(Decoration.mark({ class: "cm-lp-strong" }).range(node.from, node.to));
            if (!isActive(state, node.from, node.to)) hideMarks(node.node, "EmphasisMark");
            return true;
          case "Emphasis":
            decos.push(Decoration.mark({ class: "cm-lp-em" }).range(node.from, node.to));
            if (!isActive(state, node.from, node.to)) hideMarks(node.node, "EmphasisMark");
            return true;
          case "Strikethrough":
            decos.push(Decoration.mark({ class: "cm-lp-strike" }).range(node.from, node.to));
            if (!isActive(state, node.from, node.to)) hideMarks(node.node, "StrikethroughMark");
            return true;
          case "InlineCode":
            decos.push(Decoration.mark({ class: "cm-lp-inline-code" }).range(node.from, node.to));
            if (!isActive(state, node.from, node.to)) hideMarks(node.node, "CodeMark");
            return true;
          // —— 链接：品牌色恒挂，[]() 与 URL 光标外隐藏 ——
          case "Link": {
            decos.push(Decoration.mark({ class: "cm-lp-link" }).range(node.from, node.to));
            if (!isActive(state, node.from, node.to)) {
              hideMarks(node.node, "LinkMark");
              const url = node.node.getChild("URL");
              if (url) decos.push(HIDE.range(url.from, url.to));
            }
            return false;
          }
          // —— 图片：anikit 引用 = 真图 widget；其余 / 无缩略图 = 占位文本；光标在内回落源码 ——
          case "Image": {
            if (isActive(state, node.from, node.to)) return false;
            const urlNode = node.node.getChild("URL");
            // alt = 首末 LinkMark 之间的文本（![alt](url) 形态；无 ImageDescription 节点，lezer-markdown 无此拆）
            const marks: SyntaxNode[] = [];
            for (let c = node.node.firstChild; c; c = c.nextSibling) {
              if (c.name === "LinkMark") marks.push(c);
            }
            const altText =
              marks.length >= 2 ? state.sliceDoc(marks[0].to, marks[marks.length - 1].from).replace(/^\[|\]$/g, "") : "";
            const ref = urlNode ? parseAnikitRef(state.sliceDoc(urlNode.from, urlNode.to)) : null;
            const w =
              ref && ref.kind === "item"
                ? new ImageWidget(ref, altText, anikitThumbUrl(ref.id))
                : new ImageWidget({ kind: "item", id: "" }, altText, null);
            decos.push(Decoration.replace({ widget: w }).range(node.from, node.to));
            return false;
          }
          // —— 任务框：恒 widget（点击开关）；尾随空格随标记一起换（框距由 .cm-lp-task margin 供） ——
          case "TaskMarker": {
            let to = node.to;
            if (state.sliceDoc(to, to + 1) === " ") to++;
            const src = state.sliceDoc(node.from, node.to);
            const checked = src.includes("x") || src.includes("X");
            decos.push(Decoration.replace({ widget: new CheckboxWidget(checked, node.from) }).range(node.from, to));
            return false;
          }
          // —— 代码块：整段行挂卡片样式（首末行圆角），栅线 / 信息行保留源码（Dimmed 色档） ——
          case "FencedCode":
          case "CodeBlock": {
            decos.push(Decoration.mark({ class: "cm-lp-code-text" }).range(node.from, node.to));
            const fromLine = state.doc.lineAt(node.from);
            const toLine = state.doc.lineAt(Math.min(node.to, state.doc.length));
            for (let n = fromLine.number; n <= toLine.number; n++) {
              const l = state.doc.line(n);
              let cls = "cm-lp-code-line";
              if (n === fromLine.number) cls += " cm-lp-code-first";
              if (n === toLine.number) cls += " cm-lp-code-last";
              lineDeco(l.from, cls);
            }
            return false;
          }
          // —— 分割线：光标外整行换横线 widget ——
          case "HorizontalRule": {
            const l = state.doc.lineAt(node.from);
            if (!isActive(state, l.from, l.to)) {
              decos.push(Decoration.replace({ widget: new HrWidget() }).range(l.from, l.to));
            }
            return false;
          }
          default:
            return true;
        }
      },
    });
  }

  return Decoration.set(decos, true);
}

const lpPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildLpDeco(view);
    }
    update(u: ViewUpdate): void {
      if (
        u.docChanged ||
        u.viewportChanged ||
        u.selectionSet ||
        u.geometryChanged ||
        syntaxTree(u.startState) !== syntaxTree(u.state)
      ) {
        this.decorations = buildLpDeco(u.view);
      }
    }
  },
  { decorations: (v) => v.decorations },
);

// 源码回落态的语法高亮（激活行标记可见时的色档；排版数值在 styles/cm-md.css）
const mdHighlight = HighlightStyle.define([
  { tag: tags.heading, color: "var(--n-md-title)" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "bold" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: tags.monospace, fontFamily: "var(--n-font-mono)", color: "var(--n-md-other)" },
  { tag: tags.link, color: "var(--n-brand)" },
  { tag: tags.url, color: "var(--n-l3)" },
  { tag: tags.quote, color: "var(--n-md-other)" },
  { tag: [tags.processingInstruction, tags.punctuation], color: "var(--n-l3)" },
]);

/** 创建 md Live Preview 编辑器（挂载到 opts.parent；销毁务必 destroy） */
export function createMdEditor(opts: MdEditorOpts): MdEditorHandle {
  const extensions: Extension[] = [
    history(), // 原生事务撤销栈（键入按 500ms 新组窗合并；工具栏 / toggle / 粘贴以 userEvent 分步）
    drawSelection(),
    EditorState.allowMultipleSelections.of(false),
    markdown({ base: markdownLanguage, codeLanguages: languages }), // 自带 Enter 续列表 / 退格剥标记键图
    syntaxHighlighting(mdHighlight),
    lpPlugin,
    EditorView.lineWrapping,
    Prec.high(
      keymap.of([
        { key: "Escape", run: () => (opts.onExitEdit(), true) },
        { key: "Ctrl-Enter", run: () => (opts.onExitEdit(), true) },
        { key: "Ctrl-e", run: () => (opts.onExitEdit(), true) }, // 编辑态 Ctrl+E = 回预览（预览态进编辑走全局键）
      ]),
    ),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) opts.onDocChanged(u.state.doc.toString());
      if (u.docChanged || u.selectionSet) opts.onCursorActivity();
    }),
    EditorView.domEventHandlers({
      scroll: () => opts.onScroll(),
      // 粘贴补全（2026-10-08 S3 口径平移）：剪贴板整串为 anikit item 引用 → 插入图片语法
      paste: (e, view) => {
        const text = e.clipboardData?.getData("text/plain") ?? "";
        const ref = parseAnikitRef(text);
        if (!ref || ref.kind !== "item") return;
        e.preventDefault();
        const name = useContentStore().placements.find((x) => x.assetId === ref.id)?.name ?? "图片";
        view.dispatch(view.state.replaceSelection(`![${name}](anikit://item/${ref.id})`), { userEvent: "input.paste" });
      },
    }),
  ];

  const view = new EditorView({
    parent: opts.parent,
    state: EditorState.create({ doc: opts.doc, extensions }),
  });
  // 探针钩子（tools/cdp-md-editor-probe.py：CDP 语境拿不到 EditorView 类，经宿主元素拿到实例驱动选区 / 读文档）
  (opts.parent as unknown as { __cmView?: EditorView }).__cmView = view;

  return {
    view,
    setDoc(text) {
      const cur = view.state.doc.toString();
      if (text === cur) return;
      view.dispatch({
        changes: { from: 0, to: cur.length, insert: text },
        selection: { anchor: Math.min(view.state.selection.main.head, text.length) },
        userEvent: "md.external", // 外部回灌（分屏右栏 toggle）独立成步，不并入键入
      });
    },
    applySource(text, start, end, seq) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: text },
        selection: { anchor: start, head: end },
        userEvent: `md.toolbar.${seq}`, // 每次工具栏点击恒为独立撤销步
        scrollIntoView: true,
      });
      view.focus();
    },
    focusEnd() {
      view.dispatch({ selection: { anchor: view.state.doc.length } });
      view.focus();
    },
    destroy() {
      view.destroy();
    },
  };
}
