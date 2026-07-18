<script setup lang="ts">
// 文件夹横排区（组件布局 05.2）：区块副标题 + 单排文件夹卡片，纵轮横滚。
import { ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import FolderCard from "./FolderCard.vue";
import { useContentStore } from "../stores/content";

const content = useContentStore();
const stripRef = ref<HTMLElement>();

function onWheel(e: WheelEvent): void {
  const el = stripRef.value;
  if (!el) return;
  // 纵向滚轮转横向；能横向滚动时阻止冒泡（避免触发网格自定义滚动）
  const before = el.scrollLeft;
  el.scrollLeft += e.deltaY;
  if (el.scrollLeft !== before) e.stopPropagation();
}
</script>

<template>
  <section class="folder-strip">
    <SectionHeader title="文件夹" :count="content.stripFolders.length" />
    <div ref="stripRef" class="strip" @wheel="onWheel">
      <FolderCard v-for="f in content.stripFolders" :key="f.id" :folder="f" />
    </div>
  </section>
</template>

<style scoped>
.folder-strip {
  flex: none;
}
/* 文件夹卡片容器：内边距 8 / 文件夹列表子项间距 6 */
.strip {
  display: flex;
  gap: var(--space-6);
  padding: var(--space-8);
  overflow-x: auto;
}
</style>
