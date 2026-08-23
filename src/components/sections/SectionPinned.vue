<script setup lang="ts">
// 缩略图区块（2026-08-15 定案：独立永远置顶——不可隐藏、不参与排序、不进 22 稿编辑面板，
// 由 DetailPanel 固定渲染在排序区块列表之外，无区块标题、不可折叠）。
// 权威稿 = figma-css 02_布局/01_主窗口.md「缩略图 - 颜色信息」组：容器 padding 16 / gap 12 /
// align center——缩略图框全宽 contain / C2 底 / 1px 暗-悬停 / 圆角 8（高按图比例上限 220，
// 无尺寸回退 4:3），格式徽标 absolute 7,7 / padding 2 4 / 圆角 4 / 10/14 700 常态色；
// 颜色 bar 恒显（C4 胶囊圆角 14 / padding 4 / gap 4，7 个占位色块 16×16 圆形透明底 +
// 亮-选中 1px 描边——色块业务 6.1 接入后替换真色）；名称输入框（36 高 / C2 / padding 8 12 /
// 圆角 6 / 14/20 常态）+ 链接输入框（36 高 / C2 / padding 3 3 3 12 / 文字极淡 +
// 打开链接钮 30×30 图标 18 disabled）——2026-08-15 定案从自定义数据段上移入本组。
// 一轮只读：输入框 readonly、打开链接 disabled 占位。
// 档案形态（2026-08-15 任务 A 三项，与文件详情一模一样只无颜色 bar）：封面 + 徽标 chip +
// 名称输入框 + 链接输入框（数据 = archives.json links，http:// + open-link disabled）。
// 封面（6.1，43 稿）：自定义封面（covers/ 烘培图）优先，回退子项首图（folderThumbMap）；
// 常态只有封面图，点击封面展开封面设置组（CoverSettings：取景 / 更换封面 / 徽标 / 颜色），
// 确认 / 取消退回常态——文件不做本面板（2026-08-23 用户定）。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import IconButton from "../IconButton.vue";
import CoverSettings from "./CoverSettings.vue";
import { displayName, formatBadgeColor, isSequence } from "../../lib/formatBadge";
import type { DetailTarget } from "../../lib/detailSections";
import { useContentStore } from "../../stores/content";
import { useUiStore } from "../../stores/ui";

const props = defineProps<{ target: DetailTarget }>();
const ui = useUiStore();
const content = useContentStore();

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));

// 封面设置编辑态（6.1，43 稿；仅档案）：点击封面图展开，确认 / 取消（done）退回常态；
// 切换展示目标（别的档案 / 文件）时复位退出
const coverEditing = ref(false);
watch(
  () => (archive.value ? archive.value.node.id : null),
  () => (coverEditing.value = false),
);

const thumbUrl = computed(() => {
  const p = file.value?.placement.thumbPath;
  return p ? convertFileSrc(p) : null;
});
// 档案封面：自定义封面优先（6.1 封面设置产物 covers/，coverPath 绝对路径）；
// 无自定义 = 子项第一张静态图（folderThumbMap 既有口径）；无图 = 浅色块占位
const archiveCover = computed(() => {
  const id = archive.value?.node.id;
  if (!id) return null;
  const p = archive.value?.data.coverPath ?? content.folderThumbMap.get(id) ?? null;
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
// 链接输入框：文件 = detail.links 首条；档案 = archives.json links 首条；无则空（placeholder http://）
const firstLink = computed(() => {
  if (file.value) return file.value.detail.links[0] ?? "";
  return archive.value?.data.links[0] ?? "";
});
</script>

<template>
  <section class="section-pinned detail-block pinned">
    <!-- 封面设置编辑态（6.1，43 稿；仅档案）：整组替换，脱离 thumb-group（组件自带 padding 16 / gap 12） -->
    <CoverSettings
      v-if="archive && coverEditing"
      :node="archive.node"
      :data="archive.data"
      @done="coverEditing = false"
    />
    <div v-else class="thumb-group">
      <template v-if="file">
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
        <div class="field">
          <input type="text" :value="name" readonly />
        </div>
        <div class="field link">
          <input type="text" :value="firstLink" placeholder="http://" readonly />
          <IconButton name="open-link" title="打开链接" disabled />
        </div>
      </template>
      <template v-else-if="archive">
        <!-- 封面：自定义封面优先 / 子项首图（无图浅色块占位）；点击进入封面设置（43 稿） -->
        <div class="thumb cover-btn" v-tip="'封面设置'" @click="coverEditing = true">
          <img v-if="archiveCover" :src="archiveCover" alt="" draggable="false" />
          <span v-else class="ext" />
          <span v-if="archive.data.badge" class="fmt-badge" :style="{ background: archive.data.badge.color }">
            {{ archive.data.badge.text }}
          </span>
        </div>
        <div class="field">
          <input type="text" :value="archive.node.name" readonly />
        </div>
        <div class="field link">
          <input type="text" :value="firstLink" placeholder="http://" readonly />
          <IconButton name="open-link" title="打开链接" disabled />
        </div>
      </template>
    </div>
  </section>
</template>

<style scoped>
/* root：detail-block 公共规则与稿冲突处以稿为准——padding/gap 归零，组自带间距。
   叠类提权（2026-08-15 实测：单组 (0,3,0) 与 DetailPanel 的 .detail-panel .detail-block.pinned
   平级拼加载序会输，root 残留 pinned padding 8 16 12 → 组内缩、线不通栏；
   叠成双类 (0,4,0) 稳赢，与加载序无关） */
.section-pinned.section-pinned.detail-block.pinned {
  padding: 0;
  gap: 0;
}
.section-pinned {
  display: flex;
  flex-direction: column;
}
/* 「缩略图 - 颜色信息」组（稿 + 2026-08-15 定案）：padding 16 / gap 12 / align center */
.thumb-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-12);
  padding: var(--space-16);
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
/* 档案封面 = 封面设置入口（6.1，43 稿）：常态只有封面图，点击展开封面设置组 */
.thumb.cover-btn {
  cursor: pointer;
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
  width: 100%;
  height: 36px;
  box-sizing: border-box;
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
</style>
