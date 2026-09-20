<script setup lang="ts">
// 编辑描述弹窗（31_编辑描述稿，6.1；文件与档案共用）：
// 2026-09-20 R2 旧件烧减：壳 = modals/BaseModal framed（482 宽挪 prop；标题栏 54 带底线 +
// 关闭 X 由 framed 提供）；输入框 = inputs/TextInput sm；按钮 = buttons/ActionButton——
// 按钮栏 lg 宽自适应 + 左右 48、次左主右（旧 88 定宽与主左次右序废）；「添加描述」solid md。
// 排序箭头 = buttons/IconButton bare sm（chevron-small，禁用 = 新件自带 30% 透明度——
// 旧「--line-active 原样呈现」覆盖随旧件退役）；红色删除 delete-red 新件无此变体——
// 暂留根目录旧 IconButton（LegacyIconButton，R2 烧减遗留项）。
// 滚动自管（.dd-scroll + ScrollThumb——旧靠 AppModal .content 兜底滚动，BaseModal 无内建滚轮）。
// 内容行箭头与删除占位不可见（稿 opacity 0 对齐）；排序 = 箭头 + TransitionGroup 交换动画
//（2026-08-16 定案：弹窗 / 面板列表行不做拖拽，速查表 §9）；「添加描述」钮追加空条。
// 确认 / 取消 = 草稿式：确认一次提交（一步 undo，双空条目后端规整丢弃），取消全部放弃。
import { computed, ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import TextInput from "./inputs/TextInput.vue";
import IconButton from "./buttons/IconButton.vue";
import LegacyIconButton from "./IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
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

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const rowsEl = ref<HTMLElement>();

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
  <BaseModal v-if="target" title="编辑描述" :width="482" framed class="desc-dialog" @close="onCancel">
    <div class="dd-body">
      <div class="scroll-wrap">
        <div ref="rowsEl" class="dd-scroll kit-scroll">
          <!-- item 列表（TransitionGroup 交换动画，BlockManageDialog 同款） -->
          <TransitionGroup name="dd-swap" tag="div" class="dd-rows">
            <div v-for="(row, i) in rows" :key="row.k" class="dd-item">
              <!-- 标题行：下移 / 上移箭头 + 标题输入框 + 红色删除 -->
              <div class="dd-line">
                <IconButton
                  name="chevron-small"
                  variant="bare"
                  size="sm"
                  class="sort-arrow down"
                  :disabled="i === rows.length - 1"
                  title="下移"
                  @click="moveRow(i, 1)"
                />
                <IconButton
                  name="chevron-small"
                  variant="bare"
                  size="sm"
                  class="sort-arrow up"
                  :disabled="i === 0"
                  title="上移"
                  @click="moveRow(i, -1)"
                />
                <span class="dd-label">标题:</span>
                <TextInput v-model="row.title" size="sm" class="dd-input" />
                <LegacyIconButton name="delete-red" delete-red title="删除" @click="removeRow(i)" />
              </div>
              <!-- 内容行：箭头 / 删除占位不可见（稿 opacity 0 对齐） -->
              <div class="dd-line">
                <span class="sort-arrow ghost" />
                <span class="sort-arrow ghost" />
                <span class="dd-label">内容:</span>
                <TextInput v-model="row.content" size="sm" class="dd-input" />
                <span class="del-ghost" />
              </div>
            </div>
          </TransitionGroup>
          <!-- 添加描述（31 稿 C4 钮；换代 = ActionButton solid md + add-large 图标） -->
          <div class="dd-add">
            <ActionButton variant="solid" size="md" icon="add-large" label="添加描述" @click="rows.push({ title: '', content: '', k: rowSeq++ })" />
          </div>
        </div>
        <ScrollThumb :body="rowsEl" />
      </div>
    </div>
    <template #actions>
      <div class="dd-bottom">
        <ActionButton variant="cancel" label="取消" size="lg" :disabled="busy" @click="onCancel" />
        <ActionButton variant="primary" label="确认" size="lg" :disabled="busy" @click="onConfirm" />
      </div>
    </template>
  </BaseModal>
</template>

<style scoped>
/* 31 稿总容器：宽 482 挪 BaseModal width prop（旧 = 479 稿宽 + 真 border 3 补偿）；
   高度随内容（稿 360 为两条样例）+ 视口封顶（旧 AppModal max-height 口径沿，BaseModal 不带）；
   framed 壳 padding 0 天然贴边，gap 归零（相邻区紧贴：标题栏线 / 按钮栏线各自承担） */
.desc-dialog :deep(.bm-modal) {
  max-height: calc(100vh - 96px);
  gap: 0;
}
.desc-dialog :deep(.bm-actions) {
  flex: none;
}
.dd-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 滚动包裹层：relative 锚 ScrollThumb；flex 列让内容区可收缩（视口封顶时自滚） */
.scroll-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 内容滚动区（稿：padding 0 24 内缩自承——framed 壳 padding 0） */
.dd-scroll {
  flex: 1;
  min-height: 0;
  padding: 0 var(--n-space-24);
}
.dd-rows {
  display: flex;
  flex-direction: column;
  flex: none;
}
/* 排序交换动画（TransitionGroup move，BlockManageDialog 同款口径） */
.dd-swap-move {
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
/* item（31 稿：padding 12 0 / gap 12 / 底部分隔线） */
.dd-item {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
  box-shadow: inset 0 -1px 0 0 var(--n-g5);
}
.dd-line {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 28px;
}
/* 排序箭头（chevron-small 源图形朝上（实机渲染核对）——下移旋转 180、上移原样；
   可用态 = 亮色/一级（2026-08-14 用户定高亮档沿，选择器提权压组件自带 .ic-btn.bare）；
   禁用 = 新件自带 30% 透明度） */
.dd-line .ic-btn.sort-arrow {
  color: var(--n-l1);
}
.sort-arrow.down :deep(svg) {
  transform: rotate(180deg);
}
/* 内容行占位（稿 opacity 0 对齐列；bare sm 钮 14×14——占位同宽） */
.sort-arrow.ghost {
  width: 14px;
  height: 14px;
  flex: none;
  visibility: hidden;
}
/* 删除占位（delete-red 旧件 20×20） */
.del-ghost {
  width: 20px;
  height: 20px;
  flex: none;
  visibility: hidden;
}
/* 标签（稿：32 宽 标签色，冒号英文——速查表 §3 全库统一） */
.dd-label {
  width: 32px;
  flex: none;
  color: var(--n-l3);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.dd-input {
  flex: 1;
  min-width: 0;
}
/* 添加描述区（稿：padding 12 24 容器居中） */
.dd-add {
  display: flex;
  justify-content: center;
  padding: var(--n-space-12) 0;
  flex: none;
}
/* 底部按钮栏（62 高 / 上分隔线 / lg 宽自适应 + 左右 48、次左主右——2026-09-13 定稿口径，旧 88 定宽废） */
.dd-bottom {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: flex-end;
  gap: var(--n-space-12);
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  height: 62px;
  box-shadow: inset 0 1px 0 0 var(--n-g5);
}
</style>
