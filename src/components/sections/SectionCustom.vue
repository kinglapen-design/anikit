<script setup lang="ts">
// 区块 · 自定义数据（3.2 一轮，pinned 样式：无副标题、置顶观感；排序/显隐在 22 稿编辑面板）。
// 一轮只读：输入框 readonly、添加 / 打开链接 / 编辑描述一律 disabled 占位（编辑态归二轮）。
// 布局（figma 数据 item 稿，2026-08-15 走查贴稿）：缩略图（contain 不裁剪 / C2 底 / 圆角 6 /
// 1px border / 高按图比例上限 220，徽标 6,6）→ 颜色 bar 恒显（§6：C4 胶囊圆角 14 居中
// fit-content，7 个占位色块 16×16 圆形透明底 + 亮-选中 1px 描边——色块业务 6.1 接入后替换真色）
// → 名称输入框 → 链接输入框（placeholder http://，首条链接）+ open-link → 属性 chips 行恒显
// （AppTag readonly + 行尾 + 钮）→ 描述行 → 通栏「编辑描述」按钮（C4 / 38 高 / 圆角 6 / 带图标）。
// 档案形态：封面占位（头像三轮）+ 徽标 → 名称输入框 → 属性 chips（props）→ 描述列表 → 编辑描述；
// 颜色 bar / 链接输入框是资产数据，档案不出。
// .field / .swatches 样式回收自旧 DetailColors（2026-07-23 用户定稿）。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppTag from "../AppTag.vue";
import AppButton from "../AppButton.vue";
import IconButton from "../IconButton.vue";
import { displayName, formatBadgeColor, isSequence } from "../../lib/formatBadge";
import type { DetailTarget } from "../../lib/detailSections";
import { useUiStore } from "../../stores/ui";

const props = defineProps<{ target: DetailTarget; title?: string }>();
const ui = useUiStore();

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));

const thumbUrl = computed(() => {
  const p = file.value?.placement.thumbPath;
  return p ? convertFileSrc(p) : null;
});
// 缩略图高按图比例（aspect-ratio），无尺寸回退 4:3；上限 220 由 CSS max-height 控制
const thumbStyle = computed(() => {
  const p = file.value?.placement;
  return { "--ar": p?.w && p?.h ? `${p.w} / ${p.h}` : "4 / 3" };
});
// 格式徽标底色：静态图 / 无扩展名映射返回 null = 不出徽标
const badgeColor = computed(() => {
  if (!file.value) return null;
  const p = file.value.placement;
  return formatBadgeColor(p.ext, isSequence(p), p.animated);
});
const name = computed(() => {
  if (file.value) return displayName(file.value.placement, ui.showExt);
  return archive.value?.node.name ?? "";
});
// 链接输入框：有 links 填首条，无则空（placeholder http://）
const firstLink = computed(() => file.value?.detail.links[0] ?? "");
</script>

<template>
  <section class="section-custom detail-block pinned">
    <template v-if="file">
      <!-- 缩略图：contain 完整不裁剪（2026-08-15 走查定）；无图 = 浅色块 #C6C7CC + 扩展名 -->
      <div class="thumb" :style="thumbStyle">
        <img v-if="thumbUrl" :src="thumbUrl" alt="" draggable="false" />
        <span v-else class="ext">{{ file.placement.ext.toUpperCase() }}</span>
        <span v-if="badgeColor" class="fmt-badge" :style="{ background: badgeColor }">
          {{ file.placement.ext.toUpperCase() }}
        </span>
      </div>
      <!-- 颜色 bar（恒显）：7 个占位色块——色块业务 6.1 未做，真色接入后替换 -->
      <div class="swatches">
        <span v-for="i in 7" :key="i" class="swatch" />
      </div>
      <!-- 名称输入框（只读，值 = 显示名） -->
      <div class="field">
        <input type="text" :value="name" readonly />
      </div>
      <!-- 链接输入框（只读，placeholder http://）+ 打开链接（disabled 占位） -->
      <div class="field with-btn">
        <input type="text" :value="firstLink" placeholder="http://" readonly />
        <IconButton name="open-link" title="打开链接" disabled />
      </div>
      <!-- 属性 chips 行（恒显）+ 行尾添加钮（disabled，二轮编辑业务） -->
      <div class="chips">
        <AppTag v-for="t in file.detail.tags" :key="t" :label="t" readonly />
        <IconButton name="add-small" small disabled title="添加属性" class="chip-add" />
      </div>
      <!-- 描述行（note） -->
      <div v-if="file.detail.note" class="field desc">
        <span class="field-text">{{ file.detail.note }}</span>
      </div>
      <!-- 编辑描述（通栏恒显，disabled 占位，二轮编辑业务） -->
      <AppButton class="edit-desc" variant="panel" icon="edit-description" label="编辑描述" disabled />
    </template>
    <template v-else-if="archive">
      <!-- 封面占位：头像业务三轮，一轮浅色块占位（同无图口径，无扩展名文字） -->
      <div class="thumb">
        <span class="ext" />
        <span v-if="archive.data.badge" class="fmt-badge" :style="{ background: archive.data.badge.color }">
          {{ archive.data.badge.text }}
        </span>
      </div>
      <div class="field">
        <input type="text" :value="archive.node.name" readonly />
      </div>
      <div class="chips">
        <AppTag v-for="t in archive.data.props" :key="t" :label="t" readonly />
        <IconButton name="add-small" small disabled title="添加属性" class="chip-add" />
      </div>
      <div v-for="d in archive.data.descriptions" :key="d.title" class="desc-item">
        <span class="desc-title">{{ d.title }}</span>
        <span class="desc-content">{{ d.content }}</span>
      </div>
      <AppButton class="edit-desc" variant="panel" icon="edit-description" label="编辑描述" disabled />
    </template>
  </section>
</template>

<style scoped>
.section-custom {
  display: flex;
  flex-direction: column;
}
/* 缩略图：contain 不裁剪 / C2 底 / 圆角 6 / 1px border；高按图比例（--ar），上限 220 */
.thumb {
  position: relative;
  width: 100%;
  aspect-ratio: var(--ar, 4 / 3);
  max-height: 220px;
  border-radius: var(--radius-6);
  background: var(--c2);
  box-shadow: inset 0 0 0 1px var(--border-1);
  overflow: hidden;
  flex: none;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}
/* 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 */
.thumb .ext {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #c6c7cc;
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  font-weight: var(--text-title-weight);
  color: var(--c3);
}
/* 格式徽标 / 档案徽标芯片：左上 6,6 / 内边距 0 4 / 圆角 4 / 徽标档 10/14 700 常态色 */
.fmt-badge {
  position: absolute;
  top: 6px;
  left: 6px;
  max-width: calc(100% - 12px);
  padding: 0 var(--space-4);
  border-radius: var(--radius-4);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
}
/* 颜色 bar（§6）：C4 胶囊圆角 14 / padding 4 / gap 4 / 居中 fit-content / 1px 暗-悬停 */
.swatches {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-4);
  width: fit-content;
  align-self: center;
  border-radius: 14px;
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
/* 占位色块：16×16 圆形，透明底 + 亮-选中 1px 描边（稿的描边样式；6.1 真色接入后替换） */
.swatch {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1px solid var(--line-active);
  flex: none;
}
/* 输入框（03_输入框 01，回收旧 DetailColors 定稿）：32 高 / 内边距 6 8 / 圆角 8 / C4 / 1px border-1 */
.field {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: 32px;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
/* 带链接按钮（03_输入框 02）：内边距 2 2 2 8 */
.field.with-btn {
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-8);
}
.field input {
  flex: 1;
  min-width: 0;
  color: var(--text-highlight);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.field input::placeholder {
  color: var(--text-faint);
}
/* 描述行：多行自适应（32 下限），只读 */
.field.desc {
  height: auto;
  min-height: 32px;
  align-items: flex-start;
}
.field-text {
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  overflow-wrap: anywhere;
}
/* 属性 chips 行（恒显）：流式换行，间距 8，行尾添加钮 */
.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-8);
}
/* 编辑描述通栏钮：38 高（稿值；AppButton panel = C4 / 圆角 6 / 带图标，disabled 占位） */
.section-custom .edit-desc {
  width: 100%;
  height: 38px;
}
/* 档案描述列表：标题（标签档）+ 内容（常态） */
.desc-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.desc-title {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
}
.desc-content {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
</style>
