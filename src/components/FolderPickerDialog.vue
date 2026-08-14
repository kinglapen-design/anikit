<script setup lang="ts">
// 文件夹选择弹窗（6.3"添加至文件夹"）：文件夹树单选目标 → 确认回调（关联 / 副本弹窗前置）。
// 顶部带"未分类"根级项（2026-08-13 用户定案：落点不设限，原"候车区禁止链接落点"废除）。
// 多区块（2026-08-14 区块子系统）：顶级文件夹按区块分段，区块标题行不可选
//（简化标题行 = 05 稿区块标题文字样式，无加号无折叠）；单默认区块时不渲染标题行（与原布局一致）。
// 由 selection.folderPickPrompt 驱动，宿主 App.vue；展开态本组件自管（默认全展开）。
import { computed, ref } from "vue";
import IconButton from "./IconButton.vue";
import TreeItem from "./TreeItem.vue";
import FolderPickNode from "./FolderPickNode.vue";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const content = useContentStore();
const selection = useSelectionStore();
const pickedId = ref<string | null>(null);

const collapsed = ref(new Set<string>());
function toggle(id: string): void {
  const next = new Set(collapsed.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  collapsed.value = next;
}

const prompt = computed(() => selection.folderPickPrompt);
// 多区块才分段渲染标题行（2026-08-14）：单默认区块 = 原布局零变化
const showBlockHeaders = computed(() => content.blockSections.length > 1);

function onConfirm(): void {
  const p = prompt.value;
  selection.folderPickPrompt = null;
  if (pickedId.value) p?.onPick(pickedId.value);
}
function onCancel(): void {
  selection.folderPickPrompt = null;
}
</script>

<template>
  <div v-if="prompt" class="pick-mask">
    <div class="pick-card">
      <header class="pick-titlebar">
        <h2 class="pick-title">添加至文件夹</h2>
        <IconButton name="window-close" title="关闭" @click="onCancel" />
      </header>
      <div class="pick-body">
        <!-- "未分类"根级项（2026-08-13：落点不设限，链接允许落在未分类） -->
        <TreeItem
          icon="folder-closed"
          label="未分类"
          :depth="0"
          :selected="pickedId === 'root'"
          @select="pickedId = 'root'"
        />
        <!-- 顶级文件夹按区块分段（2026-08-14 区块子系统）：区块标题行不可选；
             空区块段不渲染（无顶级文件夹无可选内容） -->
        <template v-for="sec in content.blockSections" :key="sec.block.id">
          <template v-if="sec.nodes.length > 0">
            <div v-if="showBlockHeaders" class="pick-block-title">{{ sec.block.name }}</div>
            <FolderPickNode
              v-for="node in sec.nodes"
              :key="node.id"
              :node="node"
              :depth="0"
              :picked="pickedId"
              :collapsed="collapsed"
              @pick="pickedId = $event"
              @toggle="toggle"
            />
          </template>
        </template>
      </div>
      <footer class="pick-footer">
        <button class="btn primary" :disabled="!pickedId" @click="onConfirm">确认</button>
        <button class="btn secondary" @click="onCancel">取消</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.pick-mask {
  position: fixed;
  inset: 0;
  z-index: 2200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
.pick-card {
  display: flex;
  flex-direction: column;
  width: 360px;
  max-height: 60vh;
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    inset 0 0 0 1px var(--border-1),
    var(--shadow-pop);
}
.pick-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 44px;
  padding: 0 var(--space-8) 0 var(--space-16);
  flex: none;
}
.pick-title {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.pick-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 var(--space-8) var(--space-8);
}
/* 区块标题行（05 稿区块标题文字样式：标签档 12 标签色，左 padding 8 对齐 item 内容）；
   不可选不响应 hover，仅作分段标识 */
.pick-block-title {
  display: flex;
  align-items: center;
  height: 20px;
  padding-left: var(--space-8);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
  flex: none;
}
.pick-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-8);
  padding: 0 var(--space-16) var(--space-16);
  flex: none;
}
.btn {
  height: 38px;
  padding: var(--space-8) var(--space-16);
  border-radius: var(--radius-6);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.btn.primary {
  background: var(--brand-1);
  color: var(--font-icon-1);
}
.btn.primary:disabled {
  opacity: 0.5;
}
.btn.secondary {
  background: var(--c5);
  color: var(--font-icon-1);
}
</style>
