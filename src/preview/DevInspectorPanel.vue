<script setup lang="ts">
// dev 组件检查器面板（2026-11-02 用户定「选一个组件弹面板调间距实时看效果」）：
// 选择组件 = pick 模式点击；属性 = 计算样式 + inline 覆盖实时生效；面包屑可上钻父级；
// 覆盖集 localStorage 持久化（重载重应用），导出选择器分块 CSS / JSON 供写回或发 AI。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  ancestorsOf,
  applyStoredOverrides,
  clearAllOverrides,
  clearElementOverrides,
  clearOverride,
  exportCss,
  INSPECT_GROUPS,
  labelFor,
  loadOverrides,
  readComputed,
  selectorFor,
  setOverride,
  startPick,
  type InspectPropDef,
  type InspectorOverrides,
} from "./devInspector";

const overrides = ref<InspectorOverrides>(loadOverrides());
const picked = ref<HTMLElement | null>(null);
const computedVals = ref<Record<string, string>>({});
const picking = ref(false);
const copied = ref<"" | "css" | "json" | "selector">("");
const exportText = ref("");
let stopPick: (() => void) | null = null;

onMounted(() => {
  // 刷新后重应用覆盖（preview 展区一次性渲染，一拍即可）
  setTimeout(() => applyStoredOverrides(overrides.value), 300);
});
onBeforeUnmount(() => stopPick?.());

const selector = computed(() => (picked.value ? selectorFor(picked.value) : ""));
const chain = computed(() => (picked.value ? [picked.value, ...ancestorsOf(picked.value)] : []));
const changedCount = computed(() => Object.values(overrides.value).reduce((n, p) => n + Object.keys(p).length, 0));
const currentOverrides = computed(() => (picked.value ? (overrides.value[selector.value] ?? {}) : {}));

function togglePick(): void {
  if (picking.value) {
    stopPick?.();
    picking.value = false;
    return;
  }
  picking.value = true;
  stopPick = startPick((el) => {
    picked.value = el;
    computedVals.value = readComputed(el);
    picking.value = false;
  });
}

function pickAncestor(el: HTMLElement): void {
  picked.value = el;
  computedVals.value = readComputed(el);
}

function currentValue(key: string): string {
  return currentOverrides.value[key] ?? computedVals.value[key] ?? "";
}

function isOverridden(key: string): boolean {
  return key in currentOverrides.value;
}

function numVal(def: InspectPropDef): number {
  return Number.parseFloat(currentValue(def.key)) || 0;
}

function rangeMax(def: InspectPropDef): number {
  return Math.max(48, Math.ceil(Math.abs(numVal(def)) * 3));
}

function setProp(def: InspectPropDef, e: Event): void {
  if (!picked.value) return;
  const raw = (e.target as HTMLInputElement).value.trim();
  if (!raw) return;
  const value = def.kind === "length" && /^-?\d+(\.\d+)?$/.test(raw) ? `${raw}px` : raw;
  overrides.value = setOverride(overrides.value, picked.value, def.key, value);
}

function resetProp(def: InspectPropDef): void {
  if (picked.value) overrides.value = clearOverride(overrides.value, picked.value, def.key);
}

function resetElement(): void {
  if (picked.value) overrides.value = clearElementOverrides(overrides.value, picked.value);
}

function resetAll(): void {
  overrides.value = clearAllOverrides(overrides.value);
  if (picked.value) computedVals.value = readComputed(picked.value);
}

function reapply(): void {
  applyStoredOverrides(overrides.value);
}

async function copyText(text: string, kind: "css" | "json" | "selector"): Promise<void> {
  exportText.value = "";
  try {
    await navigator.clipboard.writeText(text);
    copied.value = kind;
    setTimeout(() => {
      copied.value = "";
    }, 1500);
  } catch {
    exportText.value = text;
  }
}
</script>

<template>
  <div class="di-tools">
    <button class="dt-btn" :class="{ picking }" @click="togglePick">{{ picking ? "取消选择" : "选择组件" }}</button>
    <button class="dt-btn" :disabled="!changedCount" title="全部组件重置" @click="resetAll">↺</button>
    <button class="dt-btn" title="重新应用覆盖" @click="reapply">重应用</button>
    <span v-if="changedCount" class="di-count">{{ changedCount }} 处改动</span>
  </div>
  <div v-if="picking" class="dt-pickbar">点击页面中的组件选中 · Esc 退出</div>
  <template v-if="picked">
    <div class="di-chain">
      <template v-for="(el, i) in chain" :key="i">
        <button class="di-crumb" :class="{ cur: i === 0 }" @click="pickAncestor(el)">{{ labelFor(el) }}</button>
        <span v-if="i < chain.length - 1" class="di-sep">›</span>
      </template>
    </div>
    <div class="di-sel" :title="selector">
      <span class="di-sel-text">{{ selector }}</span>
      <button class="dt-reset" title="复制选择器" @click="copyText(selector, 'selector')">{{ copied === "selector" ? "✓" : "⧉" }}</button>
    </div>
    <div class="di-groups">
      <div v-for="g in INSPECT_GROUPS" :key="g.name" class="di-group">
        <div class="di-gname">{{ g.name }}</div>
        <div v-for="def in g.props" :key="def.key" class="di-row" :class="{ changed: isOverridden(def.key) }">
          <span class="di-label" :title="def.key">{{ def.label }}</span>
          <template v-if="def.kind === 'length'">
            <input
              class="di-range"
              type="range"
              :min="def.key.startsWith('margin') ? -48 : 0"
              :max="rangeMax(def)"
              :value="numVal(def)"
              @input="setProp(def, $event)"
            />
            <input class="dt-input di-num" type="number" :value="numVal(def)" @change="setProp(def, $event)" />
          </template>
          <select v-else-if="def.kind === 'select'" class="dt-input di-grow" :value="currentValue(def.key)" @change="setProp(def, $event)">
            <option v-for="opt in def.options" :key="opt" :value="opt">{{ opt }}</option>
          </select>
          <input v-else class="dt-input di-grow" :value="currentValue(def.key)" @change="setProp(def, $event)" />
          <button v-if="isOverridden(def.key)" class="dt-reset" title="重置" @click="resetProp(def)">↺</button>
        </div>
      </div>
    </div>
    <div class="dt-toolrow di-actions">
      <button class="dt-btn" @click="resetElement">重置此组件</button>
      <button class="dt-btn" :disabled="!changedCount" @click="copyText(exportCss(overrides), 'css')">
        {{ copied === "css" ? "已复制 ✓" : "复制覆盖 CSS" }}
      </button>
      <button class="dt-btn" :disabled="!changedCount" @click="copyText(JSON.stringify(overrides, null, 2), 'json')">
        {{ copied === "json" ? "已复制 ✓" : "复制 JSON" }}
      </button>
    </div>
  </template>
  <div v-else class="dt-empty">点「选择组件」再点页面里的组件，<br />间距 / 宽高 / 圆角 / 字号等实时可调</div>
  <div v-if="exportText" class="di-export">
    <div class="di-export-head">
      <span>剪贴板被拦，请全选手动复制（Ctrl+C）</span>
      <button class="dt-reset" @click="exportText = ''">×</button>
    </div>
    <textarea class="di-export-text" readonly :value="exportText" @focus="($event.target as HTMLTextAreaElement).select()"></textarea>
  </div>
</template>

<style scoped>
.di-tools {
  display: flex;
  align-items: center;
  gap: 8px;
}
.di-count {
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  white-space: nowrap;
}
.dt-pickbar {
  padding: 6px 8px;
  border-radius: var(--n-radius-4);
  background: var(--n-brand-50);
  color: var(--n-l1);
  font-size: var(--n-fs-secondary);
}
.picking {
  border-color: var(--n-brand);
  color: var(--n-brand-bright);
}
.di-chain {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}
.di-crumb {
  padding: 2px 6px;
  border: none;
  border-radius: var(--n-radius-4);
  background: none;
  color: var(--n-l3);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  cursor: pointer;
}
.di-crumb:hover {
  background: var(--n-l-a3);
  color: var(--n-l1);
}
.di-crumb.cur {
  color: var(--n-brand-bright);
}
.di-sep {
  color: var(--n-l4);
  font-size: var(--n-fs-secondary);
}
.di-sel {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-4);
  background: var(--n-g1);
}
.di-sel-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--n-l3);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
}
.di-groups {
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
}
.di-gname {
  color: var(--n-l2);
  font-size: var(--n-fs-body);
  font-weight: var(--n-fw-bold);
}
.di-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.di-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
}
.di-row.changed .di-label {
  color: var(--n-brand-bright);
}
.di-label {
  flex: none;
  width: 76px;
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.di-range {
  flex: 1;
  min-width: 0;
  accent-color: var(--n-brand);
}
.di-num {
  width: 56px;
  flex: none;
}
.di-grow {
  flex: 1;
  min-width: 0;
}
.di-actions {
  padding-top: 8px;
  border-top: 1px solid var(--n-g5);
}
.di-export {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--n-g5);
}
.di-export-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
}
.di-export-text {
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
