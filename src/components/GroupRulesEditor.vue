<script setup lang="ts">
// 分组规则编辑器（33 稿分组管理 / 34 稿新建分组共用）：
// 规则行 = C3 容器 44 高圆角 6（删除× + 「规则：」+ 规则下拉 96 + 「范围：」+ 范围区）；
// 范围区形态：ext / type / 日期 = chips 多选（选中 = 品牌色底；候选按当前文件夹实际存在动态出——33 稿注释），
// 名称 = AppInput small（空格分隔分词，词间 AND）；
// 底部「添加新规则」钮（C5 底 102×29）。
// 单规则组互斥转移（PRD 02-04 条款 2）由父级经 pickValue 事件处理（编辑器不感知其他组）。
import { computed, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import AppInput from "./AppInput.vue";
import AppMenu from "./AppMenu.vue";
import IconButton from "./IconButton.vue";
import MenuItem from "./MenuItem.vue";
import {
  DATE_BUCKET_LABELS,
  DATE_BUCKET_ORDER,
  GROUP_TYPE_LABELS,
  GROUP_TYPE_ORDER,
  RULE_KIND_LABELS,
  RULE_KIND_ORDER,
  groupTypeOf,
  type GroupRule,
  type GroupRuleKind,
  type GroupType,
} from "../lib/grouping";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ items: PlacementView[] }>();
const rules = defineModel<GroupRule[]>({ required: true });
const emit = defineEmits<{ pickValue: [rule: GroupRule, value: string] }>();

// —— 候选（33 稿注释：除名称外按当前文件夹实际存在出选项） ——
const extCandidates = computed(() =>
  [...new Set(props.items.map((p) => p.ext.toLowerCase()).filter(Boolean))].sort((a, b) => a.localeCompare(b, "en")),
);
const typeCandidates = computed(() => {
  const present = new Set(props.items.map((p) => groupTypeOf(p)));
  return GROUP_TYPE_ORDER.filter((t) => present.has(t));
});

// —— 规则下拉（picker + AppMenu，同 28 稿排序下拉形态） ——
const kindPicker = ref<{ rule: GroupRule; x: number; y: number } | null>(null);
const kindPickerTrigger = ref<HTMLElement | null>(null);
function openKindPicker(rule: GroupRule, e: MouseEvent): void {
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  kindPickerTrigger.value = el;
  kindPicker.value = { rule, x: rect.left, y: rect.bottom + 4 };
}
function pickKind(kind: GroupRuleKind): void {
  const rule = kindPicker.value?.rule;
  if (rule && rule.kind !== kind) {
    rule.kind = kind;
    rule.values = []; // 换规则类型清空范围（值域不同）
  }
  kindPicker.value = null;
}

// —— 范围 chips（多选；勾选即 emit 供父级做互斥转移） ——
function toggleValue(rule: GroupRule, value: string): void {
  const i = rule.values.indexOf(value);
  if (i >= 0) {
    rule.values.splice(i, 1);
  } else {
    emit("pickValue", rule, value); // 父级先做互斥转移（从其他单规则组摘除）
    if (!rule.values.includes(value)) rule.values.push(value); // 转移后仍不在本组才补（防御）
  }
}

// —— 名称规则：分词输入（空格分隔，词间 AND；PRD 02-04） ——
function nameInput(rule: GroupRule): string {
  return rule.values.join(" ");
}
function setNameInput(rule: GroupRule, text: string): void {
  rule.values = text.split(/\s+/).filter(Boolean);
}

function addRule(): void {
  rules.value.push({ kind: "ext", values: [] });
}
function removeRule(i: number): void {
  rules.value.splice(i, 1);
}
const kindLabel = (kind: GroupRuleKind): string => RULE_KIND_LABELS[kind];
const typeLabel = (t: string): string => GROUP_TYPE_LABELS[t as GroupType] ?? t;
</script>

<template>
  <div class="rules-editor">
    <div v-for="(rule, i) in rules" :key="i" class="rule-row">
      <IconButton small name="clear" title="删除规则" class="rule-del" @click="removeRule(i)" />
      <span class="row-label">规则：</span>
      <button class="kind-pick" @click="openKindPicker(rule, $event)">
        <span class="val">{{ kindLabel(rule.kind) }}</span>
        <AppIcon name="tree-expand-arrow" :size="12" class="chev" />
      </button>
      <span class="row-label">范围：</span>
      <!-- 扩展名：当前文件夹实际存在的扩展名 chips -->
      <div v-if="rule.kind === 'ext'" class="chips">
        <button
          v-for="ext in extCandidates"
          :key="ext"
          class="chip"
          :class="{ on: rule.values.includes(ext) }"
          @click="toggleValue(rule, ext)"
        >{{ ext }}</button>
      </div>
      <!-- 类型：8 类按实际存在 -->
      <div v-else-if="rule.kind === 'type'" class="chips">
        <button
          v-for="t in typeCandidates"
          :key="t"
          class="chip"
          :class="{ on: rule.values.includes(t) }"
          @click="toggleValue(rule, t)"
        >{{ typeLabel(t) }}</button>
      </div>
      <!-- 日期：六档（档间对文件互斥，多档 = OR） -->
      <div v-else-if="rule.kind === 'importDate' || rule.kind === 'modifyDate'" class="chips">
        <button
          v-for="d in DATE_BUCKET_ORDER"
          :key="d"
          class="chip"
          :class="{ on: rule.values.includes(d) }"
          @click="toggleValue(rule, d)"
        >{{ DATE_BUCKET_LABELS[d] }}</button>
      </div>
      <!-- 名称：分词输入（空格分隔，词间 AND） -->
      <AppInput
        v-else
        size="small"
        class="name-input"
        placeholder="输入关键词，空格分隔（全部命中才归组）"
        :model-value="nameInput(rule)"
        @update:model-value="setNameInput(rule, $event)"
      />
    </div>
    <!-- 添加新规则（33/34 稿：C5 底 102×29，add 图标 12 + 标签档文字） -->
    <button class="add-rule" @click="addRule">
      <AppIcon name="add-large" :size="12" />
      <span>添加新规则</span>
    </button>

    <!-- 规则类型下拉菜单 -->
    <AppMenu v-if="kindPicker" :x="kindPicker.x" :y="kindPicker.y" :trigger="kindPickerTrigger" @close="kindPicker = null">
      <MenuItem
        v-for="k in RULE_KIND_ORDER"
        :key="k"
        :label="RULE_KIND_LABELS[k]"
        checkable
        :checked="kindPicker.rule.kind === k"
        @select="pickKind(k)"
      />
    </AppMenu>
  </div>
</template>

<style scoped>
.rules-editor {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
}
/* 规则行（33 稿：C3 底 44 高 圆角 6 / padding 8 12 / gap 8） */
.rule-row {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  height: 44px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c3);
  flex: none;
}
.row-label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
  white-space: nowrap;
  flex: none;
}
/* 规则下拉（33 稿：96×28，C4 底 / 1px 暗-悬停 / 圆角 6；文字左对齐通用规则） */
.kind-pick {
  display: inline-flex;
  align-items: center;
  gap: var(--space-4);
  width: 96px;
  height: 28px;
  box-sizing: border-box;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
  transition: background 0.12s ease;
}
.kind-pick:hover {
  background: var(--c5);
}
.kind-pick .val {
  flex: 1;
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.kind-pick .chev {
  color: var(--text-label);
}
/* 范围 chips（33 稿：28 高 C4 底；选中 = 品牌色底 + 高亮文字） */
.chips {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex-wrap: wrap;
  min-width: 0;
}
.chip {
  display: inline-flex;
  align-items: center;
  height: 28px;
  box-sizing: border-box;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  transition: background 0.12s ease;
}
.chip:hover {
  background: var(--c5);
}
.chip.on,
.chip.on:hover {
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-highlight);
}
.name-input {
  flex: 1;
  min-width: 0;
  /* 弹窗内输入框 token = C1（30 稿口径；AppInput 壳化后 class 落壳——底色在壳上覆盖） */
  background: var(--c1);
}
/* 添加新规则（33 稿：C5 底 / 1px 亮-选中 / 圆角 6 / padding 6 12 / gap 6 / 标签档） */
.add-rule {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-6);
  height: 29px;
  box-sizing: border-box;
  padding: var(--space-6) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c5);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-normal);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  align-self: flex-start;
  transition: background 0.12s ease;
}
.add-rule:hover {
  background: var(--c4);
}
</style>
