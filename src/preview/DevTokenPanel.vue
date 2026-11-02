<script setup lang="ts">
// dev token 调参面板（2026-11-02 用户定）：tokens.css 全变量可视化编辑——
// 长度 / 时长给滑杆 + 数字，颜色给色块 + 文本（#hex 加取色器），其余文本；
// 改动实时写 :root（preview 与主 App 同文档同愈）、localStorage 持久化（dev 启动恢复），
// 导出覆盖 CSS / JSON 贴回 tokens.css 或发 AI 定稿。仅 dev 经 DesignSystemPreview 挂载。
import { computed, ref } from "vue";
import tokensCss from "../styles/tokens.css?raw";
import {
  applyDevTokenOverrides,
  clearAllOverrides,
  clearOverride,
  exportCss,
  loadOverrides,
  numericValue,
  parseTokens,
  setOverride,
  tokenKind,
  type DevToken,
  type DevTokenOverrides,
} from "./devTokens";

const tokens = parseTokens(tokensCss);
// 挂载即恢复（主 App 经 main.ts 恢复；preview 独立进入时这里兜底）
applyDevTokenOverrides();

const overrides = ref<DevTokenOverrides>(loadOverrides());
const open = ref(false);
const query = ref("");
const groupFilter = ref("");
const onlyChanged = ref(false);
const copied = ref<"" | "css" | "json">("");
const exportText = ref("");

const groups = computed(() => [...new Set(tokens.map((t) => t.group).filter(Boolean))]);
const changedCount = computed(() => Object.keys(overrides.value).length);
const visible = computed(() => {
  const q = query.value.trim().toLowerCase();
  return tokens.filter((t) => {
    if (groupFilter.value && t.group !== groupFilter.value) return false;
    if (onlyChanged.value && !(t.name in overrides.value)) return false;
    if (q && !`${t.name} ${t.note}`.toLowerCase().includes(q)) return false;
    return true;
  });
});

function current(token: DevToken): string {
  return overrides.value[token.name] ?? token.value;
}

function isHex(token: DevToken): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(current(token));
}

function rangeOf(token: DevToken): { min: number; max: number; step: number } {
  const n = Math.abs(numericValue(token));
  if (tokenKind(token) === "duration") return { min: 0, max: Math.max(1000, Math.ceil((n * 3) / 100) * 100), step: 10 };
  return { min: 0, max: Math.max(48, Math.ceil(n * 3)), step: 1 };
}

function unitOf(token: DevToken): string {
  return tokenKind(token) === "duration" ? "ms" : "px";
}

function onNumber(token: DevToken, event: Event): void {
  const raw = (event.target as HTMLInputElement).value;
  if (raw.trim() === "") return;
  const kind = tokenKind(token);
  const suffix = kind === "length" ? "px" : kind === "duration" ? "ms" : "";
  overrides.value = setOverride(overrides.value, token.name, `${raw}${suffix}`);
}

function onText(token: DevToken, event: Event): void {
  const raw = (event.target as HTMLInputElement).value;
  if (raw.trim() === "") return;
  overrides.value = setOverride(overrides.value, token.name, raw.trim());
}

function reset(name: string): void {
  overrides.value = clearOverride(overrides.value, name);
}

function resetAll(): void {
  overrides.value = clearAllOverrides(overrides.value);
}

async function copy(kind: "css" | "json"): Promise<void> {
  const text = kind === "css" ? exportCss(tokens, overrides.value) : JSON.stringify(overrides.value, null, 2);
  exportText.value = "";
  try {
    await navigator.clipboard.writeText(text);
    copied.value = kind;
    setTimeout(() => {
      copied.value = "";
    }, 1500);
  } catch {
    // 剪贴板权限被拦（后台 tab / 非聚焦窗口等）：展示原文供手动复制，不丢产物
    exportText.value = text;
  }
}
</script>

<template>
  <button class="dt-fab" :class="{ changed: changedCount > 0 }" @click="open = !open">
    调参<span v-if="changedCount" class="dt-fab-count">{{ changedCount }}</span>
  </button>
  <Transition name="dt-slide">
    <aside v-if="open" class="dt-panel">
      <header class="dt-head">
        <div class="dt-title">
          Token 调参
          <span class="dt-sub">实时生效 · 主界面同吃 · 仅 dev</span>
        </div>
        <button class="dt-close" title="关闭" @click="open = false">×</button>
      </header>
      <div class="dt-tools">
        <input v-model="query" class="dt-input" placeholder="搜名称 / 注释…" />
        <div class="dt-toolrow">
          <select v-model="groupFilter" class="dt-input dt-select">
            <option value="">全部分组</option>
            <option v-for="g in groups" :key="g" :value="g">{{ g }}</option>
          </select>
          <label class="dt-check"><input v-model="onlyChanged" type="checkbox" />仅已改</label>
        </div>
        <div class="dt-toolrow">
          <button class="dt-btn" :disabled="!changedCount" @click="copy('css')">
            {{ copied === "css" ? "已复制 ✓" : "复制覆盖 CSS" }}
          </button>
          <button class="dt-btn" :disabled="!changedCount" @click="copy('json')">
            {{ copied === "json" ? "已复制 ✓" : "复制 JSON" }}
          </button>
          <button class="dt-btn dt-reset-all" :disabled="!changedCount" title="全部重置" @click="resetAll">↺</button>
        </div>
      </div>
      <div class="dt-list">
        <div v-for="t in visible" :key="t.name" class="dt-row" :class="{ changed: t.name in overrides }">
          <div class="dt-rowhead">
            <span class="dt-name" :title="t.group">{{ t.name }}</span>
            <button v-if="t.name in overrides" class="dt-reset" title="重置为原值" @click="reset(t.name)">↺</button>
          </div>
          <div v-if="t.note" class="dt-note">{{ t.note }}</div>
          <div class="dt-editor">
            <template v-if="tokenKind(t) === 'length' || tokenKind(t) === 'duration'">
              <input
                class="dt-range"
                type="range"
                :min="rangeOf(t).min"
                :max="rangeOf(t).max"
                :step="rangeOf(t).step"
                :value="numericValue({ ...t, value: current(t) })"
                @input="onNumber(t, $event)"
              />
              <input
                class="dt-input dt-num"
                type="number"
                :value="numericValue({ ...t, value: current(t) })"
                @change="onNumber(t, $event)"
              />
              <span class="dt-unit">{{ unitOf(t) }}</span>
            </template>
            <template v-else-if="tokenKind(t) === 'number'">
              <input class="dt-input dt-num" type="number" :value="current(t)" @change="onNumber(t, $event)" />
            </template>
            <template v-else-if="tokenKind(t) === 'color'">
              <span class="dt-swatch" :style="{ background: current(t) }"></span>
              <input v-if="isHex(t)" class="dt-color" type="color" :value="current(t)" @input="onText(t, $event)" />
              <input class="dt-input dt-grow" :value="current(t)" @change="onText(t, $event)" />
            </template>
            <template v-else>
              <input class="dt-input dt-grow" :value="current(t)" @change="onText(t, $event)" />
            </template>
          </div>
          <div v-if="t.name in overrides" class="dt-origin">原 {{ t.value }}</div>
        </div>
        <div v-if="!visible.length" class="dt-empty">无匹配 token</div>
      </div>
      <div v-if="exportText" class="dt-export">
        <div class="dt-export-head">
          <span>剪贴板被拦，请全选手动复制（Ctrl+C）</span>
          <button class="dt-close" title="关闭" @click="exportText = ''">×</button>
        </div>
        <textarea class="dt-export-text" readonly :value="exportText" @focus="($event.target as HTMLTextAreaElement).select()"></textarea>
      </div>
    </aside>
  </Transition>
</template>

<style scoped>
.dt-fab {
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 9998;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-pill);
  background: var(--n-g3);
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
  cursor: pointer;
  box-shadow: var(--n-shadow-sm);
}
.dt-fab:hover {
  color: var(--n-l1);
  background: var(--n-g4);
}
.dt-fab-count {
  padding: 0 6px;
  border-radius: var(--n-radius-pill);
  background: var(--n-brand);
  color: var(--n-white);
  font-size: var(--n-fs-num);
}
.dt-panel {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  width: 340px;
  background: var(--n-g3);
  border-left: 1px solid var(--n-g5);
  box-shadow: var(--n-shadow-lg);
}
.dt-slide-enter-active,
.dt-slide-leave-active {
  transition: transform 150ms var(--n-ease-out);
}
.dt-slide-enter-from,
.dt-slide-leave-to {
  transform: translateX(100%);
}
.dt-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  border-bottom: 1px solid var(--n-g5);
}
.dt-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
  color: var(--n-l1);
  font-size: var(--n-fs-body);
  font-weight: var(--n-fw-bold);
}
.dt-sub {
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  font-weight: var(--n-fw-regular);
}
.dt-close {
  padding: 0 6px;
  border: none;
  background: none;
  color: var(--n-l3);
  font-size: 18px;
  cursor: pointer;
}
.dt-close:hover {
  color: var(--n-l1);
}
.dt-tools {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border-bottom: 1px solid var(--n-g5);
}
.dt-toolrow {
  display: flex;
  align-items: center;
  gap: 8px;
}
.dt-input {
  min-width: 0;
  padding: 4px 8px;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-4);
  background: var(--n-g1);
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
}
.dt-input:focus {
  outline: none;
  border-color: var(--n-brand);
}
.dt-select {
  flex: 1;
}
.dt-check {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  white-space: nowrap;
}
.dt-btn {
  flex: 1;
  padding: 4px 8px;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-4);
  background: var(--n-g4);
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
  cursor: pointer;
  white-space: nowrap;
}
.dt-btn:hover:not(:disabled) {
  background: var(--n-g5);
  color: var(--n-l1);
}
.dt-btn:disabled {
  color: var(--n-l-disabled);
  cursor: default;
}
.dt-reset-all {
  flex: none;
}
.dt-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}
.dt-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--n-g4);
}
.dt-row.changed {
  box-shadow: inset 2px 0 0 var(--n-brand);
}
.dt-rowhead {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.dt-name {
  color: var(--n-l2);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dt-row.changed .dt-name {
  color: var(--n-brand-bright);
}
.dt-note {
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
}
.dt-editor {
  display: flex;
  align-items: center;
  gap: 8px;
}
.dt-range {
  flex: 1;
  min-width: 0;
  accent-color: var(--n-brand);
}
.dt-num {
  width: 64px;
  flex: none;
}
.dt-unit {
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
}
.dt-grow {
  flex: 1;
}
.dt-swatch {
  width: 20px;
  height: 20px;
  flex: none;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-4);
}
.dt-color {
  width: 28px;
  height: 24px;
  flex: none;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
}
.dt-reset {
  padding: 0 4px;
  border: none;
  background: none;
  color: var(--n-l3);
  cursor: pointer;
}
.dt-reset:hover {
  color: var(--n-brand-bright);
}
.dt-origin {
  color: var(--n-l4);
  font-size: var(--n-fs-secondary);
}
.dt-empty {
  padding: 24px 12px;
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  text-align: center;
}
.dt-export {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border-top: 1px solid var(--n-g5);
}
.dt-export-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
}
.dt-export-text {
  height: 120px;
  resize: vertical;
  padding: 8px;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-4);
  background: var(--n-g1);
  color: var(--n-l2);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
}
</style>
