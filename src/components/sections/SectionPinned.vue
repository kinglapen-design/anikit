<script setup lang="ts">
// pinned 缩略图 / 档案封面区块（7.1 二十三轮贴稿重构，稿 = 右侧边栏-详情面板-缩略图 / 档案封面）：
// 恒顶（不参与 tab 切换）、无区块标题、无 border-top（详情面板首块）。
// 结构 = 缩略图容器 240×240（通用二级底 + 通用三级 1px / 圆角 8；图 contain 不裁剪——速查表口径）
//   + 格式徽标 FormatBadge（左上 12/12；静态图不出 = formatBadgeColor null 判定沿用）
//   + 颜色列表 ColorSwatchItem（真色 = detail.colors 前 7，空 = 7 占位块恒显——6.1 口径）。
// 名称 / 链接输入框已下移基础信息 tab 输入框区块（旧 pinned 四件口径作废）。
// 档案形态：封面容器同款 240×240（封面 cover 充满——稿裁切语义）+ 徽标 FormatBadge（badge.text）；
//   hover = 纯黑压暗 50% + set-cover 图标居中 二级（稿 prose 1_hover-封面图）；
//   点击 = 内联展开封面设置组（CoverSettings，6.1 业务不变；设置组贴稿随本轮档案形态阶段）。
//   档案无颜色列表（稿未画）。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import CoverSettings from "./CoverSettings.vue";
import FormatBadge from "../cards/FormatBadge.vue";
import ColorSwatchItem from "../items/ColorSwatchItem.vue";
import AppIcon from "../AppIcon.vue";
import { formatBadgeColor, isSequence } from "../../lib/formatBadge";
import type { DetailTarget } from "../../lib/detailSections";
import { useContentStore } from "../../stores/content";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
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
// 格式徽标：静态图 / 无扩展名映射返回 null = 不出徽标（统一铬后仅留此判定，速查表 §16）
const badgeLabel = computed(() => {
  if (!file.value) return null;
  const p = file.value.placement;
  return formatBadgeColor(p.ext, isSequence(p), p.animated) ? p.ext.toUpperCase() : null;
});
// 颜色列表真色（6.1 颜色解析）：detail.colors 前 7（透明像素不计，存 10 显 7）；
// 空（非图片 / 无缩略图）= 7 个占位色块（恒显口径）
const swatchColors = computed(() => (file.value?.detail.colors ?? []).slice(0, 7));
</script>

<template>
  <section class="section-pinned">
    <!-- 封面设置编辑态（6.1，43 稿；仅档案）：整组替换（组件自带 padding / gap） -->
    <CoverSettings
      v-if="archive && coverEditing"
      :node="archive.node"
      :data="archive.data"
      @done="coverEditing = false"
    />
    <template v-else>
      <!-- 缩略图 / 封面容器（稿：240×240 / 通用二级底 + 通用三级 1px / 圆角 8） -->
      <div
        class="thumb"
        :class="{ cover: !!archive }"
        v-tip="archive ? '封面设置' : ''"
        @click="archive && (coverEditing = true)"
      >
        <template v-if="file">
          <img v-if="thumbUrl" :src="thumbUrl" alt="" draggable="false" class="fit-contain" />
          <!-- 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 -->
          <span v-else class="ext">{{ file.placement.ext.toUpperCase() }}</span>
          <FormatBadge v-if="badgeLabel" class="badge" :label="badgeLabel" />
        </template>
        <template v-else-if="archive">
          <img v-if="archiveCover" :src="archiveCover" alt="" draggable="false" class="fit-cover" />
          <span v-else class="ext" />
          <FormatBadge v-if="archive.data.badge" class="badge" :label="archive.data.badge.text" />
          <!-- hover 封面 = 纯黑压暗 50% + 设为封面图标居中（稿 prose） -->
          <span class="cover-hover" aria-hidden="true">
            <AppIcon name="set-cover" :size="18" />
          </span>
        </template>
      </div>
      <!-- 颜色列表（稿：透明壳横排；仅文件形态） -->
      <ColorSwatchItem v-if="file" :colors="swatchColors" :placeholder-count="7" />
    </template>
  </section>
</template>

<style scoped>
/* pinned 区块（稿：padding 12 / gap 12 纵列；无 border-top——详情面板首块） */
.section-pinned {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  flex: none;
}
/* 缩略图 / 封面容器（稿：240×240 定方，宽随栏 = 100%；通用二级底 + 通用三级 1px / 圆角 8） */
.thumb {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  border-radius: var(--n-radius-8);
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  overflow: hidden;
  flex: none;
}
.thumb img {
  width: 100%;
  height: 100%;
  display: block;
  border-radius: var(--n-radius-8); /* 铺满图自带圆角（圆角平滑三件套口径，速查表 §7） */
}
/* 文件缩略图 = contain 不裁剪；档案封面 = cover 充满（稿裁切语义） */
.fit-contain {
  object-fit: contain;
}
.fit-cover {
  object-fit: cover;
}
/* 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 */
.thumb .ext {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #c6c7cc;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-g3);
}
/* 格式 / 档案徽标（稿：左上 12/12；FormatBadge = kc-float 统一铬 + 数字档 12/16 二级） */
.badge {
  position: absolute;
  top: var(--n-space-12);
  left: var(--n-space-12);
  pointer-events: none;
}
/* 档案封面 = 封面设置入口（6.1，43 稿）：点击进入编辑态 */
.thumb.cover {
  cursor: pointer;
}
/* hover 封面 = 纯黑压暗 50% + set-cover 图标居中 二级（稿 prose 1_hover-封面图） */
.cover-hover {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  color: var(--n-l2);
  opacity: 0;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
  pointer-events: none;
}
.thumb.cover:hover .cover-hover {
  opacity: 1;
}
</style>
