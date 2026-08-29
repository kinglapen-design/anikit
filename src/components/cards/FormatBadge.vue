<script setup lang="ts">
// 格式角标——docs/UI 重构/Design Token/组件/卡片/格式角标.md（简化为统一样式）：
// 卡上（左上 12/12）与列表行内同一件——kc-float 铬（半透明-2 + 立体感-通用 + 背景模糊-小）；
// 文案 = 数字档（Fira Mono）12/16 亮色/二级，**字重 700**（2026-08-29 用户定中文加粗——
// Fira Mono 仅内嵌 Regular/Medium，拉丁/数字 700 仍落 Medium 视觉不变，CJK 回退雅黑吃 Bold）。
import "./cards.css";

defineProps<{ label: string }>();
</script>

<template>
  <span class="kc-float fmt-badge">{{ label }}</span>
</template>

<style scoped>
.fmt-badge {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-bold); /* 用户定中文加粗（Fira Mono 无 Bold 内嵌，拉丁/数字仍落 Medium） */
  color: var(--n-l2);
  white-space: nowrap;
}
</style>
