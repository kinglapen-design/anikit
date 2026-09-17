<script setup lang="ts">
// 链接文件 item——docs/06_UI重构/Design Token/组件/item/详情面板 item/链接文件
// 卡片 240×66 / padding 12 / 两行：文件夹图标 + 路径（区块名 正文加粗 二级 / 目录 加粗 三级）、文件名 三级；
// 关闭钮 hover 从右缘向左滑入并把内容推左（items.css .ni-slide 推挤机制，2026-08-29 走查定——
// 此前绝对定位 translateX 从容器右缘进来，与其他 item 从内容缘进来不统一，作废）；
// 原始文件徽章（品牌星）绝对定位右下——仅 origin 且活位置 > 1 出（2026-09-05 走查：与卡片徽章同口径，
// 单位置无链接可区分不出；判定在调用方 TabLinks：origin && linkItems.length > 1）。
import "./items.css";
import AppIcon from "../AppIcon.vue";
import TwoToneIcon from "./TwoToneIcon.vue";

withDefaults(
  defineProps<{
    blockName: string; // 区块名（正文-加粗，亮色/二级）
    folderName: string; // 目录名（正文-加粗，亮色/三级）
    fileName: string; // 显示名（正文，亮色/三级）
    origin?: boolean; // 原始文件徽章（品牌星，右下）
    forceState?: "" | "hover";
  }>(),
  { origin: false, forceState: "" },
);

const emit = defineEmits<{ click: []; close: [] }>();
</script>

<template>
  <div
    class="lf ni-card ni-hover"
    :class="forceState ? `force-${forceState}` : ''"
    @click="emit('click')"
  >
    <div class="body">
      <div class="path">
        <TwoToneIcon name="folder-closed" :size="18" />
        <span class="seg">
          <span class="blk">{{ blockName }}</span>
          <span class="sep">/</span>
          <span class="dir">{{ folderName }}</span>
        </span>
      </div>
      <div class="fname" :class="{ 'has-badge': origin }">{{ fileName }}</div>
    </div>
    <span class="ni-slide">
      <span class="ni-slide-inner">
        <button class="ni-close" @click.stop="emit('close')">
          <AppIcon name="close-small" :size="14" />
        </button>
      </span>
    </span>
    <span v-if="origin" class="badge" aria-hidden="true">
      <AppIcon name="original-file-badge" :size="14" />
    </span>
  </div>
</template>

<style scoped>
.lf {
  display: flex;
  align-items: flex-start;
  gap: var(--n-space-6);
  width: 240px;
  height: 66px;
  padding: var(--n-space-12);
  cursor: pointer;
}
.body {
  flex: 1;
  min-width: 0;
  align-self: stretch;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--n-space-6);
}
.path {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
}
.seg {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  white-space: nowrap;
}
.blk {
  color: var(--n-l2);
}
.sep {
  color: var(--n-l2);
  flex: none;
}
.dir {
  color: var(--n-l3);
  overflow: hidden;
  text-overflow: ellipsis;
}
.fname {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 有原始文件徽章（右下 14px）：名称右缩 徽章 14 + 间距 6，省略号收在徽章前——
   2026-09-13 实机走查：徽章盖住长文件名 */
.fname.has-badge {
  padding-right: calc(14px + var(--n-space-6));
}
.badge {
  position: absolute;
  right: var(--n-space-12);
  bottom: var(--n-space-12);
  width: 14px;
  height: 14px;
  display: inline-flex;
  pointer-events: none;
}
</style>
