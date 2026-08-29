<script setup lang="ts">
// 文件夹卡片——docs/UI 重构/Design Token/组件/卡片/文件夹卡片.md：
// 通用三级底 + 立体感-通用 + 圆角 8 + padding 12；hover 仅底升通用六级（稿只给 background-color，
// 无位移无阴影——与媒体卡不同）；选中 2px 品牌描边（cards.css .kc-sel）。
// 顶行 = 名称（标题档 一级，截断）+ 文件夹-大双色调图标 24——**图标恒默认色不接 tint**
//（2026-08-29 用户定：卡片变色整屏噪声太大，自定义色只留树容器）；
// 简介 = 两行统计（数字档 亮色/三级，段间 " / " 分隔 gap 4）；名称行与简介间距 36（稿值，填满 122 高链）。
import TwoToneIcon from "../items/TwoToneIcon.vue";
import "./cards.css";

withDefaults(
  defineProps<{
    name: string;
    lines?: string[][]; // 统计行（每行若干段，段间 " / "）
    selected?: boolean;
    forceState?: "" | "hover" | "selected";
  }>(),
  { lines: () => [], selected: false, forceState: "" },
);
</script>

<template>
  <div class="kc-sel folder-card" :class="[{ selected }, forceState ? `force-${forceState}` : '']">
    <div class="fd-head">
      <span class="fd-name">{{ name }}</span>
      <TwoToneIcon name="folder-large" :size="24" />
    </div>
    <div class="fd-stats">
      <div v-for="(line, i) in lines" :key="i" class="fd-line">
        <template v-for="(seg, j) in line" :key="j">
          <span v-if="j > 0" class="fd-sep">/</span>
          <span>{{ seg }}</span>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.folder-card {
  box-sizing: border-box;
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12);
  gap: var(--n-space-36);
  background: var(--n-g3);
  box-shadow: var(--n-effect-raised);
  border-radius: var(--n-radius-8);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.folder-card:hover,
.folder-card.force-hover {
  background: var(--n-g6);
}

.fd-head {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-8);
}
.fd-name {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 简介统计：两行 gap 4，行内段间 " / " gap 4，数字档 亮色/三级 */
.fd-stats {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
.fd-line {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-4);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
