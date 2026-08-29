<script setup lang="ts">
// 档案卡片——docs/UI 重构/Design Token/组件/卡片/档案卡片.md：
// 正方形比例（稿注明确，缩略图网格填充 = 封面 cover 充满）；渐变罩三行 = 名称（标题档 一级）/
// 属性标签行（数字档 二级，" | " 分隔）/ 文件数量行（" / " 分隔）；
// 左上 = 格式角标统一样式（档案 = 属性文字）；右下 = 链接文件-徽章 14 亮色/二级；
// 壳 / hover 位移 -4 + 小阴影 / 选中 2px 品牌描边 见 cards.css（kc-media / kc-sel）。
import AppIcon from "../AppIcon.vue";
import FormatBadge from "./FormatBadge.vue";
import "./cards.css";

withDefaults(
  defineProps<{
    name: string;
    coverUrl?: string; // 封面图（网格填充合成，业务侧给）
    badge?: string; // 左上角标（档案 = 属性文字，如「主角」）
    attrs?: string[]; // 属性标签行（段间 " | "）
    stats?: string[]; // 文件数量行（段间 " / "）
    linkedBadge?: boolean; // 链接文件-徽章（右下 12/12）
    selected?: boolean;
    forceState?: "" | "hover" | "selected";
  }>(),
  {
    coverUrl: "",
    badge: "",
    attrs: () => [],
    stats: () => [],
    linkedBadge: false,
    selected: false,
    forceState: "",
  },
);
</script>

<template>
  <div
    class="kc-media kc-bordered kc-sel ar-card"
    :class="[{ selected }, forceState ? `force-${forceState}` : '']"
  >
    <div class="kc-clip">
      <img v-if="coverUrl" :src="coverUrl" class="ar-cover" alt="" />
      <div class="kc-overlay">
        <span class="ar-name">{{ name }}</span>
        <span v-if="attrs.length" class="ar-row">
          <template v-for="(a, i) in attrs" :key="i">
            <span v-if="i > 0" class="ar-sep">|</span>
            <span>{{ a }}</span>
          </template>
        </span>
        <span v-if="stats.length" class="ar-row">
          <template v-for="(s, i) in stats" :key="i">
            <span v-if="i > 0" class="ar-sep">/</span>
            <span>{{ s }}</span>
          </template>
        </span>
      </div>
    </div>

    <FormatBadge v-if="badge" class="ar-badge" :label="badge" />
    <AppIcon
      v-if="linkedBadge"
      name="linked-file-badge"
      :size="14"
      class="kc-corner-badge ar-linked"
    />
  </div>
</template>

<style scoped>
.ar-card {
  width: 100%;
  height: auto;
  aspect-ratio: 1 / 1; /* 稿注：保持正方形比例 */
}
.ar-cover {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.ar-badge {
  position: absolute;
  left: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 3;
}
.ar-linked {
  color: var(--n-l2);
}

/* 名称 = 标题档 一级；属性 / 数量行 = 数字档 二级，分隔符同行 */
.ar-name {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ar-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-4);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
}
</style>
