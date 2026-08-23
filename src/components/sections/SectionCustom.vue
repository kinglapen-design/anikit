<script setup lang="ts">
// 区块 · 自定义数据（2026-08-15 定案：带区块标题的普通区块——SectionHeader「自定义数据」
// 折叠开合与其他区块一致，无 + 钮；排序/显隐在 22 稿编辑面板与面板内拖拽）。
// 区块 gap 8（标题与内容间距，detail-block 公共规则）；内容容器 = 属性 chips 行 +
// 描述组 + 编辑描述按钮，容器内 gap 12。
// 6.1 编辑态落地（2026-08-23）：「添加属性」开 32 稿属性标签弹窗、「编辑描述」开 31 稿弹窗
//（文件与档案同做；ui store propsEdit / descEdit 快照目标，宿主 App.vue）；
// 文件描述源 = metadata.descriptions（原 note 冒充显示废除——note 字段保留，UI 不再当描述）；
// 描述内容超 3 行截断 +「展开 / 收起」标签色小字钮（PRD 03_数据链接 02，行数 2026-08-23 用户定）。
import { computed, nextTick, onMounted, ref, watch } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppTag from "../AppTag.vue";
import AppButton from "../AppButton.vue";
import IconButton from "../IconButton.vue";
import type { DetailTarget } from "../../lib/detailSections";
import { useUiStore } from "../../stores/ui";

const props = defineProps<{ target: DetailTarget; title: string }>();
const ui = useUiStore();
const open = ref(true);

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));
// 属性（文件 = tags 语义属性词；档案 = props）与描述（两侧统一标题 + 内容列表）
const propWords = computed(() => (file.value ? file.value.detail.tags : (archive.value?.data.props ?? [])));
const descriptions = computed(() =>
  file.value ? file.value.detail.descriptions : (archive.value?.data.descriptions ?? []),
);

// —— 描述截断（超 3 行出「展开 / 收起」，CSS line-clamp 3 行）——
const descEls = ref<HTMLElement[]>([]);
const needsClamp = ref<boolean[]>([]);
const expanded = ref<Set<number>>(new Set());
async function measure(): Promise<void> {
  await nextTick();
  needsClamp.value = descriptions.value.map((_, i) => {
    if (expanded.value.has(i)) return needsClamp.value[i] ?? false; // 展开态保持判定（展开后量不出截断）
    const el = descEls.value[i];
    return el ? el.scrollHeight > el.clientHeight + 1 : false;
  });
}
onMounted(measure);
watch(descriptions, () => {
  expanded.value = new Set();
  needsClamp.value = [];
  void measure();
});
function toggleDesc(i: number): void {
  const next = new Set(expanded.value);
  if (next.has(i)) next.delete(i);
  else next.add(i);
  expanded.value = next;
}

// —— 编辑入口（6.1）：快照目标 + 当前值，弹窗宿主 App.vue ——
function openPropsDialog(): void {
  const t = props.target;
  ui.propsEdit =
    t.kind === "file"
      ? { kind: "file", id: t.detail.assetId, name: t.placement.name, initial: [...t.detail.tags] }
      : { kind: "archive", id: t.node.id, name: t.node.name, initial: [...t.data.props] };
}
function openDescDialog(): void {
  const t = props.target;
  ui.descEdit =
    t.kind === "file"
      ? { kind: "file", id: t.detail.assetId, name: t.placement.name, initial: t.detail.descriptions.map((d) => ({ ...d })) }
      : { kind: "archive", id: t.node.id, name: t.node.name, initial: t.data.descriptions.map((d) => ({ ...d })) };
}
</script>

<template>
  <section class="section-custom detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <!-- 内容容器：gap 12（2026-08-15 定案） -->
    <div v-if="open" class="custom-body">
      <!-- 属性标签行（稿：gap 6；chip 高 25 / C4 / padding 4 8 / 圆角 6 / 12/17 标签色）
           + 行尾添加钮（6.1 点亮：开 32 稿属性标签弹窗） -->
      <div class="chips">
        <AppTag v-for="t in propWords" :key="t" :label="t" readonly />
        <IconButton name="add-small" small title="添加属性" class="chip-add" @click="openPropsDialog" />
      </div>
      <!-- 描述组（稿：纯文本行 gap 8；行 = 标签: 标签色 + 内容 常态，行内 gap 4；
           内容超 3 行截断 + 展开/收起标签色小字钮，PRD 03_数据链接 02） -->
      <div v-if="descriptions.length > 0" class="desc-group">
        <div v-for="(d, i) in descriptions" :key="i" class="desc-item">
          <div class="desc-row">
            <span v-if="d.title" class="d-label">{{ d.title }}:</span>
            <span
              :ref="(el) => { if (el) descEls[i] = el as HTMLElement; }"
              class="d-content"
              :class="{ clamp: !expanded.has(i) }"
            >{{ d.content }}</span>
          </div>
          <button v-if="needsClamp[i]" class="desc-toggle" @click="toggleDesc(i)">
            {{ expanded.has(i) ? "收起" : "展开" }}
          </button>
        </div>
      </div>
      <!-- 编辑描述按钮（稿：通栏 36 高 / C5 / 圆角 6 / 图标 18 + 14/20 常态；6.1 点亮开 31 稿弹窗） -->
      <AppButton class="edit-desc" variant="sidebar" icon="edit-description" label="编辑描述" @click="openDescDialog" />
    </div>
  </section>
</template>

<style scoped>
.section-custom {
  display: flex;
  flex-direction: column;
}
/* 内容容器：chips + 描述组 + 编辑描述，gap 12（2026-08-15 定案） */
.custom-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
}
/* 属性标签行（稿）：gap 6；chip 文字 12/17 标签色（AppTag 默认常态色，本区域按稿覆盖）
   + 行尾添加钮 20×20 */
.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-6);
}
.chips .app-tag {
  color: var(--text-label);
}
/* 描述组（稿）：纯文本行 gap 8；行 = 标签: 标签色 + 内容 常态，行内 gap 4 */
.desc-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
.desc-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.desc-row {
  display: flex;
  gap: var(--space-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.d-label {
  color: var(--text-label);
  white-space: nowrap;
  flex: none;
}
.d-content {
  color: var(--text-normal);
  min-width: 0;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
/* 3 行截断（PRD：内容超过设定行数显示展开/收起；行数 = 3，2026-08-23 用户定） */
.d-content.clamp {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
  overflow: hidden;
}
/* 展开 / 收起钮（2026-08-23 用户定：标签色小字钮，hover 提亮常态色） */
.desc-toggle {
  align-self: flex-start;
  color: var(--text-label);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  transition: color 0.15s ease;
}
.desc-toggle:hover {
  color: var(--text-normal);
}
/* 编辑描述按钮（稿）：通栏 36 高 / C5（AppButton sidebar）/ 圆角 6 / 图标 18 + 14/20 常态 */
.section-custom .edit-desc {
  width: 100%;
  height: 36px;
}
</style>
