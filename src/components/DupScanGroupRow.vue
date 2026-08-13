<script setup lang="ts">
// 扫描重复文件组行（35_扫描重复文件稿 item，320.94 高）：行头 = 组级复选框「N 个相同文件」
//（默认全选，用户单个取消）+ 行尾红色 ⊖（从列表移除该组——纯前端操作，文件不做任何处理，
// 2026-08-13 用户定案）；卡片横排，超过两个出横向滚动条（35 稿注明）。
import AppCheckbox from "./AppCheckbox.vue";
import IconButton from "./IconButton.vue";
import DupScanCard from "./DupScanCard.vue";
import type { DupGroup } from "../lib/dupScanOps";

defineProps<{ group: DupGroup }>();
const emit = defineEmits<{ remove: [] }>();
</script>

<template>
  <div class="scan-group">
    <!-- 行头（35 稿：复选框 + 红色删除，两端分布） -->
    <div class="row-head">
      <AppCheckbox v-model="group.checked" :label="`${group.assets.length} 个相同文件`" />
      <IconButton name="delete-red" delete-red title="从列表移除该组" @click="emit('remove')" />
    </div>
    <!-- 卡片横排（35 稿：gap 12；>2 张横向滚动） -->
    <div class="cards">
      <DupScanCard v-for="a in group.assets" :key="a.assetId" :asset="a" />
    </div>
  </div>
</template>

<style scoped>
/* item（35 稿）：padding 24 / gap 12 / 下分隔线（暗-悬停） */
.scan-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding: var(--space-24);
  box-shadow: inset 0 -1px 0 0 var(--line-hover);
  flex: none;
}
.row-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  height: 20px;
  flex: none;
}
/* 卡片行：横向滚动（35 稿注明：单行卡片超过两个时横向滚动条）；
   原生滚动条全局隐藏（base.css），此处按侧栏口径 6px 胶囊自绘横向条 */
.cards {
  display: flex;
  gap: var(--space-12);
  align-items: center;
  overflow-x: auto;
  padding-bottom: var(--space-6); /* 给横向滚动条留呼吸 */
}
.cards::-webkit-scrollbar {
  height: 6px;
}
.cards::-webkit-scrollbar-thumb {
  background: var(--line-hover);
  border-radius: 3px;
}
.cards::-webkit-scrollbar-thumb:hover {
  background: var(--line-active);
}
</style>
