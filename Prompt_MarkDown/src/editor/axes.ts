import { EditorView, keymap } from "@codemirror/view";

const PITCH_UP = ["微仰拍", "仰拍", "大仰拍"];
const PITCH_DOWN = ["微俯拍", "俯拍", "大俯拍"];
const DIST = ["特写", "近景", "中景", "全景", "远景"];

interface AxisSession {
  from: number;
  rendered: string;
  h: -1 | 0 | 1;
  d: -1 | 0 | 1;
  pitchDir: -1 | 0 | 1;
  pitchIdx: number;
  distIdx: number;
}

let session: AxisSession | null = null;

function render(s: AxisSession): string {
  const parts: string[] = [];
  if (s.d !== 0) {
    parts.push(`${s.h === 0 ? "正" : s.h < 0 ? "左" : "右"}${s.d > 0 ? "前" : "后"}方`);
  } else if (s.h !== 0) {
    parts.push(s.h < 0 ? "左侧" : "右侧");
  }
  if (s.pitchDir !== 0) parts.push((s.pitchDir > 0 ? PITCH_UP : PITCH_DOWN)[s.pitchIdx]);
  if (s.distIdx >= 0) parts.push(DIST[s.distIdx]);
  return parts.join("，");
}

function matchSession(view: EditorView): AxisSession | null {
  if (!session) return null;
  const head = view.state.selection.main.head;
  if (head !== session.from + session.rendered.length) return null;
  if (view.state.doc.sliceString(session.from, head) !== session.rendered) return null;
  return session;
}

function applyAxis(view: EditorView, mutate: (s: AxisSession) => void): boolean {
  let s = matchSession(view);
  if (!s) {
    s = {
      from: view.state.selection.main.head,
      rendered: "",
      h: 0,
      d: 0,
      pitchDir: 0,
      pitchIdx: -1,
      distIdx: -1,
    };
    session = s;
  }
  mutate(s);
  const text = render(s);
  view.dispatch({
    changes: { from: s.from, to: s.from + s.rendered.length, insert: text },
    selection: { anchor: s.from + text.length },
  });
  s.rendered = text;
  return true;
}

export const axesKeymap = keymap.of([
  { key: "Ctrl-ArrowUp", run: (v) => applyAxis(v, (s) => { s.d = s.d === 1 ? 0 : 1; }) },
  { key: "Ctrl-ArrowDown", run: (v) => applyAxis(v, (s) => { s.d = s.d === -1 ? 0 : -1; }) },
  { key: "Ctrl-ArrowLeft", run: (v) => applyAxis(v, (s) => { s.h = s.h === -1 ? 0 : -1; }) },
  { key: "Ctrl-ArrowRight", run: (v) => applyAxis(v, (s) => { s.h = s.h === 1 ? 0 : 1; }) },
  {
    key: "Ctrl-Shift-ArrowUp",
    run: (v) =>
      applyAxis(v, (s) => {
        if (s.pitchDir === 1) s.pitchIdx = (s.pitchIdx + 1) % PITCH_UP.length;
        else {
          s.pitchDir = 1;
          s.pitchIdx = 0;
        }
      }),
  },
  {
    key: "Ctrl-Shift-ArrowDown",
    run: (v) =>
      applyAxis(v, (s) => {
        if (s.pitchDir === -1) s.pitchIdx = (s.pitchIdx + 1) % PITCH_DOWN.length;
        else {
          s.pitchDir = -1;
          s.pitchIdx = 0;
        }
      }),
  },
  {
    key: "Ctrl-Alt-ArrowUp",
    run: (v) =>
      applyAxis(v, (s) => {
        s.distIdx = s.distIdx < 0 ? 1 : (s.distIdx + DIST.length - 1) % DIST.length;
      }),
  },
  {
    key: "Ctrl-Alt-ArrowDown",
    run: (v) =>
      applyAxis(v, (s) => {
        s.distIdx = s.distIdx < 0 ? DIST.length - 2 : (s.distIdx + 1) % DIST.length;
      }),
  },
]);
