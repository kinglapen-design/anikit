// 悬停 tip（阶段七稿：docs/UI 重构/Design Token/组件/悬浮栏/悬停-tip + 悬停-卡片名称）：
// 壳 = 通用/半透明-1 + 1px 亮色/半透明-3 + 小阴影 + blur 小 + 圆角 6；标题 = 次要文字-加粗 二级；
// keys（快捷键 / 卡片简介副行）= 数字档 Fira Mono 三级 **原文直显**（kbd 徽章形态废除——稿为纯文字，
// 2026-08-29；右键菜单 KbdBadge 不受影响）。
// 悬停 400ms 显示，跟随元素下方居中（空间不足翻上方，水平钳制在窗口内）；点击即隐藏。
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
    const meta = document.createElement("span");
    meta.className = "app-tip-meta";
    meta.textContent = value.keys;
    tip.appendChild(meta);
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
