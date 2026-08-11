// 工具 tip（02_通用组件/09_工具tip）：C3 / 圆角 6 / 2px border-1 / 内边距 4 6 / 间距 6，
// 标题 副文本 font-icon-1 + 快捷键徽章组（同右键菜单 KbdBadge：按 "+" 拆分多枚，
// 词键小写 ctrl/shift/alt/enter/delete、单字母大写、中文键名原样，2026-08-11 用户定）。
// 悬停 400ms 显示，跟随元素下方居中。
// 用法：v-tip="'菜单'" 或 v-tip="{ text: '新建', keys: 'Ctrl+N' }"；
// column: true = 名称 / 简介两行竖排（gap 4，卡片名称 tip，2026-08-10 用户定）。
import type { Directive } from "vue";

export interface TipValue {
  text: string;
  keys?: string;
  column?: boolean; // true = 两行竖排（名称 / 简介）；默认横排（按钮 tip）
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
  tip.classList.toggle("col", value.column === true);
  tip.replaceChildren();
  const span = document.createElement("span");
  span.textContent = value.text;
  tip.appendChild(span);
  if (value.keys) {
    const group = document.createElement("span");
    group.className = "app-tip-keys";
    for (const raw of value.keys.split("+").filter(Boolean)) {
      const kbd = document.createElement("kbd");
      kbd.className = raw.length <= 1 ? "app-tip-kbd single" : "app-tip-kbd";
      // 显示口径同菜单（MenuItem）：词键小写、单字母大写、中文原样
      kbd.textContent = /^[A-Za-z]{2,}$/.test(raw) ? raw.toLowerCase() : raw;
      group.appendChild(kbd);
    }
    tip.appendChild(group);
  }
  const rect = el.getBoundingClientRect();
  tip.classList.add("show");
  // 水平钳制（2026-08-08 用户反馈：tip 不被窗口裁剪）——居中理想值超出窗口左右缘时收回界内
  const tipW = tip.offsetWidth;
  const half = tipW / 2;
  const cx = rect.left + rect.width / 2;
  tip.style.left = `${Math.min(Math.max(cx, 4 + half), window.innerWidth - 4 - half)}px`;
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
    // 点击即隐藏（打开菜单 / 弹窗后 tip 不应残留，2026-07-24 用户定）
    el.addEventListener("pointerdown", () => {
      clearTimeout(showTimer);
      hide();
    });
  },
  unmounted() {
    clearTimeout(showTimer);
    hide();
  },
};
