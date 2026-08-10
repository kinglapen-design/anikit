<script setup lang="ts">
// 文件夹选择弹窗（6.3"添加至文件夹"）：文件夹树单选目标 → 确认回调（关联 / 副本弹窗前置）。
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
        <TreeItem
          icon="folder-closed"
          label="未分类"
          :selected="pickedId === 'root'"
          @select="pickedId = 'root'"
        />
        <FolderPickNode
          v-for="node in content.folders"
          :key="node.id"
          :node="node"
          :depth="0"
          :picked="pickedId"
          :collapsed="collapsed"
          @pick="pickedId = $event"
          @toggle="toggle"
        />
      </div>
      <footer class="pick-footer">
        <button class="btn secondary" @click="onCancel">取消</button>
        <button class="btn primary" :disabled="!pickedId" @click="onConfirm">确认</button>
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
