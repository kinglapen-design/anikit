// dev 组件检查器（2026-11-02 用户定：opendesign 式——选中组件弹出属性面板，改间距实时看效果）。
// pick 模式 = 悬停高亮 + 点击选中；属性编辑 = inline style 覆盖（实时）；覆盖集按选择器存
// localStorage（刷新后重应用）；导出 = 选择器分块 CSS，贴回组件样式或发 AI 定稿。仅 dev preview 链路。

export type InspectorOverrides = Record<string, Record<string, string>>; // selector → cssProp → value

const STORAGE_KEY = "anikit.devInspectorOverrides.v1";

export interface InspectPropDef {
  key: string;
  label: string;
  kind: "length" | "text" | "select";
  options?: string[];
}

export const INSPECT_GROUPS: { name: string; props: InspectPropDef[] }[] = [
  {
    name: "宽高与间距",
    props: [
      { key: "width", label: "宽", kind: "length" },
      { key: "height", label: "高", kind: "length" },
      { key: "padding-top", label: "padding 上", kind: "length" },
      { key: "padding-right", label: "padding 右", kind: "length" },
      { key: "padding-bottom", label: "padding 下", kind: "length" },
      { key: "padding-left", label: "padding 左", kind: "length" },
      { key: "margin-top", label: "margin 上", kind: "length" },
      { key: "margin-right", label: "margin 右", kind: "length" },
      { key: "margin-bottom", label: "margin 下", kind: "length" },
      { key: "margin-left", label: "margin 左", kind: "length" },
      { key: "gap", label: "gap", kind: "length" },
      { key: "border-radius", label: "圆角", kind: "length" },
    ],
  },
  {
    name: "弹性布局",
    props: [
      { key: "display", label: "display", kind: "select", options: ["flex", "block", "inline-flex", "inline-block", "grid", "none"] },
      { key: "flex-direction", label: "方向", kind: "select", options: ["row", "column", "row-reverse", "column-reverse"] },
      { key: "justify-content", label: "主轴", kind: "select", options: ["flex-start", "center", "flex-end", "space-between", "space-around", "space-evenly"] },
      { key: "align-items", label: "交叉轴", kind: "select", options: ["flex-start", "center", "flex-end", "stretch", "baseline"] },
    ],
  },
  {
    name: "文字",
    props: [
      { key: "font-size", label: "字号", kind: "length" },
      { key: "line-height", label: "行高", kind: "length" },
      { key: "font-weight", label: "字重", kind: "select", options: ["400", "500", "700"] },
      { key: "color", label: "文字色", kind: "text" },
    ],
  },
  {
    name: "外观",
    props: [
      { key: "background", label: "背景", kind: "text" },
      { key: "border", label: "描边", kind: "text" },
      { key: "box-shadow", label: "阴影", kind: "text" },
    ],
  },
];

export function labelFor(el: HTMLElement): string {
  const cls = [...el.classList].filter((c) => !c.startsWith("dt-")).slice(0, 2).join(".");
  return `${el.tagName.toLowerCase()}${cls ? "." + cls : ""}`;
}

function segmentFor(el: HTMLElement): string {
  const tag = el.tagName.toLowerCase();
  if (el.id) return `#${el.id}`;
  const classes = [...el.classList].filter((c) => !c.startsWith("dt-"));
  const cls = classes.slice(0, 2).map((c) => "." + c).join("");
  const parent = el.parentElement;
  if (!parent) return tag + cls;
  const sameTag = [...parent.children].filter((c) => c.tagName === el.tagName);
  const nth = sameTag.length > 1 ? `:nth-of-type(${sameTag.indexOf(el) + 1})` : "";
  return tag + cls + nth;
}

/// 选择器生成：id 直达；否则沿父链最多 5 层（class 段 + 同 tag 多兄弟时 nth-of-type），导出可读可贴回
export function selectorFor(el: HTMLElement): string {
  if (el.id) return `#${el.id}`;
  const parts: string[] = [];
  let cur: HTMLElement | null = el;
  let depth = 0;
  while (cur && cur !== document.body && depth < 5) {
    parts.unshift(segmentFor(cur));
    if (cur.id) break;
    cur = cur.parentElement;
    depth++;
  }
  return parts.join(" > ");
}

export function ancestorsOf(el: HTMLElement): HTMLElement[] {
  const chain: HTMLElement[] = [];
  let cur = el.parentElement;
  while (cur && cur !== document.body && chain.length < 6) {
    chain.push(cur);
    cur = cur.parentElement;
  }
  return chain;
}

export function readComputed(el: HTMLElement): Record<string, string> {
  const cs = getComputedStyle(el);
  const out: Record<string, string> = {};
  for (const g of INSPECT_GROUPS) {
    for (const p of g.props) {
      out[p.key] = cs.getPropertyValue(p.key);
    }
  }
  return out;
}

export function loadOverrides(): InspectorOverrides {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) return parsed as InspectorOverrides;
  } catch {
    // 损坏快照按空，不阻塞
  }
  return {};
}

function saveOverrides(o: InspectorOverrides): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(o));
  } catch {
    // 写不进时仅本次会话生效
  }
}

export function setOverride(overrides: InspectorOverrides, el: HTMLElement, prop: string, value: string): InspectorOverrides {
  const selector = selectorFor(el);
  const next = { ...overrides, [selector]: { ...(overrides[selector] ?? {}), [prop]: value } };
  el.style.setProperty(prop, value);
  saveOverrides(next);
  return next;
}

export function clearOverride(overrides: InspectorOverrides, el: HTMLElement, prop: string): InspectorOverrides {
  const selector = selectorFor(el);
  const next: InspectorOverrides = { ...overrides };
  const props = { ...(next[selector] ?? {}) };
  delete props[prop];
  if (Object.keys(props).length) next[selector] = props;
  else delete next[selector];
  el.style.removeProperty(prop);
  saveOverrides(next);
  return next;
}

export function clearElementOverrides(overrides: InspectorOverrides, el: HTMLElement): InspectorOverrides {
  const selector = selectorFor(el);
  const next = { ...overrides };
  for (const prop of Object.keys(next[selector] ?? {})) {
    el.style.removeProperty(prop);
  }
  delete next[selector];
  saveOverrides(next);
  return next;
}

export function clearAllOverrides(overrides: InspectorOverrides): InspectorOverrides {
  for (const [selector, props] of Object.entries(overrides)) {
    for (const el of safeQueryAll(selector)) {
      for (const prop of Object.keys(props)) el.style.removeProperty(prop);
    }
  }
  saveOverrides({});
  return {};
}

function safeQueryAll(selector: string): HTMLElement[] {
  try {
    return [...document.querySelectorAll<HTMLElement>(selector)];
  } catch {
    return [];
  }
}

/// 刷新后重应用（preview 展区一次性渲染，延时一拍即可；面板另给「重新应用」手动入口）
export function applyStoredOverrides(overrides: InspectorOverrides): void {
  for (const [selector, props] of Object.entries(overrides)) {
    for (const el of safeQueryAll(selector)) {
      for (const [prop, value] of Object.entries(props)) el.style.setProperty(prop, value);
    }
  }
}

export function exportCss(overrides: InspectorOverrides): string {
  return (
    Object.entries(overrides)
      .map(([selector, props]) => {
        const lines = Object.entries(props).map(([prop, value]) => `  ${prop}: ${value};`);
        return `/* ${selector} */\n${selector} {\n${lines.join("\n")}\n}`;
      })
      .join("\n\n") + "\n"
  );
}

/// pick 模式：悬停高亮 + 标签跟随，点击选中（拦截原交互），Esc 退出。返回清理函数
export function startPick(onPick: (el: HTMLElement) => void): () => void {
  const overlay = document.createElement("div");
  overlay.style.cssText =
    "position:fixed;z-index:10000;pointer-events:none;outline:2px solid #37a5ff;background:rgba(15,137,236,0.10);display:none";
  const tip = document.createElement("div");
  tip.style.cssText =
    "position:fixed;z-index:10001;pointer-events:none;background:#1f1f1f;color:#cfcfcf;font:12px/16px monospace;padding:2px 6px;border-radius:4px;display:none";
  document.body.append(overlay, tip);

  function targetOf(e: MouseEvent): HTMLElement | null {
    const el = e.target instanceof HTMLElement ? e.target : null;
    if (!el) return null;
    if (el.closest(".dt-panel, .dt-fab, .dt-pickbar")) return null;
    return el;
  }
  function onMove(e: MouseEvent): void {
    const el = targetOf(e);
    if (!el) {
      overlay.style.display = "none";
      tip.style.display = "none";
      return;
    }
    const r = el.getBoundingClientRect();
    overlay.style.display = "block";
    overlay.style.left = `${r.left}px`;
    overlay.style.top = `${r.top}px`;
    overlay.style.width = `${r.width}px`;
    overlay.style.height = `${r.height}px`;
    tip.style.display = "block";
    tip.textContent = labelFor(el);
    tip.style.left = `${Math.min(r.left, window.innerWidth - 220)}px`;
    tip.style.top = `${Math.max(0, r.top - 20)}px`;
  }
  function onClick(e: MouseEvent): void {
    const el = targetOf(e);
    if (!el) return;
    e.preventDefault();
    e.stopPropagation();
    cleanup();
    onPick(el);
  }
  function onKey(e: KeyboardEvent): void {
    if (e.key === "Escape") cleanup();
  }
  function cleanup(): void {
    window.removeEventListener("mousemove", onMove, true);
    window.removeEventListener("click", onClick, true);
    window.removeEventListener("keydown", onKey, true);
    overlay.remove();
    tip.remove();
  }
  window.addEventListener("mousemove", onMove, true);
  window.addEventListener("click", onClick, true);
  window.addEventListener("keydown", onKey, true);
  return cleanup;
}
