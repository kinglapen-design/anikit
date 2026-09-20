<script setup lang="ts">
// 区块管理弹窗（30 稿 + 26 稿删除区块警告，2026-08-14 区块子系统）：
// 2026-09-20 R2 旧件烧减：壳 = modals/BaseModal framed（559 宽挪 prop；标题栏 54 带底线 +
// 关闭 X 由 framed 提供）；按钮 = ActionButton（按钮栏 lg 宽自适应 + 左右 48、次左主右；
// 添加钮 solid md——旧 88 / 104 定宽皆废）；删除警告 = modals/ConfirmModal warning
//（旧 AppModal alert 退役；提示音沿——弹开即播）。
// 草稿式：打开时从 content store 快照区块列表进本地草稿，行编辑（改名 / 换图标 / 箭头排序 /
// 删除标记）只动草稿；确认 → apply_block_changes 一次提交（数组序 = 最终顺序）；
// needConfirm → 26 稿警告，确认后 confirmed=true 重提；
// 取消 / X = 丢弃草稿纯前端关闭。入口 = SidebarLeft section-manage（ui.blockManageOpen），宿主 App.vue。
import { nextTick, ref, watch } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ConfirmModal from "./modals/ConfirmModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import BlockManageRow from "./BlockManageRow.vue";
import { playAlert } from "../lib/sounds";
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

// 警告提示音（旧 AppModal alert onMounted 口径沿——弹开即播）
watch(conflicts, (v) => {
  if (v) playAlert();
});

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
  <BaseModal title="区块管理" :width="559" framed class="block-manage" @close="emit('close')">
    <div class="bm-body">
      <div class="scroll-wrap">
        <div ref="rowsEl" class="bm-rows kit-scroll">
          <!-- TransitionGroup：箭头排序时两行交换位置给位移过渡（2026-08-14 用户定：排序反馈要明显） -->
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
      <!-- 内容底部添加区块（30 稿：容器 60 高，按钮居中） -->
      <div class="add-wrap">
        <ActionButton variant="solid" size="md" icon="add-large" label="添加区块" class="add-btn" @click="addBlock" />
      </div>
    </div>
    <template #actions>
      <ActionButton variant="cancel" label="取消" size="lg" :disabled="busy" @click="emit('close')" />
      <ActionButton variant="primary" label="确认" size="lg" :disabled="busy" @click="submit(false)" />
    </template>
  </BaseModal>
  <!-- 删除非空区块警告（26 稿 → ConfirmModal warning + 提示音；确认「删除」 / 取消回编辑；
       busy 重入由 submit 内部闸挡——ConfirmModal 无 disabled 能力） -->
  <ConfirmModal
    v-if="conflicts"
    kind="warning"
    title="删除区块"
    confirm-label="删除"
    @confirm="submit(true)"
    @cancel="conflicts = null"
  >
    <p v-for="line in conflictLines()" :key="line" class="warn-line">{{ line }}</p>
    <p class="warn-line">删除后文件夹将移入文件夹区块。</p>
  </ConfirmModal>
</template>

<style scoped>
/* 30 稿壳：559 宽挪 width prop（旧 = 内容宽 556 + 真 border 3 补偿）；framed padding 0 贴边、
   gap 归零（标题栏 / 按钮栏贴边，同 BatchRenameDialog 口径）；视口封顶沿旧 AppModal 口径（BaseModal 不带） */
.block-manage :deep(.bm-modal) {
  max-height: calc(100vh - 96px);
  gap: 0;
}
/* 底部按钮栏（30 稿）：62 高 / padding 12 12 12 24 / 顶线通栏 / lg 宽自适应 + 左右 48、次左主右 */
.block-manage :deep(.bm-actions) {
  height: 62px;
  align-items: center;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-top: 1px solid var(--n-g5);
  flex: none;
}
.bm-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 滚动包裹层：relative 锚 ScrollThumb（悬浮滚轮悬停显隐，主窗口统一交互）；
   flex 列让行列表可收缩（视口封顶时自滚）。旧 .content padding + 负 margin 对随 AppModal
   退役废除——framed 壳 padding 0，包裹层天然通栏，thumb 贴弹窗缘 */
.scroll-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 行列表：超高自滚（4 行封顶，稿内容区 3 行量上放宽一行）；0 24 内缩自承（framed 壳 padding 0） */
.bm-rows {
  display: flex;
  flex-direction: column;
  min-height: 0;
  max-height: 208px;
  padding: 0 var(--n-space-24);
}
.bm-rows-inner {
  display: flex;
  flex-direction: column;
}
/* 排序交换动画：TransitionGroup move 钩子量位差补过渡 */
.bm-swap-move {
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
/* 添加区块容器（30 稿）：60 高 / padding 12 24 / 按钮居中（solid md，旧 104 定宽废——BaseModal 罩宽自适应 + 左右 48） */
.add-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 60px;
  padding: var(--n-space-12) var(--n-space-24);
  flex: none;
}
</style>
