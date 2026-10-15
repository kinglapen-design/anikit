<script setup lang="ts">
// 扫描文件/档案链接组行（35 稿复用口径，PRD 03_查找文件-04，2026-10-16 立项）：
// 行头 = 「N 个链接文件」说明文字（无组级复选——复选语义改挂在链接卡上）+ 行尾红色 X
//（从列表移除该组，纯前端操作，链接不动）；卡片横排 = 首列原始文件卡（恒留不可选）+
// 其后全部链接卡（整卡可点，复选 = 删除标记，选中压暗 30%，默认全不选），超过两个横向滚动。
// 样式 = scanDialog.css 扫描弹窗族共享件（与 DupScanGroupRow 同构）。
import IconButton from "./buttons/IconButton.vue";
import DupScanCard from "./DupScanCard.vue";
import type { LinkGroup } from "../lib/linkScanOps";
import "./scanDialog.css";

defineProps<{ group: LinkGroup }>();
const emit = defineEmits<{ remove: [] }>();
</script>

<template>
  <div class="scd-group">
    <div class="scd-head">
      <span class="scd-head-text">{{ group.links.length }} 个链接文件</span>
      <IconButton name="clear" variant="bare" size="sm" title="从列表移除该组" @click="emit('remove')" />
    </div>
    <div class="scd-cards kit-scroll-x">
      <!-- 首列原始文件卡（不可选）；链接卡同名撞键带序号兜底（同文件夹重复链接场景） -->
      <DupScanCard :asset="group.asset" />
      <DupScanCard
        v-for="(l, i) in group.links"
        :key="`${l.folderId}:${l.name}:${i}`"
        :asset="group.asset"
        badge="link"
        :name-override="l.name"
        :meta-override="l.folderName"
        selectable
        :selected="l.checked"
        @toggle="l.checked = !l.checked"
      />
    </div>
  </div>
</template>
