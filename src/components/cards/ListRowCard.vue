<script setup lang="ts">
// 列表卡片——docs/UI 重构/Design Token/组件/卡片/列表卡片.md：
// 行高 57 / padding 6/12/6/6 / gap 12 / 圆角 8；通用二级底 + 通用三级 1px 描边；
// hover = 底升通用三级 + 描边升通用五级（描边瞬时切换，踩坑 036 口径）；选中 2px 品牌描边（.kc-sel）。
// 列：缩略图 80×45（通用三级底 + 五级 1px 描边 + 圆角 6；thumbFit = contain 默认 / icon 格式图标
//   50% 居中不撑满——2026-08-30 用户实机，FileCard icon 档同口径）
//   / 名称+徽章（flex 1 gap 12：正文-加粗 一级 + 原始文件徽章 14 / 链接文件徽章 14）
//   / 尺寸 / 格式（FormatBadge）/ 文件大小 / 修改时间（日期 + 时间 gap 6）——各 123 宽，数字-大档 亮色/三级。
// 业务态（7.1 内容容器轮接线，同 cards/FileCard 模式）：naming = 行内重命名（名称位换输入框，
// Esc = 取消 nameCancel 上抛，Enter/失焦 = nameCommit，空/不变判定在父级壳层）。
import AppIcon from "../AppIcon.vue";
import FormatBadge from "./FormatBadge.vue";
import { useInlineRename } from "../../lib/useInlineRename";
import "./cards.css";

const props = withDefaults(
  defineProps<{
    name: string;
    thumbUrl?: string;
    thumbFit?: "contain" | "icon"; // 缩略图适配：contain 默认 / icon = 格式图标 50% 居中（FileCard 同口径）
    badge?: string; // 格式角标（扩展名列）
    dims?: string; // 尺寸（1920 x 1080）
    fileSize?: string; // 文件大小（12 MB）
    modDate?: string; // 修改日期（2026/08/24）
    modTime?: string; // 修改时间（16:07）
    originBadge?: boolean; // 名称后原始文件-徽章
    linkedBadge?: boolean; // 名称后链接文件-徽章（非原始位置，活位置 > 1）
    selected?: boolean;
    naming?: boolean; // 行内重命名输入态
    forceState?: "" | "hover" | "selected";
  }>(),
  {
    thumbUrl: "",
    thumbFit: "contain",
    badge: "",
    dims: "",
    fileSize: "",
    modDate: "",
    modTime: "",
    originBadge: false,
    linkedBadge: false,
    selected: false,
    naming: false,
    forceState: "",
  },
);

const emit = defineEmits<{
  nameCommit: [name: string];
  nameCancel: []; // Esc（文件重命名口径 = 取消）
  nameEdit: []; // 名称双击 = 进命名态
}>();

// 行内重命名（useInlineRename 封装，速查 §4）：selectStem = 词干选区不包后缀（文件口径）
const { draft: nameDraft, bindInput, commit: commitName, onEsc: cancelName } = useInlineRename({
  active: () => props.naming,
  initial: () => props.name,
  selectStem: true,
  onCommit: (name) => emit("nameCommit", name),
  onCancel: () => emit("nameCancel"),
});
</script>

<template>
  <div
    class="kc-bordered kc-sel lr-card"
    :class="[{ selected, naming }, forceState ? `force-${forceState}` : '']"
  >
    <span class="lr-thumb">
      <img v-if="thumbUrl" :src="thumbUrl" class="lr-thumb-img" :class="thumbFit" alt="" decoding="async" draggable="false" />
    </span>
    <span class="lr-name-cell">
      <!-- 命名态：input 替换名称位（整行品牌环提示，输入框透明底，速查 §4 统一口径） -->
      <input
        v-if="naming"
        :ref="bindInput"
        v-model="nameDraft"
        class="rename-input"
        @keydown.enter.prevent="commitName"
        @keydown.esc.prevent="cancelName"
        @blur="commitName"
        @click.stop
        @dblclick.stop
        @pointerdown.stop
      />
      <span v-else class="lr-name" @dblclick.stop="emit('nameEdit')">{{ name }}</span>
      <AppIcon v-if="originBadge" name="original-file-badge" :size="14" />
      <AppIcon v-else-if="linkedBadge" name="linked-file-badge" :size="14" class="lr-linked" />
    </span>
    <span class="lr-col lr-dims">{{ dims }}</span>
    <span class="lr-col">
      <FormatBadge v-if="badge" :label="badge" />
    </span>
    <span class="lr-col lr-size">{{ fileSize }}</span>
    <span class="lr-col lr-mtime">
      <span>{{ modDate }}</span>
      <span>{{ modTime }}</span>
    </span>
  </div>
</template>

<style scoped>
.lr-card {
  box-sizing: border-box;
  position: relative;
  width: 100%;
  display: flex;
  flex-direction: row;
  align-items: center;
  padding: var(--n-space-6) var(--n-space-12) var(--n-space-6) var(--n-space-6);
  gap: var(--n-space-12);
  background: var(--n-g2);
  border-radius: var(--n-radius-8);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.lr-card:hover,
.lr-card.force-hover {
  background: var(--n-g3);
}
/* 常态 1px 描边 = cards.css .kc-bordered::after，hover 升五级（瞬时换色） */
.lr-card:hover::after,
.lr-card.force-hover::after {
  border-color: var(--n-g5);
}

/* 缩略图容器 80×45：通用三级底 + 五级 1px 描边 + 圆角 6，图 contain */
.lr-thumb {
  flex: none;
  width: 80px;
  height: 45px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-g3);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-6);
  overflow: hidden;
}
.lr-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
/* 格式图标档（fla 等 shell 图标）：50% 居中不撑满（2026-08-30 用户实机，FileCard icon 档同口径） */
.lr-thumb-img.icon {
  width: 50%;
  height: 50%;
}

/* 名称列：flex 1 gap 12，正文-加粗 一级截断 + 徽章 14 */
.lr-name-cell {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-12);
}
.lr-name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  /* 名称区悬停文本光标（双击可改名） */
  cursor: text;
}
.lr-linked {
  color: var(--n-l3);
}
/* 命名输入：透明底不单独描边（整行品牌环提示）；覆盖共享基座 = 名称档（正文-加粗 13/18 一级） */
.rename-input {
  flex: 1;
  min-width: 0;
  height: 18px;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  text-align: left;
}
/* 命名态整行品牌描边（复用 .kc-sel 选中环层，opacity 恒亮） */
.lr-card.naming::before {
  opacity: 1;
}

/* 数据列：123 宽，数字-大档 亮色/三级 */
.lr-col {
  flex: none;
  width: 123px;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-6);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
}
</style>
