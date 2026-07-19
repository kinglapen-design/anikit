<script setup lang="ts">
// 文件夹树（递归）：展开 / 折叠状态每层自管，选中态写入 content store（阶段 1 仅样式）。
// 缩进线（02_左侧边栏 04）：竖线为子层容器级连续线（从子项展开占位中心 x5 贯穿整层，
// 穿过有子项的行与 item 间距，止于最后子项中心）；横线由叶子 TreeItem 自画（x5→x9）。
import { ref } from "vue";
import TreeItem from "./TreeItem.vue";
import { useContentStore } from "../stores/content";
import type { FakeFolder } from "../lib/fakeData";

withDefaults(defineProps<{ nodes: FakeFolder[]; depth?: number }>(), { depth: 0 });
const content = useContentStore();

const openSet = ref(new Set<number>());
function toggle(id: number): void {
  const next = new Set(openSet.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  openSet.value = next;
}
</script>

<template>
  <div v-for="node in nodes" :key="node.id">
    <TreeItem
      :icon="openSet.has(node.id) ? 'folder-open' : 'folder-closed'"
      :label="node.name"
      :count="node.count"
      :depth="depth"
      :expandable="node.children.length > 0"
      :open="openSet.has(node.id)"
      :selected="content.sidebarSelected === `folder:${node.id}`"
      guide
      @toggle="toggle(node.id)"
      @select="content.sidebarSelected = `folder:${node.id}`"
    />
    <div
      v-if="openSet.has(node.id) && node.children.length > 0"
      class="ft-children"
      :style="{ '--guide-x': `${(depth + 1) * 14 + 5}px` }"
    >
      <FolderTree :nodes="node.children" :depth="depth + 1" />
    </div>
  </div>
</template>

<style scoped>
.ft-children {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
/* 连续竖线：穿过整层（含 2px item 间距与有子项的行），止于最后子项中心（L 收笔）。
   颜色 = 装饰图标 fill 色（C4-7），自定义文件夹颜色时同步 */
.ft-children::before {
  content: "";
  position: absolute;
  left: var(--guide-x);
  top: 0;
  bottom: 14px;
  width: 1px;
  background: var(--c4-7);
}
</style>
