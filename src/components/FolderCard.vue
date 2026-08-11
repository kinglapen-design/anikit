<script setup lang="ts">
// 文件夹卡片（figma-css 02_布局/42_中间面板 文件夹卡片，2026-08-10 重做定稿）：
// 横条芯片 65 高 / padding 12 24 / 间距 4 / C3 底 / 1px 暗-悬停边框（内阴影）/ 圆角 6；
// 行1 = 文件夹-关闭图标 18（高亮）+ 名称（正文 14/20 高亮，最多一行截断——旧两行居中作废）；
// 行2 = 数量（标签 12/17 标签档）；旧 SVG 纸卡造型与有缩略图态随新稿取消。
// 状态：hover 边框提亮 亮-选中（figma 无稿，按 token 线档递进）；选中 = 1.5px 品牌色-边框线；
// 双击名称重命名 = 整卡 1.5px 品牌色-边框线高亮提示（2026-08-10 用户定），输入框不再单独描边。
// 交互（2026-07-29 复验三轮）：双击卡片（非名称区域）= 进入该文件夹（FolderStrip 接线）；
// 双击名称 = 就地重命名（rename_folder 命令，stop 防触发"进入"）。
import { computed, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import { dragView } from "../lib/dragOperation";
import { renameFolder } from "../lib/placementOps";
import { useInlineRename } from "../lib/useInlineRename";
import { useSelectionStore } from "../stores/selection";
import type { FolderNode } from "../stores/content";

const props = withDefaults(defineProps<{ folder: FolderNode; selected?: boolean; dimmed?: boolean }>(), {
  selected: false,
  dimmed: false,
});
// select 携带 MouseEvent（2026-07-28 复验反馈 5：Ctrl 加选 / Shift 范围选需要修饰键）
const emit = defineEmits<{ select: [e: MouseEvent] }>();

// 拖拽落点视觉（6.1 C4b，2026-07-28 定案）：并入 = 覆盖整卡的方形覆盖
//（--brand-1-12 + inset 1px --brand-1）；同级排序 = 卡片左/右侧间隙竖块（仅文件夹拖拽）
const dropInto = computed(() => dragView.folderCardId === props.folder.id);
const gapSide = computed(() =>
  dragView.folderGapId === props.folder.id ? (dragView.folderGapAfter ? "right" : "left") : null,
);

// 副文本规则：有子文件夹显示"X 个文件 / Y 个子文件夹"，没有则仅"X 个文件"（可为 0）
const meta = computed(() => {
  const files = `${props.folder.count.toLocaleString()} 个文件`;
  return props.folder.children.length > 0
    ? `${files} / ${props.folder.children.length} 个子文件夹`
    : files;
});

// —— 名称就地重命名（2026-07-29 复验三轮）：双击名称进输入态（stop 防触发卡片双击"进入"）；
// Enter / 失焦 / Esc 一律保存退出——非空且变了才调 renameFolder，空或不变直接退出不动；
// 输入框 pointerdown 自停冒泡（FolderCard 是拖拽起点，防起拖拽 / 防头部空白清选择）
const renaming = ref(false);
const selection = useSelectionStore();
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
// 右键菜单"重命名"（6.3）：经 selection.namingFolderCardId 触发（与名称双击同入口）
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
    class="folder-card"
    :class="{ selected, dimmed, renaming }"
    :data-folder-id="folder.id"
    :data-drag-name="folder.name"
    @click="emit('select', $event)"
  >
    <div class="row-title">
      <AppIcon name="folder-closed" :size="18" class="f-icon" />
      <!-- 名称（2026-07-29 复验三轮）：双击 = 就地重命名（stop 防触发卡片双击"进入"）；
           命名态整卡 1.5px 品牌色-边框线高亮提示，输入框透明底不单独描边（2026-08-10 用户定） -->
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
      <!-- 名称悬停 tip（2026-08-10 用户定）：完整名称（限宽换行）+ 简介档（数量行） -->
      <span v-else class="title folder-name" v-tip="{ text: folder.name, keys: meta, column: true }" @dblclick.stop="startRename">{{ folder.name }}</span>
    </div>
    <span class="meta">{{ meta }}</span>
    <!-- 拖拽并入覆盖（C4b）：盖住整卡的方形覆盖 -->
    <span v-if="dropInto" class="drop-cover" aria-hidden="true" />
    <!-- 同级排序间隙竖块（C4 混排护栏：文件夹只与文件夹排序；宽 12 = 卡片间距档） -->
    <span v-if="gapSide" class="drop-gap" :class="gapSide" aria-hidden="true" />
  </div>
</template>

<style scoped>
/* 卡片（figma 42）：padding 12 24 / 间距 4 / C3 底 / 1px 暗-悬停边框（内阴影，规范第 4 条不用
   border）/ 圆角 6；宽度由 FolderStrip grid 弹性控制（min 185）。
   1.5px 状态边框由 ::after 伪元素 border 绘制（2026-08-10 走查两轮：inset box-shadow 1.5px
   分数物理像素底缘渗线；outline 负 offset + 圆角角部出尖刺 artifact——均弃；
   伪元素 border = 元素自身绘制不占布局、圆角内收、无渗线无尖角，同旧 icon-wrap::after 机制） */
.folder-card {
  position: relative; /* 状态边框伪元素与拖拽间隙竖块挂点 */
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-12) var(--space-24);
  border-radius: var(--radius-6);
  background: var(--c3);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  cursor: default;
}
.folder-card::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1.5px solid transparent;
  border-radius: var(--radius-6);
  pointer-events: none;
  transition: border-color 0.12s ease;
}
/* hover 1.5px 亮-选中（2026-08-10 用户定）；选中 / 重命名 1.5px 品牌色-边框线（重命名 =
   整卡高亮提示，输入框透明底不单独描边） */
.folder-card:hover::after {
  border-color: var(--line-active);
}
.folder-card.selected::after,
.folder-card.renaming::after {
  border-color: var(--brand-border);
}
/* 行1：图标 + 标题，间距 6，高 20 */
.row-title {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  height: 20px;
  min-width: 0;
  flex: none;
}
.f-icon {
  color: var(--text-highlight);
  flex: none;
}
/* 名称：正文 14/20 高亮，最多一行截断（2026-08-10 用户定，旧两行居中作废） */
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
/* 名称区悬停文本光标（2026-07-29 复验三轮：双击可改名） */
.folder-name:hover {
  cursor: text;
}
/* 重命名输入：透明底不单独描边（整卡高亮提示）；共享基座在 base.css，
   本地差异 = 满宽 + 不挤压图标 + 高亮字色 */
.rename-input {
  width: 100%;
  min-width: 0;
  color: var(--text-highlight);
}
/* 行2：数量，标签 12/17 标签档，单行截断 */
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: none;
}
/* 拖拽并入覆盖（C4b）：品牌 30% 填充 + 1.5px 品牌色-边框线（2026-08-10 用户定统一口径），盖住整卡 */
.drop-cover {
  position: absolute;
  inset: 0;
  z-index: 2;
  border-radius: var(--radius-6);
  background: var(--brand-1-30);
  box-shadow: inset 0 0 0 1.5px var(--brand-border);
  pointer-events: none;
}
/* 同级排序间隙竖块（C4）：宽 = 卡片间距 12 / 高随卡片整体，圆角竖向矩形色块；
   填充 30%（2026-08-10 用户定：12% 不够明显，与并入填充同档） */
.drop-gap {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 12px;
  border-radius: var(--radius-4);
  background: var(--brand-1-30);
  box-shadow: inset 0 0 0 1px var(--brand-1);
  pointer-events: none;
}
.drop-gap.left {
  left: -12px;
}
.drop-gap.right {
  right: -12px;
}
/* 拖拽源整卡半透明（2026-07-28 复验反馈 6①，拖拽结束引擎清态即恢复） */
.folder-card.dimmed {
  opacity: var(--opacity-drag-source);
}
</style>
