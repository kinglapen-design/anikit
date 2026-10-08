<script setup lang="ts">
// 分组规则编辑器（33 稿分组管理 / 34 稿新建分组共用）：
// 规则行 = C3 容器（min 44 高 圆角 6 + 暗-悬停边框，2026-08-19 复验补）：头段（删除× + 「规则:」+
// 规则下拉 96 +「范围:」，恒 28 居首行）+ 范围区（chips 多行向下排）；
// 范围区形态：ext / type / 日期 = chips 多选（选中 = 品牌色底；候选按当前文件夹实际存在动态出——33 稿注释），
// 标签 = 当前文件夹实际存在 tags chips（2026-09-17，词间 AND；空态一行提示），
// 名称 = 分词 chips 输入（TermChipsInput：回车/空格/+/失焦提交，词间 AND——2026-08-19 复验由单输入框改）；
// 底部「添加新规则」钮（C5 底 102×29）。标签冒号 = 英文「:」（2026-08-19 用户定，全库统一）。
// 单规则组互斥转移（PRD 02-04 条款 2，仅扩展名/类型/日期；名称/标签不互斥——2026-10-08 用户定）
// 由父级经 pickValue 事件处理（编辑器不感知其他组）。
import { computed, nextTick, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import AppMenu from "./AppMenu.vue";
import IconButton from "./buttons/IconButton.vue";
import MenuItem from "./MenuItem.vue";
import TermChipsInput from "./TermChipsInput.vue";
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
// 标签候选（2026-09-17）：当前文件夹实际存在的 tags 去重出 chips（33 稿注释口径）
const tagCandidates = computed(() =>
  [...new Set(props.items.flatMap((p) => p.tags))].sort((a, b) => a.localeCompare(b, "zh")),
);

// —— 规则下拉（picker + AppMenu，同 28 稿排序下拉形态） ——
const kindPicker = ref<{ rule: GroupRule; x: number; y: number } | null>(null);
const kindPickerTrigger = ref<HTMLElement | null>(null);
const rootEl = ref<HTMLElement>();
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
    // 选中「名称」= 自动聚焦分词输入框（2026-08-19 用户定）
    if (kind === "name") {
      const idx = rules.value.indexOf(rule);
      void nextTick(() => {
        rootEl.value
          ?.querySelectorAll(".rule-row")[idx]
          ?.querySelector<HTMLElement>(".term-input")
          ?.focus();
      });
    }
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

// —— 名称规则：分词 chips（TermChipsInput；词间 AND；名称/标签不参互斥——2026-10-08，重叠词靠命中词数权重归属） ——
function addTerm(rule: GroupRule, value: string): void {
  const v = value.trim();
  if (!v || rule.values.includes(v)) return;
  emit("pickValue", rule, v); // 父级先做互斥转移（从其他单规则组摘除）
  if (!rule.values.includes(v)) rule.values.push(v); // 转移后仍不在本组才补（防御）
}
function removeTerm(rule: GroupRule, value: string): void {
  const i = rule.values.indexOf(value);
  if (i >= 0) rule.values.splice(i, 1);
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
  <div ref="rootEl" class="rules-editor">
    <div v-for="(rule, i) in rules" :key="i" class="rule-row">
      <!-- 头段（× + 规则 + 下拉 + 范围）：恒 28 高居首行——chips 换行使行变高时标签保持首行垂直居中 -->
      <div class="row-head">
        <IconButton variant="bare" size="sm" name="clear" title="删除规则" class="rule-del" @click="removeRule(i)" />
        <span class="row-label">规则:</span>
        <button class="kind-pick" @click="openKindPicker(rule, $event)">
          <span class="val">{{ kindLabel(rule.kind) }}</span>
          <AppIcon name="tree-expand-arrow" :size="12" class="chev" />
        </button>
        <span class="row-label">范围:</span>
      </div>
      <!-- 范围区：chips 多行向下排 / 名称 = 分词 chips 输入 -->
      <div class="row-body">
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
        <!-- 标签（2026-09-17）：当前文件夹实际存在的 tags chips（多选，词间 AND） -->
        <div v-else-if="rule.kind === 'tags'" class="chips">
          <button
            v-for="t in tagCandidates"
            :key="t"
            class="chip"
            :class="{ on: rule.values.includes(t) }"
            @click="toggleValue(rule, t)"
          >{{ t }}</button>
          <span v-if="tagCandidates.length === 0" class="chips-empty">当前文件夹没有标签</span>
        </div>
        <!-- 名称：分词 chips（回车/空格/+/失焦提交，词间 AND） -->
        <TermChipsInput
          v-else
          :terms="rule.values"
          @add="addTerm(rule, $event)"
          @remove="removeTerm(rule, $event)"
        />
      </div>
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
  gap: var(--n-space-12);
}
/* 规则行（33 稿：C3 底 44 高 圆角 6 / padding 8 12 / gap 8）；
   2026-08-19 复验：补 暗-悬停边框（稿口径）；chips 换行时行高自适应（min-height），
   头段（row-head 28）居首行保持垂直居中——标签不随多行内容整体居中 */
.rule-row {
  display: flex;
  align-items: flex-start;
  gap: var(--n-space-8);
  min-height: 44px;
  box-sizing: border-box;
  padding: var(--n-space-8) var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g3);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  flex: none;
}
/* 头段（× 规则 下拉 范围）：恒 28 高与首行 chips 同高居中 */
.row-head {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 28px;
  flex: none;
}
/* 范围区：chips 多行向下排（flex-wrap），首行与头段同线 */
.row-body {
  display: flex;
  flex: 1;
  min-width: 0;
}
.row-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
  flex: none;
}
/* 规则下拉（33 稿：96×28，C4 底 / 1px 暗-悬停 / 圆角 6；文字左对齐通用规则） */
.kind-pick {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-4);
  width: 96px;
  height: 28px;
  box-sizing: border-box;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  color: var(--n-l2);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  flex: none;
  transition: background 0.12s ease;
}
.kind-pick:hover {
  background: var(--n-g5);
}
.kind-pick .val {
  flex: 1;
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.kind-pick .chev {
  color: var(--n-l3);
}
/* 范围 chips（33 稿：28 高 C4 底；选中 = 品牌色底 + 高亮文字） */
.chips {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex-wrap: wrap;
  min-width: 0;
}
.chip {
  display: inline-flex;
  align-items: center;
  height: 28px;
  box-sizing: border-box;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  color: var(--n-l2);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  white-space: nowrap;
  transition: background 0.12s ease;
}
.chip:hover {
  background: var(--n-g5);
}
.chip.on,
.chip.on:hover {
  background: var(--n-brand);
  box-shadow: inset 0 0 0 1px var(--n-l-a2);
  color: var(--n-l1);
}
/* 标签候选空态提示（2026-09-17：当前文件夹无 tags 时） */
.chips-empty {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
}
/* 添加新规则（33 稿：C5 底 / 1px 亮-选中 / 圆角 6 / padding 6 12 / gap 6 / 标签档） */
.add-rule {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--n-space-6);
  height: 29px;
  box-sizing: border-box;
  padding: var(--n-space-6) var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g5);
  box-shadow: inset 0 0 0 1px var(--n-l-a2);
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  align-self: flex-start;
  transition: background 0.12s ease;
}
.add-rule:hover {
  background: var(--n-g4);
}
</style>
