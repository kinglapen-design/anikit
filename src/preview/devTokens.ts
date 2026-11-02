// dev token 调参（2026-11-02 用户定：preview 页像 opendesign 直接调间距等，免文字描述往返）。
// 数据源 = src/styles/tokens.css?raw 解析 :root 变量；改动实时写 :root inline 样式
//（同文档主 App 同愈），localStorage 持久化（dev 启动 applyDevTokenOverrides 恢复），
// 导出 = 覆盖 CSS / JSON，贴回 tokens.css 或给 AI 即定稿。仅 dev 链路动态 import，不进生产。

export interface DevToken {
  name: string;
  value: string;
  note: string;
  group: string;
}

export type DevTokenOverrides = Record<string, string>;

const STORAGE_KEY = "anikit.devTokenOverrides.v1";

/// 解析 tokens.css：变量行取 名称 / 值 / 行内注释；独立注释行记为最近分组名（含旧体系组，全量可调）
export function parseTokens(css: string): DevToken[] {
  const tokens: DevToken[] = [];
  let group = "";
  for (const line of css.split("\n")) {
    const decl = /^\s*(--[\w-]+)\s*:\s*([^;]+?)\s*;\s*(?:\/\*\s*(.*?)\s*\*\/)?\s*$/.exec(line);
    if (decl) {
      tokens.push({ name: decl[1], value: decl[2], note: decl[3] ?? "", group });
      continue;
    }
    const comment = /^\s*\/\*\s*(.*?)\s*\*\/\s*$/.exec(line);
    if (comment) {
      group = comment[1];
    }
  }
  return tokens;
}

export type DevTokenKind = "length" | "duration" | "number" | "color" | "text";

export function tokenKind(token: DevToken): DevTokenKind {
  const v = token.value.trim();
  if (/^-?\d+(?:\.\d+)?px$/.test(v)) return "length";
  if (/^-?\d+(?:\.\d+)?ms$/.test(v)) return "duration";
  if (/^-?\d+(?:\.\d+)?$/.test(v)) return "number";
  if (/^(#|hsla?\(|rgba?\()/.test(v)) return "color";
  return "text";
}

export function numericValue(token: DevToken): number {
  return Number.parseFloat(token.value) || 0;
}

export function loadOverrides(): DevTokenOverrides {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return Object.fromEntries(
        Object.entries(parsed).filter((e): e is [string, string] => typeof e[1] === "string"),
      );
    }
  } catch {
    // 损坏快照按空调参，不阻塞启动
  }
  return {};
}

function saveOverrides(overrides: DevTokenOverrides): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
  } catch {
    // 隐私模式等写不进去时仅本次会话生效
  }
}

export function applyOverrides(overrides: DevTokenOverrides): void {
  for (const [name, value] of Object.entries(overrides)) {
    document.documentElement.style.setProperty(name, value);
  }
}

/// main.ts dev 启动入口：localStorage 覆盖恢复到 :root（主 App 与 preview 页同吃）
export function applyDevTokenOverrides(): void {
  applyOverrides(loadOverrides());
}

export function setOverride(overrides: DevTokenOverrides, name: string, value: string): DevTokenOverrides {
  const next = { ...overrides, [name]: value };
  document.documentElement.style.setProperty(name, value);
  saveOverrides(next);
  return next;
}

export function clearOverride(overrides: DevTokenOverrides, name: string): DevTokenOverrides {
  const next = { ...overrides };
  delete next[name];
  document.documentElement.style.removeProperty(name);
  saveOverrides(next);
  return next;
}

export function clearAllOverrides(overrides: DevTokenOverrides): DevTokenOverrides {
  for (const name of Object.keys(overrides)) {
    document.documentElement.style.removeProperty(name);
  }
  saveOverrides({});
  return {};
}

/// 导出覆盖 CSS：按 tokens.css 原顺序，行内注释记原值，贴回 tokens.css 或发 AI 即定稿依据
export function exportCss(tokens: DevToken[], overrides: DevTokenOverrides): string {
  const lines = tokens
    .filter((t) => t.name in overrides)
    .map((t) => `  ${t.name}: ${overrides[t.name]}; /* 原 ${t.value} */`);
  return `:root {\n${lines.join("\n")}\n}\n`;
}
