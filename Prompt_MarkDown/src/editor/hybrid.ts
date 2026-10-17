import { RangeSetBuilder } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, WidgetType } from "@codemirror/view";
import type { DecorationSet, ViewUpdate } from "@codemirror/view";

const RE_HEADING = /^(#{1,6})\s+/;
const RE_HR = /^\s{0,3}(?:-{3,}|\*{3,}|_{3,})\s*$/;
const RE_QUOTE = /^(?:>\s?)+/;
const RE_LIST = /^(\s*)([-*+])(\s+)/;
const RE_FENCE = /^```/;
const RE_BOLD = /\*\*([^*]+)\*\*/g;
const RE_LINK = /\[([^\]]+)\]\(([^)]+)\)/g;

class DotWidget extends WidgetType {
  toDOM(): HTMLElement {
    const span = document.createElement("span");
    span.className = "md-li-dot";
    span.textContent = "• ";
    return span;
  }

  eq(): boolean {
    return true;
  }
}
const dotWidget = new DotWidget();

interface Pending {
  from: number;
  to: number;
  deco: Decoration;
}

function lineClasses(text: string, inFence: boolean, fenceLine: boolean): string[] {
  const classes: string[] = [];
  if (fenceLine) {
    classes.push("md-fence");
    return classes;
  }
  if (inFence) {
    classes.push("md-code");
    return classes;
  }
  const heading = RE_HEADING.exec(text);
  if (heading) classes.push(`md-h${heading[1].length}`);
  else if (RE_HR.test(text)) classes.push("md-hr");
  else {
    if (RE_QUOTE.test(text)) classes.push("md-quote");
    if (RE_LIST.test(text)) classes.push("md-li");
  }
  return classes;
}

function buildDecorations(view: EditorView): DecorationSet {
  const { doc } = view.state;
  const builder = new RangeSetBuilder<Decoration>();
  const selRanges = view.state.selection.ranges;

  for (const vr of view.visibleRanges) {
    let inFence = false;
    for (let pos = 0; pos < vr.from && pos < doc.length; ) {
      const line = doc.lineAt(pos);
      if (RE_FENCE.test(line.text)) inFence = !inFence;
      pos = line.to + 1;
    }

    for (let pos = vr.from; pos <= vr.to && pos <= doc.length; ) {
      const line = doc.lineAt(pos);
      pos = line.to + 1;
      const { text } = line;
      if (text.length === 0) {
        if (inFence) builder.add(line.from, line.from, Decoration.line({ class: "md-code" }));
        continue;
      }

      const fenceLine = RE_FENCE.test(text);
      const onCursorLine = selRanges.some((r) => r.from <= line.to && r.to >= line.from);
      const pending: Pending[] = [];

      if (fenceLine) {
        const classes = ["md-fence", inFence ? "md-fence-last" : "md-fence-first"];
        builder.add(line.from, line.from, Decoration.line({ class: classes.join(" ") }));
        inFence = !inFence;
        continue;
      }

      for (const cls of lineClasses(text, inFence, false)) {
        builder.add(line.from, line.from, Decoration.line({ class: cls }));
      }

      if (inFence) {
        const prevLine = line.number > 1 ? doc.line(line.number - 1) : null;
        const nextLine = line.number < doc.lines ? doc.line(line.number + 1) : null;
        if (prevLine && RE_FENCE.test(prevLine.text)) {
          builder.add(line.from, line.from, Decoration.line({ class: "md-code-first" }));
        }
        if (nextLine && RE_FENCE.test(nextLine.text)) {
          builder.add(line.from, line.from, Decoration.line({ class: "md-code-last" }));
        }
        continue;
      }

      const heading = RE_HEADING.exec(text);
      if (heading) {
        if (!onCursorLine) {
          pending.push({
            from: line.from,
            to: line.from + heading[0].length,
            deco: Decoration.replace({}),
          });
        }
      } else if (RE_HR.test(text)) {
        if (!onCursorLine) {
          pending.push({ from: line.from, to: line.to, deco: Decoration.replace({}) });
        }
      } else {
        const quote = RE_QUOTE.exec(text);
        if (quote && !onCursorLine) {
          pending.push({
            from: line.from,
            to: line.from + quote[0].length,
            deco: Decoration.replace({}),
          });
        }
        const list = RE_LIST.exec(text);
        if (list && !onCursorLine) {
          pending.push({
            from: line.from + list[1].length,
            to: line.from + list[0].length,
            deco: Decoration.replace({ widget: dotWidget }),
          });
        }

        for (const re of [RE_BOLD, RE_LINK]) re.lastIndex = 0;
        let m: RegExpExecArray | null;
        while ((m = RE_BOLD.exec(text))) {
          const s = line.from + m.index;
          const e = s + m[0].length;
          pending.push({ from: s + 2, to: e - 2, deco: Decoration.mark({ class: "md-strong" }) });
          if (!onCursorLine) {
            pending.push({ from: s, to: s + 2, deco: Decoration.replace({}) });
            pending.push({ from: e - 2, to: e, deco: Decoration.replace({}) });
          }
        }
        while ((m = RE_LINK.exec(text))) {
          const s = line.from + m.index;
          const e = s + m[0].length;
          const textEnd = s + 1 + m[1].length;
          pending.push({ from: s + 1, to: textEnd, deco: Decoration.mark({ class: "md-link" }) });
          if (!onCursorLine) {
            pending.push({ from: s, to: s + 1, deco: Decoration.replace({}) });
            pending.push({ from: textEnd, to: e, deco: Decoration.replace({}) });
          }
        }
      }

      pending.sort((a, b) => a.from - b.from || a.to - b.to);
      for (const p of pending) builder.add(p.from, p.to, p.deco);
    }
  }
  return builder.finish();
}

export const hybridPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged || u.selectionSet) {
        this.decorations = buildDecorations(u.view);
      }
    }
  },
  { decorations: (v) => v.decorations },
);
