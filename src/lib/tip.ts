// 工具 tip（02_通用组件/09_工具tip）：C5 / 圆角 6 / 1px C4-15 / 内边距 4 6 / 间距 6，
// 标题 副文本 C4 + 快捷键 副文本 C4-40。悬停 400ms 显示，跟随元素下方居中。
// 用法：v-tip="'菜单'" 或 v-tip="{ text: '新建', keys: 'Ctrl+N' }"
import type { Directive } from "vue";

export interface TipValue {
  text: string;
  keys?: string;
}

let tipEl: HTMLDivElement | null = null;
let showTimer: number | undefined;

function ensureEl(): HTMLDivElement {
  if (!tipEl) {
    tipEl = document.createElement("div");
    tipEl.className = "app-tip";
    document.body.appendChild(tipEl);
  }
  return tipEl;
}

function show(el: HTMLElement, value: TipValue): void {
  const tip = ensureEl();
  tip.replaceChildren();
  const span = document.createElement("span");
  span.textContent = value.text;
  tip.appendChild(span);
  if (value.keys) {
    const k = document.createElement("span");
    k.className = "app-tip-keys";
    k.textContent = value.keys;
    tip.appendChild(k);
  }
  const rect = el.getBoundingClientRect();
  tip.style.left = `${rect.left + rect.width / 2}px`;
  tip.classList.add("show");
  // 位置自适应：下方空间不足（会被窗口下缘截断）时翻到元素上方
  const tipH = tip.offsetHeight;
  let top = rect.bottom + 6;
  if (top + tipH > window.innerHeight - 4) {
    top = rect.top - tipH - 6;
  }
  tip.style.top = `${Math.max(4, top)}px`;
}

function hide(): void {
  ensureEl().classList.remove("show");
}

export const vTip: Directive<HTMLElement, string | TipValue | undefined> = {
  mounted(el, binding) {
    el.addEventListener("pointerenter", () => {
      if (!binding.value) return;
      showTimer = window.setTimeout(() => {
        const v = binding.value;
        show(el, typeof v === "string" ? { text: v } : v!);
      }, 400);
    });
    el.addEventListener("pointerleave", () => {
      clearTimeout(showTimer);
      hide();
    });
  },
  unmounted() {
    clearTimeout(showTimer);
    hide();
  },
};
