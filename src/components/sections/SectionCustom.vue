<script setup lang="ts">
// 区块 · 自定义数据（3.2 一轮，pinned 无副标题；排序/显隐在 22 稿编辑面板）。
// 权威稿 = figma-css 02_布局/01_主窗口.md「缩略图 - 颜色信息」组 +「自定义数据」块
//（2026-08-15 走查贴稿；旧 DetailColors 回收样式与 pinned padding 8 16 12 16 旧口径均作废）：
// 组1「缩略图 - 颜色信息」= padding 16 / gap 8 / align center——缩略图框全宽 contain /
// C2 底 / 1px 暗-悬停 / 圆角 8（高按图比例上限 220），格式徽标 absolute 7,7 / padding 2 4 /
// 圆角 4 / 10/14 700 常态色；颜色 bar 恒显（C4 胶囊圆角 14 / padding 4 / gap 4，
// 7 个占位色块 16×16 圆形透明底 + 亮-选中 1px 描边——色块业务 6.1 接入后替换真色）。
// 组2「自定义数据」= padding 12 16 / gap 12 / border-top 1px 暗-悬停——名称输入框（36 高 /
// C2 / padding 8 12 / 圆角 6 / 14/20 常态）+ 链接输入框（36 高 / C2 / padding 3 3 3 12 /
// 文字极淡 + 打开链接钮 30×30 图标 18 disabled）+ 属性标签行（gap 6；chip 高 25 / C4 /
// padding 4 8 / 圆角 6 / 12/17 标签色 + 行尾添加钮 disabled）+ 描述组（纯文本行 gap 8：
// 标签: 标签色 + 内容 常态，行内 gap 4）+ 编辑描述按钮（C5 / 36 高 / 圆角 6 / 图标 18 常态）。
// 一轮只读：输入框 readonly、添加 / 打开链接 / 编辑描述一律 disabled 占位（编辑态归二轮）。
// 档案形态：封面占位（头像三轮）+ 徽标 → 名称输入框 → 属性 chips（props）→ 描述列表 → 编辑描述；
// 颜色 bar / 链接输入框是资产数据，档案不出。
// detail-block 公共规则与稿冲突处以稿为准（本组件内覆盖 root padding/gap，不动其他区块）。
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
      <!-- 组1「缩略图 - 颜色信息」（pinned 无副标题） -->
      <div class="thumb-group">
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
      </div>
      <!-- 组2「自定义数据」（border-top 1px 暗-悬停） -->
      <div class="custom-body">
        <div class="field">
          <input type="text" :value="name" readonly />
        </div>
        <div class="field link">
          <input type="text" :value="firstLink" placeholder="http://" readonly />
          <IconButton name="open-link" title="打开链接" disabled />
        </div>
        <div class="chips">
          <AppTag v-for="t in file.detail.tags" :key="t" :label="t" readonly />
          <IconButton name="add-small" small disabled title="添加属性" class="chip-add" />
        </div>
        <div v-if="file.detail.note" class="desc-group">
          <div class="desc-row">
            <span class="d-content">{{ file.detail.note }}</span>
          </div>
        </div>
        <AppButton class="edit-desc" variant="sidebar" icon="edit-description" label="编辑描述" disabled />
      </div>
    </template>
    <template v-else-if="archive">
      <div class="thumb-group">
        <!-- 封面占位：头像业务三轮，一轮浅色块占位（同无图口径，无扩展名文字） -->
        <div class="thumb">
          <span class="ext" />
          <span v-if="archive.data.badge" class="fmt-badge" :style="{ background: archive.data.badge.color }">
            {{ archive.data.badge.text }}
          </span>
        </div>
      </div>
      <div class="custom-body">
        <div class="field">
          <input type="text" :value="archive.node.name" readonly />
        </div>
        <div class="chips">
          <AppTag v-for="t in archive.data.props" :key="t" :label="t" readonly />
          <IconButton name="add-small" small disabled title="添加属性" class="chip-add" />
        </div>
        <div v-if="archive.data.descriptions.length > 0" class="desc-group">
          <div v-for="d in archive.data.descriptions" :key="d.title" class="desc-row">
            <span class="d-label">{{ d.title }}:</span>
            <span class="d-content">{{ d.content }}</span>
          </div>
        </div>
        <AppButton class="edit-desc" variant="sidebar" icon="edit-description" label="编辑描述" disabled />
      </div>
    </template>
  </section>
</template>

<style scoped>
/* root：detail-block 公共规则与稿冲突处以稿为准——padding/gap 归零，两段组自带间距
   （叠类提权压过 DetailPanel 的 .detail-panel .detail-block.pinned 规则） */
.section-custom.detail-block.pinned {
  padding: 0;
  gap: 0;
}
.section-custom {
  display: flex;
  flex-direction: column;
}
/* 组1「缩略图 - 颜色信息」（稿）：padding 16 / gap 8 / align center */
.thumb-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-16);
}
/* 组2「自定义数据」（稿）：padding 12 16 / gap 12 / border-top 1px 暗-悬停 */
.custom-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-16);
  border-top: 1px solid var(--line-hover);
}
/* 缩略图框（稿）：组内全宽 / C2 底 / 1px 暗-悬停 / 圆角 8；contain 不裁剪，
   高按图比例（--ar），上限 220 */
.thumb {
  position: relative;
  width: 100%;
  aspect-ratio: var(--ar, 4 / 3);
  max-height: 220px;
  border-radius: var(--radius-8);
  background: var(--c2);
  box-shadow: inset 0 0 0 1px var(--line-hover);
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
/* 格式徽标 / 档案徽标芯片（稿）：absolute 7,7 / padding 2 4 / 圆角 4 / 徽标档 10/14 700 常态色 */
.fmt-badge {
  position: absolute;
  top: 7px;
  left: 7px;
  max-width: calc(100% - 14px);
  padding: 2px var(--space-4);
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
/* 颜色 bar（稿 §6）：C4 胶囊圆角 14 / padding 4 / gap 4 / 1px 暗-悬停 */
.swatches {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-4);
  width: fit-content;
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
/* 名称输入框（稿）：36 高 / C2 / padding 8 12 / 圆角 6 / 1px 暗-悬停 / 文字 14/20 常态 */
.field {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: 36px;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c2);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
/* 链接输入框（稿）：36 高 / C2 / padding 3 3 3 12 / 文字极淡 + 右侧打开链接钮（30×30 图标 18） */
.field.link {
  padding: 3px 3px 3px var(--space-12);
}
.field input {
  flex: 1;
  min-width: 0;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.field.link input,
.field.link input::placeholder {
  color: var(--text-faint);
}
/* 属性标签行（稿）：gap 6；chip 高 25 / C4 / padding 4 8 / 圆角 6 / 文字 12/17 标签色
   （AppTag 默认常态色，本区域按稿覆盖为标签色）+ 行尾添加钮 20×20 */
.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-6);
}
.chips .app-tag {
  color: var(--text-label);
}
/* 描述组（稿）：纯文本行 gap 8；行 = 标签: 标签色 + 内容 常态，行内 gap 4 */
.desc-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
.desc-row {
  display: flex;
  gap: var(--space-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.d-label {
  color: var(--text-label);
  white-space: nowrap;
  flex: none;
}
.d-content {
  color: var(--text-normal);
  min-width: 0;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
/* 编辑描述按钮（稿）：通栏 36 高 / C5（AppButton sidebar）/ 圆角 6 / 图标 18 + 14/20 常态 */
.section-custom .edit-desc {
  width: 100%;
  height: 36px;
}
</style>
