<script setup lang="ts">
// 序列帧 / 动图帧条（4.3；动图 2026-08-20 解包废除后改帧索引渲染）：常驻贴底，播放条下方。
// 容器样式同 13_播放条稿总容器（2026-08-20 用户定"帧选择统一成这种样式"）：直角 / C3 /
// border-top 暗-悬停 / padding 8 16——原"圆角 8 inset 边框浮条"口径作废。
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
    <IconButton name="nav-history" title="上一帧" :disabled="preview.frame <= 0" @click="preview.stepFrame(-1)" />
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
      name="nav-history"
      title="下一帧"
      class="rot-180"
      :disabled="preview.frame >= count - 1"
      @click="preview.stepFrame(1)"
    />
  </div>
</template>

<style scoped>
/* 总容器（13_播放条稿"逐帧栏样式和播放条总容器差不多，直角 border-top"，2026-08-20 用户定）：
   直角 / C3 / border-top 暗-悬停 / padding 8 16（左右与播放条对齐）——原圆角 8 inset 边框 +
   margin-top 4 浮条口径作废 */
.frame-strip {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-16);
  background: var(--c3);
  border-top: 1px solid var(--line-hover);
}
/* 上/下帧 = nav-history 与面包屑后退/前进同款（2026-08-20 用户定，弃用 chevron-small 旋转箭头）：
   上一帧不旋转（朝左），下一帧 rotate(180°) 朝右 */
.rot-180 :deep(svg) {
  transform: rotate(180deg);
}
/* 播放条系图标 = 标签档（13 稿 Vector 文字/标签，与播放条统一） */
.frame-strip :deep(.icon-btn) {
  color: var(--text-label);
}
/* hover 提亮到常态（同播放条口径）；禁用态不提亮 */
.frame-strip :deep(.icon-btn:hover:not(:disabled)) {
  color: var(--text-normal);
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
