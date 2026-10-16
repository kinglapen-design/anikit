<script setup lang="ts">
// 扫描文件/档案链接组行（35 稿族口径，PRD 03_查找文件-04 屏 1）：行头 = 组级复选框「N 个链接文件」
//（默认全选，用户单个取消——选中 = 本组进归并）+ 行尾红色 X（从列表移除该组，纯前端）；
// 卡片横排只看不动（2026-10-16 两段式改版：逐条选择删除移到屏 2 保留者 radio 与链接管理）——
// 首列原始文件卡，其后链接卡（链接显示名 + 所在文件夹 + 链接徽章）；超过两个横向滚动。
// 样式 = scanDialog.css 扫描弹窗族共享件（与 DupScanGroupRow 同构）。
import Checkbox from "./buttons/Checkbox.vue";
import IconButton from "./buttons/IconButton.vue";
import DupScanCard from "./DupScanCard.vue";
import type { LinkGroup } from "../lib/linkScanOps";
import "./scanDialog.css";

defineProps<{ group: LinkGroup }>();
const emit = defineEmits<{ remove: [] }>();
</script>

<template>
  <div class="scd-group" :class="{ off: !group.checked }">
    <div class="scd-head">
      <Checkbox v-model="group.checked" :label="`${group.links.length} 个链接文件`" />
      <IconButton name="clear" variant="bare" size="sm" title="从列表移除该组" @click="emit('remove')" />
    </div>
    <div class="scd-cards kit-scroll-x">
      <!-- 首列原始文件卡；链接卡同名撞键带序号兜底（同文件夹重复链接场景） -->
      <DupScanCard :asset="group.asset" />
      <DupScanCard
        v-for="(l, i) in group.links"
        :key="`${l.folderId}:${l.name}:${i}`"
        :asset="group.asset"
        badge="link"
        :name-override="l.name"
        :meta-override="l.folderName"
      />
    </div>
  </div>
</template>
