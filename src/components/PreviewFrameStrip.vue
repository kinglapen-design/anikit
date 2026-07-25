<script setup lang="ts">
// 序列帧帧条（4.3；动图 5.2 完全复用）：常驻，播放条下方。
// 上/下帧按钮 + 逐帧预览块（后端预生成的 cache 小图）；点击定位帧；
// 切换帧时高亮块先移至可见区两端，顶头后帧列表按需滑动（scrollIntoView nearest，2026-07-25 定案）。
import { nextTick, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import IconButton from "./IconButton.vue";
import { usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
const listRef = ref<HTMLElement>();

// 高亮块确保可见（nearest：可见区外才滑动，正好实现"先移到两端、顶头后按需滑动"）
watch(
  () => preview.frame,
  async () => {
    await nextTick();
    const el = listRef.value?.children[preview.frame] as HTMLElement | undefined;
    el?.scrollIntoView({ inline: "nearest", block: "nearest" });
  },
);
</script>

<template>
  <div v-if="preview.seq" class="frame-strip">
    <IconButton name="chevron-small" title="上一帧" class="rot-l" @click="preview.stepFrame(-1)" />
    <div ref="listRef" class="frames">
      <button
        v-for="(f, i) in preview.seq.frames"
        :key="f.name"
        class="frame"
        :class="{ active: i === preview.frame }"
        @click="preview.setFrame(i)"
      >
        <img :src="convertFileSrc(f.stripPath)" :alt="f.name" draggable="false" loading="lazy" />
      </button>
    </div>
    <IconButton name="chevron-small" title="下一帧" class="rot-r" @click="preview.stepFrame(1)" />
  </div>
</template>

<style scoped>
/* 总容器：照播放条（C3 / inset border-1 / 圆角 8 / 内边距 8 / 间距 8） */
.frame-strip {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow: inset 0 0 0 1px var(--border-1);
  margin-top: var(--space-4);
}
/* chevron-small 原图标朝下：上一帧 = 顺时针 90°（朝左），下一帧 = 逆时针 90°（朝右） */
.rot-l :deep(svg) {
  transform: rotate(90deg);
}
.rot-r :deep(svg) {
  transform: rotate(-90deg);
}
.frames {
  display: flex;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none; /* 帧列表按需滑动，不出滚动条 */
}
.frames::-webkit-scrollbar {
  display: none;
}
/* 帧块：高 56 宽按比例 / 圆角 8 / 1px border-2；当前帧 2px 品牌紫（卡片选中语言） */
.frame {
  flex: none;
  height: 56px;
  border: 1px solid var(--border-2);
  border-radius: var(--radius-8);
  overflow: hidden;
  transition: border-color 0.12s ease;
}
.frame img {
  display: block;
  height: 100%;
  width: auto;
}
.frame.active {
  border: 2px solid var(--brand-1);
}
</style>
