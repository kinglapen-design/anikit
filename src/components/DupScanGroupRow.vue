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
  <div class="scan-group" :class="{ off: !group.checked }">
    <!-- 行头（35 稿：复选框 + 红色删除，两端分布） -->
    <div class="row-head">
      <AppCheckbox v-model="group.checked" :label="`${group.assets.length} 个相同文件`" />
      <IconButton name="delete-red" delete-red title="从列表移除该组" @click="emit('remove')" />
    </div>
    <!-- 卡片横排（35 稿：gap 12；>2 张横向滚动） -->
    <div class="cards kit-scroll-x">
      <DupScanCard v-for="a in group.assets" :key="a.assetId" :asset="a" />
    </div>
  </div>
</template>

<style scoped>
/* item（35 稿）：padding 24 / gap 12；底线 ::after 通栏（真 border 后与竖框不重叠）；
   最后一组不出底线——否则与底部按钮栏顶线重叠成双线（2026-08-13 走查） */
.scan-group {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding: var(--space-24);
  flex: none;
}
.scan-group::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 1px;
  background: var(--line-hover);
}
.scan-group:last-child::after {
  display: none;
}
.row-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  height: 20px;
  flex: none;
}
/* 取消勾选：卡片行压暗 30%（库内"跳过/压暗"统一口径，同 DuplicateRow skipped / 35 稿注释）；
   行头复选框保持常态可操作（可逆勾回） */
.scan-group.off .cards {
  opacity: 0.3;
}
/* 卡片行：横向滚动（35 稿注明：单行卡片超过两个时横向滚动条）；
   滚轮走 base.css .kit-scroll-x 统一类（6px 胶囊），padding-bottom 6 预留高度容纳 */
.cards {
  display: flex;
  gap: var(--space-12);
  align-items: center;
  padding-bottom: var(--space-6);
  transition: opacity 0.15s ease;
}
</style>
