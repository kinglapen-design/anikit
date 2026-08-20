<script setup lang="ts">
// 序列帧 / 动图帧条（4.3；动图 2026-08-20 解包废除后改帧索引渲染）：常驻，播放条下方。
// 上/下帧按钮 + 逐帧预览块（cache/frames 会话级小图：序列帧从磁盘帧图缩放，动图 origin 按需解码）；
// 点击定位帧；切换帧时高亮块先移至可见区两端，顶头后帧列表按需滑动（scrollIntoView nearest，2026-07-25 定案）。
// 动图分支：帧索引 0..animTotal-1，小图 key = `f:{i}`（animStrips 渐进填充），笔记圆点同 key。
import { computed, nextTick, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import IconButton from "./IconButton.vue";
import { usePreviewStore } from "../stores/preview";
import { usePreviewNotesStore } from "../stores/previewNotes";

const preview = usePreviewStore();
const notes = usePreviewNotesStore();
const listRef = ref<HTMLElement>();

// 帧条出现条件：序列帧有帧清单；动图帧流活着（animTotal > 0。回退态不显示）
const show = computed(() => !!preview.seq || (preview.kind === "anim" && preview.animTotal > 0));
// 总帧数：序列帧 = 清单长度；动图 = animTotal
const count = computed(() => (preview.kind === "anim" ? preview.animTotal : (preview.seq?.frames.length ?? 0)));
// 动图帧块数据（索引驱动；stripPath 渐进填充，未就绪占位）
const animFrames = computed(() =>
  Array.from({ length: preview.animTotal }, (_, i) => ({
    key: `f:${i}`,
    stripPath: preview.animStrips[`f:${i}`] ?? null,
  })),
);

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
  <div v-if="show" class="frame-strip">
    <IconButton name="chevron-small" title="上一帧" class="rot-l" :disabled="preview.frame <= 0" @click="preview.stepFrame(-1)" />
    <div ref="listRef" class="frames">
      <!-- 动图分支（帧索引驱动） -->
      <template v-if="preview.kind === 'anim'">
        <button
          v-for="f in animFrames"
          :key="f.key"
          class="frame"
          :class="{ active: Number(f.key.slice(2)) === preview.frame }"
          @click="preview.setFrame(Number(f.key.slice(2)))"
        >
          <img v-if="f.stripPath" :src="convertFileSrc(f.stripPath)" :alt="f.key" draggable="false" loading="lazy" />
          <span v-else class="strip-pending" />
          <span v-if="notes.keySet.has(f.key)" class="note-dot" />
        </button>
      </template>
      <!-- 序列帧分支（帧清单驱动） -->
      <button
        v-else
        v-for="(f, i) in preview.seq!.frames"
        :key="f.name"
        class="frame"
        :class="{ active: i === preview.frame }"
        @click="preview.setFrame(i)"
      >
        <img v-if="f.stripPath" :src="convertFileSrc(f.stripPath)" :alt="f.name" draggable="false" loading="lazy" />
        <span v-else class="strip-pending" />
        <span v-if="notes.keySet.has(f.name)" class="note-dot" />
      </button>
    </div>
    <IconButton
      name="chevron-small"
      title="下一帧"
      class="rot-r"
      :disabled="preview.frame >= count - 1"
      @click="preview.stepFrame(1)"
    />
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
/* 中栏工具栏图标按钮 80%（2026-07-23 用户定，同预览标题栏） */
.frame-strip :deep(.icon-btn) {
  color: var(--font-icon-2);
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
/* 帧块：高 56 宽按比例 / 圆角 8 / 1px border-2；当前帧 1.5px 品牌色-边框线（卡片选中语言）。
   当前帧边框 = ::after 状态层 opacity 过渡（2026-08-19 残留修复，踩坑 036：元素 border
   换色过渡是重绘型动画，圆角 + 1.5px 分数像素下合成损伤区偶发漏覆盖致品牌色残留） */
.frame {
  position: relative;
  flex: none;
  height: 56px;
  border: 1px solid var(--border-2);
  border-radius: var(--radius-8);
  overflow: hidden;
}
.frame::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1.5px solid var(--brand-border);
  border-radius: var(--radius-8);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.12s ease;
}
.frame.active::after {
  opacity: 1;
}
/* 有笔记的帧：右上角品牌紫小圆点（5.2） */
.note-dot {
  position: absolute;
  top: var(--space-2);
  right: var(--space-2);
  width: var(--space-6);
  height: var(--space-6);
  border-radius: 50%;
  background: var(--brand-1);
}
.frame img {
  display: block;
  height: 100%;
  width: auto;
}
/* 帧条小图后台生成中的占位（方形 C4，就绪后按图比例替换，2026-07-25） */
.strip-pending {
  display: block;
  width: 56px;
  height: 100%;
  background: var(--c4);
}
</style>
