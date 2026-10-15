<script setup lang="ts">
// 扫描污染文件行（2026-10-16，PRD 03_查找文件-05）：ni-card 52 高横条 = 40 缩略图 + 名称 +
// 「N 个污染文件 · 所在文件夹」；点击行 = 定位（locate）；hover 滑入两动作 = 打开资源管理器 /
// 全部丢到回收站（ni-slide + ni-act 族，多钮组 flex:none 完整滑入口径）。
// forceState = preview 陈列强制 hover（滑入钮组可见性演示）。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import type { PollutionAsset } from "../lib/pollutionScanOps";
import "./items/items.css";

const props = withDefaults(defineProps<{ row: PollutionAsset; busy?: boolean; forceState?: "" | "hover" }>(), {
  busy: false,
  forceState: "",
});
const emit = defineEmits<{ locate: []; reveal: []; trash: [] }>();

const thumbUrl = computed(() => (props.row.asset.thumbPath ? convertFileSrc(props.row.asset.thumbPath) : null));
</script>

<template>
  <div class="ps-row ni-card ni-hover" :class="forceState ? `force-${forceState}` : ''" @click="emit('locate')">
    <span class="ps-thumb">
      <img
        v-if="thumbUrl"
        :class="{ icon: row.asset.thumbKind === 'formatIcon' }"
        :src="thumbUrl!"
        :alt="row.asset.name"
        decoding="async"
        draggable="false"
      />
      <span v-else class="ext">{{ row.asset.ext.toUpperCase() }}</span>
    </span>
    <span class="ps-text">
      <span class="ps-name" :title="row.asset.name">{{ row.asset.name }}</span>
      <span class="ps-meta">{{ row.items.length }} 个污染文件 · {{ row.asset.folderName }}</span>
    </span>
    <span class="ni-slide">
      <span class="ni-slide-inner">
        <button class="ni-act" title="在资源管理器中打开" @click.stop="emit('reveal')">
          <AppIcon name="open-in-explorer" :size="18" />
        </button>
        <button class="ni-act" title="全部丢到回收站" :disabled="busy" @click.stop="emit('trash')">
          <AppIcon name="trash" :size="18" />
        </button>
      </span>
    </span>
  </div>
</template>

<style scoped>
/* 行 = ni-card（g3 底 + 1px g5 + 圆角 8，hover g4/g6 由 ni-card 自带）：52 高横条，
   点击 = 定位（cursor pointer）；滑入钮组 flex:none（多钮组必须完整滑入，2026-10-16 口径） */
.ps-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  height: 52px;
  padding: var(--n-space-6) var(--n-space-12);
  flex: none;
  cursor: pointer;
}
.ps-row .ni-slide {
  flex: none;
}
/* 缩略图 40×40（扫描族组列表同档）：g3 底 + 圆角 6；格式图标 contain 不裁切 */
.ps-thumb {
  width: 40px;
  height: 40px;
  border-radius: var(--n-radius-6);
  background: var(--n-g3);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
.ps-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: var(--n-radius-6);
}
.ps-thumb img.icon {
  object-fit: contain;
}
.ps-thumb .ext {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
}
.ps-text {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  min-width: 0;
  flex: 1;
}
.ps-name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ps-meta {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
