<script setup lang="ts">
// 档案卡片（2026-08-15 档案子系统一轮，figma-css 02_布局/42_中间面板 档案卡片 260×298）：
// 固定长宽比（260:298），卡后两层背景纸张（archive-paper-01/02，旋转 ±1° 标签/极淡色）微露边；
// 封面缩略图铺满（默认 = 子项第一张静态图，content.folderThumbMap 同文件夹卡片口径）——
// 无图时封面区透明，露出背景纸张做底；自定义封面（ArchiveData.avatar）随 22 稿编辑面板接入。
// 左上角徽标（archives map badge 有值时：文字 + 色值底色芯片，figma 格式徽标 2/4 圆角 4）；
// 底部信息条（78 高 C3 底 + 顶部 1px 暗-悬停线）：第一行名称单行省略（14/20 高亮），
// 第二行 N 个文件（12/17 标签），右下回形针（paperclip 16×38 标签色）。
// 整卡 1px 暗-悬停边框（内阴影）圆角 6；hover / 选中 / 重命名 = ::after 1.5px 状态描边
//（速查表 §7，同 FolderCard 机制）；重命名走 useInlineRename（速查表 §4），
// 入口 = 名称双击 / 右键菜单（selection.namingFolderCardId，与 FolderCard 同机制）。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { dragView } from "../lib/dragOperation";
import { getIcon } from "../lib/icons";
import { renameFolder } from "../lib/placementOps";
import { useInlineRename } from "../lib/useInlineRename";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const props = withDefaults(defineProps<{ folder: FolderNode; selected?: boolean; dimmed?: boolean }>(), {
  selected: false,
  dimmed: false,
});
// select 携带 MouseEvent（与 FolderCard 同：Ctrl 加选 / Shift 范围选需要修饰键）
const emit = defineEmits<{ select: [e: MouseEvent] }>();

const content = useContentStore();
const selection = useSelectionStore();

// 背景纸张 SVG（04_archive_card）：currentColor 填充，颜色由 .paper 的 color 档控制
const paperSvg1 = getIcon("archive-paper-01");
const paperSvg2 = getIcon("archive-paper-02");
const clipSvg = getIcon("paperclip");

// 封面：子项第一张静态图（folderThumbMap 既有口径）；无图 = 封面区透明露背景纸张
const cover = computed(() => {
  const p = content.folderThumbMap.get(props.folder.id);
  return p ? convertFileSrc(p) : null;
});

// 左上角徽标（ArchiveData.badge 有值才出）
const badge = computed(() => content.archives[props.folder.id]?.badge ?? null);

// 数量行：档案无子文件夹（后端护栏），恒"N 个文件"
const meta = computed(() => `${props.folder.count.toLocaleString()} 个文件`);

// 拖拽落点视觉（3.2 一轮）：素材拖入 = 覆盖整卡的并入覆盖（同 FolderCard drop-cover 口径）；
// 文件夹拖拽对档案卡片无并入落点（引擎侧已拦，档案下不允许子级文件夹）
const dropInto = computed(() => dragView.folderCardId === props.folder.id);

// —— 名称就地重命名（同 FolderCard 机制）：双击名称进输入态（stop 防触发卡片双击"进入"）；
// Enter / 失焦 / Esc 一律保存退出——非空且变了才调 renameFolder，空或不变直接退出不动
const renaming = ref(false);
const { draft: renameDraft, bindInput, commit: commitRename } = useInlineRename({
  active: () => renaming.value,
  initial: () => props.folder.name,
  escSaves: true,
  onCommit: async (name) => {
    renaming.value = false;
    if (!name || name === props.folder.name) return;
    await renameFolder(props.folder.id, name);
  },
});
function startRename(): void {
  renaming.value = true;
}
// 右键菜单"重命名"（04 稿）：经 selection.namingFolderCardId 触发（与名称双击同入口）
watch(
  () => selection.namingFolderCardId,
  (id) => {
    if (id === props.folder.id) {
      selection.namingFolderCardId = null;
      startRename();
    }
  },
);
</script>

<template>
  <div
    class="archive-card"
    :class="{ selected, dimmed, renaming }"
    :data-folder-id="folder.id"
    :data-drag-name="folder.name"
    @click="emit('select', $event)"
  >
    <!-- 背景纸张两层（figma 背景纸张-01/02）：微旋转露出卡片边缘 -->
    <span class="paper p2" aria-hidden="true" v-html="paperSvg2" />
    <span class="paper p1" aria-hidden="true" v-html="paperSvg1" />
    <div class="card-body">
      <!-- 封面区（220:78 比例分高）：有图铺满；无图透明露背景纸张做底 -->
      <div class="thumb" :class="{ empty: !cover }">
        <img v-if="cover" :src="cover" alt="" draggable="false" />
      </div>
      <!-- 底部信息条：名称单行省略 + N 个文件 -->
      <div class="info">
        <input
          v-if="renaming"
          :ref="bindInput"
          v-model="renameDraft"
          class="rename-input"
          @keydown.enter.prevent="commitRename"
          @keydown.esc.prevent="commitRename"
          @blur="commitRename"
          @click.stop
          @dblclick.stop
          @pointerdown.stop
        />
        <!-- 名称悬停 tip（同 FolderCard 口径）：完整名称 + 简介档（数量行） -->
        <span v-else class="title archive-name" v-tip="{ text: folder.name, keys: meta, column: true }" @dblclick.stop="startRename">{{ folder.name }}</span>
        <span class="meta">{{ meta }}</span>
      </div>
      <!-- 左上角徽标（badge 有值时：文字 + 色值底色芯片） -->
      <span v-if="badge" class="badge" :style="{ background: badge.color }">{{ badge.text }}</span>
      <!-- 右下回形针 -->
      <span class="clip" aria-hidden="true" v-html="clipSvg" />
    </div>
    <!-- 素材拖拽并入覆盖：盖住整卡的方形覆盖（同 FolderCard drop-cover 口径） -->
    <span v-if="dropInto" class="drop-cover" aria-hidden="true" />
  </div>
</template>

<style scoped>
/* 卡片（figma 42）：260×298 固定长宽比；宽度由 ArchiveStrip grid 弹性控制（min 260 = 稿宽）。
   1.5px 状态边框由 ::after 伪元素 border 绘制（速查表 §7，同 FolderCard 机制） */
.archive-card {
  position: relative; /* 背景纸张 / 状态边框伪元素 / 拖拽覆盖挂点 */
  aspect-ratio: 260 / 298;
  cursor: default;
}
.archive-card::after {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 4;
  border: 1.5px solid transparent;
  border-radius: var(--radius-6);
  pointer-events: none;
  transition: border-color 0.12s ease;
}
/* hover 1.5px 亮-选中；选中 / 重命名 1.5px 品牌色-边框线（同 FolderCard 口径） */
.archive-card:hover::after {
  border-color: var(--line-active);
}
.archive-card.selected::after,
.archive-card.renaming::after {
  border-color: var(--brand-border);
}
/* 背景纸张（figma：258×298 旋转 ±1° 微露卡片边缘）：currentColor 吃 color 档——
   01 = 文字/标签 rotate(1.15deg) 在上，02 = 文字/极淡 rotate(-1.42deg) 在下 */
.paper {
  position: absolute;
  inset: 0;
  color: var(--text-faint);
}
.paper.p1 {
  color: var(--text-label);
}
.paper :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
/* 卡片本体（figma 卡片层）：1px 暗-悬停边框（内阴影，规范第 4 条）圆角 6；
   封面/信息条圆角由本层 overflow 裁切 */
.card-body {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  overflow: hidden;
}
/* 封面区：220/298 比例分高；有图 C3 底铺满裁切，无图透明露背景纸张（archive-paper 做底） */
.thumb {
  flex: 220;
  min-height: 0;
  background: var(--c3);
}
.thumb.empty {
  background: transparent;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
/* 信息条（figma 简介 78 高）：padding 8 8 8 12、间距 4、C3 底、顶部 1px 暗-悬停线 */
.info {
  flex: 78;
  min-height: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--space-4);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
  background: var(--c3);
  box-shadow: inset 0 1px 0 var(--line-hover);
}
/* 名称：正文 14/20 高亮，单行截断（同 FolderCard 口径） */
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
/* 名称区悬停文本光标（双击可改名，同 FolderCard） */
.archive-name:hover {
  cursor: text;
}
/* 重命名输入：透明底不单独描边（整卡高亮提示）；共享基座在 base.css */
.rename-input {
  width: 100%;
  min-width: 0;
  color: var(--text-highlight);
}
/* 数量行：标签 12/17 标签档，单行截断 */
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 左上角徽标芯片（figma 格式徽标）：padding 2 4、圆角 4、徽标 10/14 700 常态色、底色 = badge.color */
.badge {
  position: absolute;
  left: 6px;
  top: 6px;
  max-width: calc(100% - 12px);
  padding: 2px var(--space-4);
  border-radius: var(--radius-4);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 右下回形针（paperclip 16×38 标签色） */
.clip {
  position: absolute;
  right: 8px;
  bottom: 8px;
  width: 16px;
  height: 38px;
  color: var(--text-label);
  pointer-events: none;
}
.clip :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
/* 素材拖拽并入覆盖：品牌 30% 填充 + 1.5px 品牌色-边框线（::after border 同 FolderCard 口径），盖住整卡 */
.drop-cover {
  position: absolute;
  inset: 0;
  z-index: 3;
  border: 1.5px solid var(--brand-border);
  border-radius: var(--radius-6);
  background: var(--brand-1-30);
  pointer-events: none;
}
/* 拖拽源整卡半透明（同 FolderCard 口径，拖拽结束引擎清态即恢复） */
.archive-card.dimmed {
  opacity: var(--opacity-drag-source);
}
</style>
