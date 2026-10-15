<script setup lang="ts">
// 扫描重复文件组行（35_扫描重复文件稿 item，320.94 高）：行头 = 组级复选框「N 个相同文件」
//（默认全选，用户单个取消）+ 行尾红色 X（从列表移除该组——纯前端操作，文件不做任何处理，
// 2026-08-13 用户定案）；卡片横排，超过两个出横向滚动条（35 稿注明）。
// 样式 = scanDialog.css 扫描弹窗族共享件（2026-10-16 抽出，与 LinkScanGroupRow 同构）。
import Checkbox from "./buttons/Checkbox.vue";
import IconButton from "./buttons/IconButton.vue";
import DupScanCard from "./DupScanCard.vue";
import type { DupGroup } from "../lib/dupScanOps";
import "./scanDialog.css";

defineProps<{ group: DupGroup }>();
const emit = defineEmits<{ remove: [] }>();
</script>

<template>
  <div class="scd-group" :class="{ off: !group.checked }">
    <!-- 行头（35 稿：复选框 + 红色删除，两端分布） -->
    <div class="scd-head">
      <Checkbox v-model="group.checked" :label="`${group.assets.length} 个相同文件`" />
      <IconButton name="clear" variant="bare" size="sm" title="从列表移除该组" @click="emit('remove')" />
    </div>
    <!-- 卡片横排（35 稿：gap 12；>2 张横向滚动） -->
    <div class="scd-cards kit-scroll-x">
      <DupScanCard v-for="a in group.assets" :key="a.assetId" :asset="a" />
    </div>
  </div>
</template>
