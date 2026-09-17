<script setup lang="ts">
// 编辑描述弹窗（31_编辑描述稿，6.1；文件与档案共用）：
// 壳 = 479 稿宽（482 真 border 补偿）/ 标题栏 54 / 按钮栏 62（主左次右 88×38）；
// item = 下移 + 上移动箭头（chevron-small，首行禁上 / 末行禁下——禁用色 --line-active，
// 覆盖 IconButton 提 specificity 照 BlockManageRow 范式）+ 标题输入框 + 红色删除（delete-red）；
// 内容行箭头与删除占位不可见（稿 opacity 0 对齐）；排序 = 箭头 + TransitionGroup 交换动画
//（2026-08-16 定案：弹窗 / 面板列表行不做拖拽，速查表 §9）；「添加描述」C4 钮追加空条。
// 确认 / 取消 = 草稿式：确认一次提交（一步 undo，双空条目后端规整丢弃），取消全部放弃。
import { computed, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppInput from "./AppInput.vue";
import IconButton from "./IconButton.vue";
import { useUiStore } from "../stores/ui";
import { setArchiveDescriptions, setAssetDescriptions, type DescItemInput } from "../lib/customDataOps";

const ui = useUiStore();
const target = computed(() => ui.descEdit);
const isFile = computed(() => target.value?.kind === "file");

// 草稿（初值 = 打开时快照；k = 本地行 key——TransitionGroup 交换动画需稳定 key，提交时剥除）
let rowSeq = 0;
type Row = DescItemInput & { k: number };
const rows = ref<Row[]>([...(target.value?.initial ?? [])].map((d) => ({ ...d, k: rowSeq++ })));
const busy = ref(false);

function moveRow(index: number, dir: -1 | 1): void {
  const to = index + dir;
  if (to < 0 || to >= rows.value.length) return;
  const [row] = rows.value.splice(index, 1);
  rows.value.splice(to, 0, row);
}
function removeRow(index: number): void {
  rows.value.splice(index, 1);
}

async function onConfirm(): Promise<void> {
  const t = target.value;
  if (!t || busy.value) return;
  busy.value = true;
  try {
    const items: DescItemInput[] = rows.value.map((r) => ({ title: r.title, content: r.content }));
    const ok = isFile.value ? await setAssetDescriptions(t.id, items) : await setArchiveDescriptions(t.id, items);
    if (ok) ui.descEdit = null;
  } finally {
    busy.value = false;
  }
}
function onCancel(): void {
  if (busy.value) return;
  ui.descEdit = null;
}
</script>

<template>
  <AppModal v-if="target" variant="standard" title="编辑描述" class="desc-dialog" @close="onCancel">
    <div class="dd-body">
      <!-- item 列表（TransitionGroup 交换动画，BlockManageDialog 同款） -->
      <TransitionGroup name="dd-swap" tag="div" class="dd-rows">
        <div v-for="(row, i) in rows" :key="row.k" class="dd-item">
          <!-- 标题行：下移 / 上移箭头 + 标题输入框 + 红色删除 -->
          <div class="dd-line">
            <IconButton
              name="chevron-small"
              small
              class="sort-arrow down"
              :disabled="i === rows.length - 1"
              title="下移"
              @click="moveRow(i, 1)"
            />
            <IconButton
              name="chevron-small"
              small
              class="sort-arrow up"
              :disabled="i === 0"
              title="上移"
              @click="moveRow(i, -1)"
            />
            <span class="dd-label">标题:</span>
            <AppInput v-model="row.title" size="small" class="dd-input" />
            <IconButton name="delete-red" delete-red title="删除" @click="removeRow(i)" />
          </div>
          <!-- 内容行：箭头 / 删除占位不可见（稿 opacity 0 对齐） -->
          <div class="dd-line">
            <span class="sort-arrow ghost" />
            <span class="sort-arrow ghost" />
            <span class="dd-label">内容:</span>
            <AppInput v-model="row.content" size="small" class="dd-input" />
            <span class="del-ghost" />
          </div>
        </div>
      </TransitionGroup>
      <!-- 添加描述（31 稿：C4 104×36 + 新增图标 18 + 14/20 常态） -->
      <div class="dd-add">
        <AppButton variant="panel" icon="add-large" label="添加描述" @click="rows.push({ title: '', content: '', k: rowSeq++ })" />
      </div>
    </div>
    <template #actions>
      <div class="dd-bottom">
        <AppButton variant="primary" label="确认" :disabled="busy" @click="onConfirm" />
        <AppButton variant="panel" label="取消" :disabled="busy" @click="onCancel" />
      </div>
    </template>
  </AppModal>
</template>

<style scoped>
/* 31 稿总容器：479 宽（482 真 border 补偿）；高度随内容（稿 360 为两条样例） */
.desc-dialog :deep(.app-modal) {
  width: 482px;
  padding: 0;
  gap: 0;
}
/* 标题栏（31 稿）：54 高 / padding 12 12 12 24 / 下分隔线 */
.desc-dialog :deep(.titlebar) {
  height: 54px;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  box-shadow: inset 0 -1px 0 0 var(--line-hover);
  flex: none;
}
/* 内容区（稿：padding 0 24）；
   壳 padding 0：归零 AppModal .content 的负 margin（2026-09-13 实机走查贴右缘口径）——
   padding-right 保留 24（条目靠它内缩）；.content 自身即滚动容器，
   AppModal 兜底 ScrollThumb 贴弹窗右缘 */
.desc-dialog :deep(.content) {
  flex: 1;
  min-height: 0;
  padding: 0 var(--space-24);
  gap: 0;
  overflow: auto;
  margin-right: 0;
}
.desc-dialog :deep(.actions) {
  flex: none;
}
.dd-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.dd-rows {
  display: flex;
  flex-direction: column;
  flex: none;
}
/* 排序交换动画（TransitionGroup move，BlockManageDialog 同款口径） */
.dd-swap-move {
  transition: transform 0.18s ease;
}
/* item（31 稿：padding 12 0 / gap 12 / 底部分隔线） */
.dd-item {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--space-12);
  padding: var(--space-12) 0;
  box-shadow: inset 0 -1px 0 0 var(--line-hover);
}
.dd-line {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  height: 28px;
}
/* 排序箭头（chevron-small 源图形朝上（实机渲染核对，BlockManageRow 同口径）——
   下移旋转 180、上移原样；可用 = 高亮档，禁用 = --line-active 取消压暗；
   选择器提权压 IconButton 自带 .icon-btn.small / :disabled（BlockManageRow 范式）） */
.dd-line .icon-btn.sort-arrow {
  color: var(--text-highlight);
}
.dd-line .icon-btn.sort-arrow:disabled {
  color: var(--line-active);
  opacity: 1; /* 覆盖 IconButton 禁用 0.5 压暗——禁用色原样呈现（30 稿口径） */
}
.sort-arrow.down :deep(svg) {
  transform: rotate(180deg);
}
/* 内容行占位（稿 opacity 0 对齐列） */
.sort-arrow.ghost {
  width: 20px;
  height: 20px;
  flex: none;
  visibility: hidden;
}
.del-ghost {
  width: 20px;
  height: 20px;
  flex: none;
  visibility: hidden;
}
/* 标签（稿：32 宽 14/20 标签色，冒号英文——速查表 §3 全库统一） */
.dd-label {
  width: 32px;
  flex: none;
  color: var(--text-label);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
/* 输入框（稿「输入框 - 小」：28 高 C1 底——AppInput small 尺寸，底色按稿覆盖 C2 → C1，提权压组件自带） */
.desc-dialog .dd-input :deep(.app-input-shell.small) {
  background: var(--c1);
}
/* 添加描述区（稿：padding 12 24 容器居中） */
.dd-add {
  display: flex;
  justify-content: center;
  padding: var(--space-12) 0;
  flex: none;
}
/* 底部按钮栏（62 高 / 上分隔线 / 主左次右 88×38，速查表 §1） */
.dd-bottom {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  height: 62px;
  box-shadow: inset 0 1px 0 0 var(--line-hover);
}
.dd-bottom :deep(.app-btn) {
  width: 88px;
}
</style>
