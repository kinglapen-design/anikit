<script setup lang="ts">
// 逐帧笔记浮层（5.2）：播放条上方弹出（footer 内绝对定位 bottom:100%+间距 4，
// C3 底 / inset 1px border-1 / 圆角 8 / --shadow-pop / 内边距 8）。
// 头部："笔记" + 当前帧标识（sequence/anim=帧 x/y，video=帧 x/y 或 mm:ss 随"时间显示"设置，5.3）+ 关闭；
// textarea 写当前帧/时间点笔记（C4 输入框样式，失焦与 Ctrl+Enter 保存，key 见 previewNotes）；
// 下方全部笔记列表（帧号/时间 + 摘要单行省略 + 删除；点击跳帧/跳时间，当前项高亮）。
import { computed, nextTick, ref, watch } from "vue";
import IconButton from "./IconButton.vue";
import { formatDuration } from "../lib/formatDuration";
import { frameOfTime, totalFrames } from "../lib/formatFrame";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";
import { usePreviewNotesStore } from "../stores/previewNotes";

const ui = useUiStore();
const preview = usePreviewStore();
const media = usePreviewMediaStore();
const notes = usePreviewNotesStore();

const draft = ref("");
const inputRef = ref<HTMLTextAreaElement>();
// 用户编辑标志：仅 dirty 时失焦才保存——"打开未动就关/点列表项"不触发保存，
// 防空草稿误写删笔记（2026-07-26 用户反馈：从详情面板进入后笔记被误删）
const dirty = ref(false);

// 视频帧显示（5.3 帧化改造，2026-07-28 用户定：帧号显示，底层 key 仍 `t:{秒}` 不变）：
// 查看方式"时间显示 = 帧数"且 fps 可用；音频无 fps 天然回退时间显示
const videoFrames = computed(() => ui.timeDisplay === "frames" && media.fps != null && media.fps > 0);

// 当前帧标识：sequence/anim = 帧 x/y；video = 帧 x/y（帧数模式）或当前时间 mm:ss
const frameLabel = computed(() => {
  if (preview.kind === "video") {
    if (videoFrames.value && media.fps) {
      const total = totalFrames(media.duration, media.fps);
      return `帧 ${frameOfTime(media.currentTime, media.fps, total)} / ${total}`;
    }
    return formatDuration(media.currentTime);
  }
  const n = preview.seq?.frames.length ?? 0;
  return n > 0 ? `帧 ${preview.frame + 1} / ${n}` : "";
});

// 回填草稿：盯 currentText 本身（切帧/切素材/笔记异步加载完成都会触发）；
// 用户编辑中（dirty）不回填，防冲掉草稿
watch(
  () => [notes.currentKey, notes.currentText] as const,
  () => {
    if (dirty.value) return;
    draft.value = notes.currentText;
  },
  { immediate: true },
);

// 打开即进入编辑状态（v-if 挂载后聚焦输入框）
watch(
  () => notes.open,
  async (on) => {
    if (!on) return;
    await nextTick();
    inputRef.value?.focus();
  },
  { immediate: true },
);

function save(): void {
  if (!dirty.value) return;
  const k = notes.currentKey;
  if (k && draft.value !== notes.currentText) void notes.save(k, draft.value);
  dirty.value = false;
}
function onKeydown(e: KeyboardEvent): void {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
    save();
    e.preventDefault();
  }
}
// 列表条目标识：t:{秒} → 帧号（帧数模式且 fps 可用，5.3）或 mm:ss；帧名 → 帧号
function itemLabel(key: string): string {
  if (key.startsWith("t:")) {
    const secs = Number(key.slice(2));
    if (videoFrames.value && media.fps) {
      const total = totalFrames(media.duration, media.fps);
      return `帧 ${frameOfTime(secs, media.fps, total)}`;
    }
    return formatDuration(secs);
  }
  const i = preview.seq?.frames.findIndex((f) => f.name === key) ?? -1;
  return i >= 0 ? `帧 ${i + 1}` : key;
}
</script>

<template>
  <div v-if="notes.open" class="notes-pop">
    <header class="notes-head">
      <span class="notes-title">笔记</span>
      <span class="notes-frame">{{ frameLabel }}</span>
      <IconButton name="window-close" small title="关闭" @click="notes.open = false" />
    </header>
    <textarea
      ref="inputRef"
      v-model="draft"
      class="notes-input"
      rows="3"
      placeholder="记下这一帧的想法..."
      @input="dirty = true"
      @blur="save"
      @keydown="onKeydown"
    />
    <ul v-if="notes.notes.length > 0" class="notes-list">
      <li
        v-for="n in notes.notes"
        :key="n.key"
        class="note-item"
        :class="{ active: n.key === notes.currentKey }"
        @click="notes.jump(n.key)"
      >
        <span class="note-key">{{ itemLabel(n.key) }}</span>
        <span class="note-text">{{ n.text }}</span>
        <IconButton name="clear" small title="删除" @click.stop="notes.remove(n.key)" />
      </li>
    </ul>
  </div>
</template>

<style scoped>
/* 浮层：播放条上方（footer relative 内 bottom:100%+4），C3 / inset border-1 / 圆角 8 / --shadow-pop / 内边距 8 */
.notes-pop {
  position: absolute;
  left: var(--space-12);
  right: var(--space-12);
  bottom: 100%;
  margin-bottom: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
  z-index: 20;
  /* 弹出动画同查看方式菜单（锚点在下——从播放条向上展开，2026-07-26 用户定） */
  animation: menu-pop 0.14s cubic-bezier(0.16, 1, 0.3, 1);
  transform-origin: bottom center;
}
@keyframes menu-pop {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
.notes-head {
  display: flex;
  align-items: center;
  gap: var(--space-6);
}
.notes-title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  flex: none;
}
.notes-frame {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  flex: 1;
  min-width: 0;
}
/* 输入框：C4 底 / inset border-1 / 圆角 8 / 内边距 8，聚焦品牌紫描边（03_输入框语言） */
.notes-input {
  width: 100%;
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-1);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  resize: none;
}
.notes-input::placeholder {
  color: var(--font-icon-placeholder);
}
.notes-input:focus {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
/* 笔记列表：最多 5 条高滚动；当前项选中底色（font-icon-5） */
.notes-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 150px;
  overflow-y: auto;
}
.notes-list::-webkit-scrollbar {
  width: 4px;
}
.notes-list::-webkit-scrollbar-thumb {
  background: var(--border-1);
  border-radius: 2px;
}
.note-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-6);
  border-radius: var(--radius-4);
  cursor: pointer;
  transition: background 0.12s ease;
}
.note-item:hover {
  background: var(--font-icon-6);
}
.note-item.active {
  background: var(--font-icon-5);
}
/* 列表项小 ×：60% / hover 80%（与头部窗口关闭的主次区分，2026-07-26 用户定） */
.note-item .icon-btn {
  color: var(--font-icon-3);
}
.note-item .icon-btn:hover {
  color: var(--font-icon-2);
}
.note-key {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
  flex: none;
}
.note-text {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-2);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
