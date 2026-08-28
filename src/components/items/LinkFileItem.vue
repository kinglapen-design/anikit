<script setup lang="ts">
// 链接文件 item——docs/UI 重构/Design Token/组件/item/详情面板 item/链接文件
// 卡片 240×66 / padding 12 / 两行：文件夹图标 + 路径（区块名 正文加粗 二级 / 目录 加粗 三级）、文件名 三级；
// 关闭钮绝对定位右上（hover 滑入），原始文件徽章（品牌星）绝对定位右下（仅 origin）。
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
    <div class="path">
      <TwoToneIcon name="folder-closed" :size="18" />
      <span class="seg">
        <span class="blk">{{ blockName }}</span>
        <span class="sep">/</span>
        <span class="dir">{{ folderName }}</span>
      </span>
    </div>
    <div class="fname">{{ fileName }}</div>
    <button class="ni-close lf-close" @click.stop="emit('close')">
      <AppIcon name="close-small" :size="14" />
    </button>
    <span v-if="origin" class="badge" aria-hidden="true">
      <AppIcon name="original-file-badge" :size="14" />
    </span>
  </div>
</template>

<style scoped>
.lf {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--n-space-6);
  width: 240px;
  height: 66px;
  padding: var(--n-space-12);
  cursor: pointer;
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
/* 关闭钮：绝对定位右上，常态不显示，hover 右至左滑入（位移 + 透明度） */
.lf-close {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  opacity: 0;
  transform: translateX(4px);
  transition:
    opacity var(--n-dur-150) var(--n-ease-in-out),
    transform var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-100) var(--n-ease-in-out);
}
.ni-card:hover .lf-close,
.ni-card.force-hover .lf-close {
  opacity: 1;
  transform: translateX(0);
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
