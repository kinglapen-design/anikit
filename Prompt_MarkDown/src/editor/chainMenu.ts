import { Prec } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import type { ViewUpdate } from "@codemirror/view";
import { templateConfig } from "./templatesConfig";

interface Segment {
  ruleIdx: number;
  from: number;
  to: number;
}

interface ChainSession {
  chainName: string;
  rules: string[];
  idx: number;
  active: number;
  segments: Segment[];
}

let session: ChainSession | null = null;
let panel: HTMLElement | null = null;
let viewRef: EditorView | null = null;

function currentItems(): string[] {
  if (!session) return [];
  return templateConfig.menus[session.rules[session.idx]] ?? [];
}

function showPanel(view: EditorView) {
  hidePanel();
  panel = document.createElement("div");
  panel.className = "chain-panel";
  document.body.appendChild(panel);
  renderPanel(view);
}

function hidePanel() {
  panel?.remove();
  panel = null;
}

function renderPanel(view: EditorView) {
  if (!panel || !session) return;
  const items = currentItems();
  panel.innerHTML = "";
  const title = document.createElement("div");
  title.className = "chain-panel-title";
  title.textContent = `${session.chainName} · ${session.rules[session.idx]} ${session.idx + 1}/${session.rules.length}`;
  panel.appendChild(title);
  items.forEach((text, i) => {
    const row = document.createElement("div");
    row.className = "chain-item" + (i === session!.active ? " active" : "");
    const num = document.createElement("span");
    num.className = "num";
    num.textContent = String(i + 1);
    const label = document.createElement("span");
    label.textContent = text;
    row.append(num, label);
    row.addEventListener("mousedown", (e) => {
      e.preventDefault();
      selectItem(i);
    });
    panel!.appendChild(row);
  });
  const coords = view.coordsAtPos(view.state.selection.main.head);
  if (coords) {
    panel.style.left = `${Math.min(coords.left, window.innerWidth - 220)}px`;
    panel.style.top = "auto";
    panel.style.bottom = `${window.innerHeight - coords.top + 8}px`;
  }
}

function endSession() {
  session = null;
  hidePanel();
}

function advance(view: EditorView) {
  if (!session) return;
  session.idx += 1;
  session.active = 0;
  if (session.idx >= session.rules.length) {
    endSession();
    return;
  }
  renderPanel(view);
}

function selectItem(i: number) {
  if (!session || !viewRef) return;
  const items = currentItems();
  const text = items[i];
  if (text === undefined) return;
  const view = viewRef;
  const head = view.state.selection.main.head;
  const insert = session.segments.length === 0 ? text : `，${text}`;
  session.segments.push({ ruleIdx: session.idx, from: head, to: head + insert.length });
  view.dispatch({
    changes: { from: head, insert },
    selection: { anchor: head + insert.length },
  });
  if (!session) return;
  advance(view);
}

function back(view: EditorView) {
  if (!session || session.idx === 0) return;
  session.idx -= 1;
  session.active = 0;
  const segIdx = session.segments.findIndex((s) => s.ruleIdx === session!.idx);
  if (segIdx >= 0) {
    const seg = session.segments[segIdx];
    session.segments.splice(segIdx, 1);
    view.dispatch({ changes: { from: seg.from, to: seg.to } });
    if (!session) return;
  }
  renderPanel(view);
}

function expectedHead(): number {
  if (!session) return -1;
  const last = session.segments[session.segments.length - 1];
  return last ? last.to : session.segments.length === 0 ? sessionHeadStart : -1;
}

let sessionHeadStart = 0;

function sessionAlive(view: EditorView): boolean {
  if (!session) return false;
  const head = view.state.selection.main.head;
  return head === expectedHead();
}

export const chainKeymap = Prec.high(
  keymap.of([
    {
      key: "Escape",
      run: (v) => {
        if (!sessionAlive(v)) return false;
        endSession();
        return true;
      },
    },
    {
      key: "Tab",
      run: (v) => {
        if (!sessionAlive(v)) return false;
        advance(v);
        return true;
      },
    },
    {
      key: "Shift-Tab",
      run: (v) => {
        if (!sessionAlive(v)) return false;
        back(v);
        return true;
      },
    },
    {
      key: "Enter",
      run: (v) => {
        if (!sessionAlive(v)) return false;
        selectItem(session!.active);
        return true;
      },
    },
    {
      key: "ArrowUp",
      run: (v) => {
        if (!sessionAlive(v)) return false;
        const items = currentItems();
        if (items.length > 0) {
          session!.active = (session!.active - 1 + items.length) % items.length;
          renderPanel(v);
        }
        return true;
      },
    },
    {
      key: "ArrowDown",
      run: (v) => {
        if (!sessionAlive(v)) return false;
        const items = currentItems();
        if (items.length > 0) {
          session!.active = (session!.active + 1) % items.length;
          renderPanel(v);
        }
        return true;
      },
    },
    ...[1, 2, 3, 4, 5, 6, 7, 8, 9].map((d) => ({
      key: String(d),
      run: (v: EditorView) => {
        if (!sessionAlive(v)) return false;
        if (d - 1 >= currentItems().length) return false;
        selectItem(d - 1);
        return true;
      },
    })),
  ]),
);

export const chainTrigger = EditorView.updateListener.of((u: ViewUpdate) => {
  if (session && !sessionAlive(u.view)) {
    endSession();
  }
  if (session || !u.docChanged) return;
  const head = u.state.selection.main.head;
  if (head === 0) return;
  let slashTyped = false;
  u.changes.iterChanges((_fromA, _toA, _fromB, toB, inserted) => {
    if (toB === head && inserted.toString().endsWith("/")) slashTyped = true;
  });
  if (!slashTyped) return;
  const chain = templateConfig.chains[0];
  if (!chain || chain.rules.length === 0) return;
  viewRef = u.view;
  sessionHeadStart = head - 1;
  session = {
    chainName: chain.name,
    rules: chain.rules,
    idx: 0,
    active: 0,
    segments: [],
  };
  u.view.dispatch({ changes: { from: head - 1, to: head } });
  showPanel(u.view);
});
