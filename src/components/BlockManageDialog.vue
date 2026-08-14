<script setup lang="ts">
// 区块管理弹窗（30 稿 + 26 稿删除区块警告，2026-08-14 区块子系统）：
// AppModal standard 壳 556 宽（+3 真 border 补偿 = 559），标题栏 54 / 内容区 padding 0 24 /
// 底部按钮栏 62 确认 primary · 取消 panel（88×38 主左次右）；标题常态色（AppModal 默认，勿覆盖）。
// 草稿式：打开时从 content store 快照区块列表进本地草稿，行编辑（改名 / 换图标 / 箭头排序 /
// 删除标记）只动草稿；确认 → apply_block_changes 一次提交（数组序 = 最终顺序）；
// needConfirm → 26 稿 alert warning（48 徽章 + 提示音），确认后 confirmed=true 重提；
// 取消 / X = 丢弃草稿纯前端关闭。入口 = SidebarLeft section-manage（ui.blockManageOpen），宿主 App.vue。
import { nextTick, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import BlockManageRow from "./BlockManageRow.vue";
import { applyBlockChanges, type BlockDeleteConflict, type BlockDraft, type BlockInput } from "../lib/blockOps";
import { DEFAULT_BLOCK_ID, useContentStore } from "../stores/content";

const emit = defineEmits<{ close: [] }>();
const content = useContentStore();

// 打开快照：store 区块表 → 本地草稿（v-if 宿主，setup 即"打开时"）
let seq = 0;
const draft = ref<BlockDraft[]>(
  content.blocks.map((b) => ({ key: b.id, id: b.id, name: b.name, icon: b.icon })),
);
// 删除标记（仅既有区块需要回传 id；新建行未入库直接出草稿即可）
const deletedIds: string[] = [];
// 删除非空区块的两段式确认（26 稿）：needConfirm 报告条目；非 null = 警告弹窗打开
const conflicts = ref<BlockDeleteConflict[] | null>(null);
const busy = ref(false);

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const rowsEl = ref<HTMLElement>();

// 添加区块：本地临时行（id = null）+ 进入名称输入态（聚焦新行输入框）
function addBlock(): void {
  draft.value.push({ key: `new-${++seq}`, id: null, name: "", icon: null });
  void nextTick(() => {
    const input = rowsEl.value?.querySelector<HTMLInputElement>(".bm-row:last-child input");
    input?.focus();
  });
}

function moveRow(index: number, dir: -1 | 1): void {
  const rows = draft.value;
  const j = index + dir;
  if (j < 0 || j >= rows.length) return;
  const [row] = rows.splice(index, 1);
  rows.splice(j, 0, row);
}

function removeRow(index: number): void {
  const row = draft.value[index];
  if (row.id === DEFAULT_BLOCK_ID) return; // 默认区块禁删（行内按钮已隐藏，双保险）
  if (row.id) deletedIds.push(row.id);
  draft.value.splice(index, 1);
}

// 确认提交：needConfirm → 弹 26 稿警告（不动数据）；成功 → 关闭弹窗（刷新 / toast 在 blockOps）
async function submit(confirmed: boolean): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const blocks: BlockInput[] = draft.value.map((r) => ({ id: r.id, name: r.name, icon: r.icon }));
    const report = await applyBlockChanges(blocks, deletedIds, confirmed);
    if (!report) return; // 失败 toast 已弹，弹窗保持打开
    if (report.needConfirm) {
      conflicts.value = report.blocks;
      return;
    }
    conflicts.value = null;
    emit("close");
  } finally {
    busy.value = false;
  }
}

// 警告弹窗文案（26 稿；删除口径 2026-08-14 用户定：文件夹带子树移入「目录」区块根级）
function conflictLines(): string[] {
  return (conflicts.value ?? []).map((c) => `区块「${c.name}」内还有 ${c.folderCount} 个文件夹`);
}
</script>

<template>
  <AppModal variant="standard" title="区块管理" class="block-manage" @close="emit('close')">
    <div class="bm-body">
      <div class="scroll-wrap">
        <div ref="rowsEl" class="bm-rows kit-scroll">
          <!-- TransitionGroup：箭头排序时两行交换位置给 0.18s 位移过渡（2026-08-14 用户定：排序反馈要明显） -->
          <TransitionGroup name="bm-swap" tag="div" class="bm-rows-inner">
            <BlockManageRow
              v-for="(row, i) in draft"
              :key="row.key"
              :row="row"
              :is-first="i === 0"
              :is-last="i === draft.length - 1"
              :is-default="row.id === DEFAULT_BLOCK_ID"
              @move-up="moveRow(i, -1)"
              @move-down="moveRow(i, 1)"
              @remove="removeRow(i)"
            />
          </TransitionGroup>
        </div>
        <ScrollThumb :body="rowsEl" />
      </div>
      <!-- 内容底部添加区块（30 稿：容器 60 高，按钮 C4 104×36 居中） -->
      <div class="add-wrap">
        <AppButton variant="panel" icon="add-large" label="添加区块" class="add-btn" @click="addBlock" />
      </div>
    </div>
    <template #actions>
      <AppButton variant="primary" label="确认" :disabled="busy" @click="submit(false)" />
      <AppButton variant="panel" label="取消" :disabled="busy" @click="emit('close')" />
    </template>
  </AppModal>
  <!-- 删除非空区块警告（26 稿：alert warning 48 徽章 + 提示音；主"删除"次"取消"） -->
  <AppModal
    v-if="conflicts"
    variant="alert"
    alert-icon="warning"
    title="删除区块"
    @close="conflicts = null"
  >
    <p v-for="line in conflictLines()" :key="line" class="warn-line">{{ line }}</p>
    <p class="warn-line">删除后文件夹将移入目录区块。</p>
    <template #actions>
      <AppButton variant="primary" label="删除" :disabled="busy" @click="submit(true)" />
      <AppButton variant="panel" label="取消" :disabled="busy" @click="conflicts = null" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 30 稿壳：内容宽 556 + 真 border 3 补偿 = 559，padding 0（标题栏 / 按钮栏贴边，同 DupScanDialog 口径） */
.block-manage :deep(.app-modal) {
  width: 559px;
  padding: 0;
  gap: 0;
}
/* 标题栏（30 稿）：54 高 / padding 12 12 12 24 / 底线通栏 */
.block-manage :deep(.titlebar) {
  height: 54px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-bottom: 1px solid var(--line-hover);
  flex: none;
}
/* 内容区（30 稿）：padding 0 24；滚动自管（ScrollThumb 悬浮件），壳 overflow 关闭 */
.block-manage :deep(.content) {
  padding: 0 var(--space-24);
  gap: 0;
  overflow: hidden;
}
/* 底部按钮栏（30 稿）：62 高 / padding 12 12 12 24 / 顶线通栏 / 按钮 88×38 主左次右 */
.block-manage :deep(.actions) {
  height: 62px;
  box-sizing: border-box;
  align-items: center;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-top: 1px solid var(--line-hover);
  flex: none;
}
.block-manage :deep(.actions) .app-btn {
  width: 88px;
}
.bm-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 滚动包裹层：relative 锚 ScrollThumb（悬浮滚轮悬停显隐，主窗口统一交互） */
.scroll-wrap {
  position: relative;
  min-height: 0;
}
/* 行列表：超高自滚（4 行封顶，稿内容区 3 行量上放宽一行） */
.bm-rows {
  display: flex;
  flex-direction: column;
  max-height: 208px;
}
.bm-rows-inner {
  display: flex;
  flex-direction: column;
}
/* 排序交换动画：TransitionGroup move 钩子量位差补过渡 */
.bm-swap-move {
  transition: transform 0.18s ease;
}
/* 添加区块容器（30 稿）：60 高 / padding 12 24 / 按钮居中 104×36 */
.add-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 60px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-24);
  flex: none;
}
.add-btn {
  width: 104px;
}
/* 警告弹窗文案：正文常态色，逐行竖排 */
.warn-line {
  margin: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
</style>
