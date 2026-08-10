<script setup lang="ts">
// 详情模块 · 颜色信息（03_UI 布局/04_右侧边栏 03，2026-07-23 用户定重组）：
// 永久置顶（pinned，不参与拖拽排序）、无副标题；
// 预览图 + 颜色 item 一容器（间距 8）｜标题 / 注释 / 链接输入框一容器（间距 8，内边距 0）。
import IconButton from "./IconButton.vue";

// 假色板：贴近参考图的 10 色
const colors = ["#3A6EA5", "#2B9FD9", "#7A5C3E", "#A53E3E", "#3E8A57", "#6B4F8A", "#C0A23E", "#4E4E52", "#8A8D99", "#1D1D21"];
</script>

<template>
  <section class="detail-colors detail-block pinned">
    <div class="preview-group">
      <div class="thumb">
        <span class="badge">PSD</span>
      </div>
      <div class="swatches">
        <span v-for="c in colors" :key="c" class="swatch" :style="{ background: c }" v-tip="c" />
      </div>
    </div>
    <div class="inputs-group">
      <div class="field">
        <input type="text" value="中继站场景" placeholder="输入标题" readonly />
      </div>
      <div class="field">
        <input type="text" placeholder="输入注释" readonly />
      </div>
      <div class="field with-btn">
        <input type="text" placeholder="http://" readonly />
        <IconButton name="open-link" title="打开链接" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.detail-colors {
  display: flex;
  flex-direction: column;
}
/* 预览图 + 颜色 item 容器：间距 8；色块容器居中（2026-07-23 用户定） */
.preview-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-8);
}
/* 输入框组容器：间距 8 / 内边距 0 */
.inputs-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: 0;
}
/* 图片容器：高 120 / 圆角 6 / 1px border-1 */
.thumb {
  position: relative;
  width: 100%;
  height: 120px;
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--border-1);
  background: hsl(215 30% 22%);
}
/* 格式角标：左上 4px / 内边距 0 4 / 圆角 4 / 深色实底（同素材卡片降噪定案）/ 角标文字 10/16 粗体 60% */
.badge {
  position: absolute;
  top: 4px;
  left: 4px;
  padding: 0 var(--space-4);
  border-radius: var(--radius-4);
  background: var(--custom-blue);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--font-icon-3);
  pointer-events: none;
}
/* 色块容器（02_item 08）：胶囊圆角 14 / C4 / 1px border-1 / 内边距 4 / 间距 4 /
   宽度按子项自适应（fit-content，2026-07-23 用户定，原填充宽度右侧多余） */
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-4);
  padding: var(--space-4);
  width: fit-content;
  border-radius: 14px;
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.swatch {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  /* 描边 12% 透明白（font-icon-4）：透下去与色块自身颜色叠加，不用实灰（发糊，2026-07-23 用户定） */
  box-shadow: inset 0 0 0 1px var(--font-icon-4);
  flex: none;
}
.swatch:hover {
  box-shadow: inset 0 0 0 1px var(--font-icon-2);
}
/* 输入框（03_输入框 01）：内边距 6 8 / 圆角 8 / C4 / 1px border-1；聚焦边框品牌色-1 */
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
.field:focus-within {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
.field input {
  flex: 1;
  min-width: 0;
  color: var(--font-icon-1);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.field input::placeholder {
  color: var(--font-icon-placeholder);
}
</style>
