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
  <div v-for="(node, index) in nodes" :key="node.id">
    <TreeItem
      :icon="openSet.has(node.id) ? 'folder-open' : 'folder-closed'"
      :label="node.name"
      :count="node.count"
      :depth="depth"
      :expandable="node.children.length > 0"
      :open="openSet.has(node.id)"
      :selected="content.sidebarSelected === `folder:${node.id}`"
      :last="index === nodes.length - 1"
      guide
      @toggle="toggle(node.id)"
      @select="content.sidebarSelected = `folder:${node.id}`"
    />
    <div
      v-if="openSet.has(node.id) && node.children.length > 0"
      class="ft-children"
      :style="{ '--guide-x': `${depth * 20 + 24}px` }"
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
/* 缩进竖线（02_左侧边栏 04）：从父级装饰图标中心（= 子项 item 容器左缘，depth×14+25）
   贯穿整层——穿过 item 间距与嵌套子层区域（子级的子级旁父级线仍在，两条并存），
   止于最后子项中心（L 收笔）；各层线间距 = 层级缩进 14px。
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
