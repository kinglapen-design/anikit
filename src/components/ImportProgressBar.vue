<script setup lang="ts">
// 导入进度条（03_UI 布局/11_导入进度条）：插在中间面板标题栏与卡片容器之间。
// 构造：进度条（胶囊 6px，占剩余空间）+ 标签（正在导入文件 x/y）+ 预计剩余 + 关闭按钮。
// 关闭 = 取消：进度条立即隐藏，后台把当前文件写完后真实停止（cancel 语义，见 11 文档注）。
import { computed } from "vue";
import IconButton from "./IconButton.vue";
import { useContentStore } from "../stores/content";

const content = useContentStore();
const p = computed(() => content.importProgress);

const scanning = computed(() => (p.value ? p.value.total === 0 : false));
const ratio = computed(() => {
  if (!p.value || p.value.total === 0) return 0;
  return Math.min(1, p.value.done / p.value.total);
});
const label = computed(() => {
  if (!p.value) return "";
  return p.value.total === 0
    ? "正在扫描文件…"
    : `正在导入文件 (${p.value.done} / ${p.value.total})`;
});
// 预计剩余：按已导入速率估算（done 变化即重算）
const eta = computed(() => {
  if (!p.value || p.value.done === 0 || p.value.total === 0) return "";
  const elapsed = (Date.now() - p.value.startedAt) / 1000;
  if (elapsed <= 0) return "";
  const remain = ((p.value.total - p.value.done) / p.value.done) * elapsed;
  if (remain < 60) return `预计剩余约 ${Math.max(1, Math.ceil(remain))} 秒`;
  if (remain < 3600) return `预计剩余约 ${Math.ceil(remain / 60)} 分钟`;
  return `预计剩余约 ${Math.floor(remain / 3600)} 小时`;
});
</script>

<template>
  <div v-if="p" class="import-progress">
    <div class="bar" :class="{ scanning }">
      <div class="fill" :style="scanning ? undefined : { width: `${ratio * 100}%` }" />
    </div>
    <span class="label">{{ label }}</span>
    <span v-if="eta" class="eta">{{ eta }}</span>
    <IconButton name="window-close" title="取消导入" @click="content.cancelImport()" />
  </div>
</template>

<style scoped>
/* 容器（11）：padding 8 16（左右 16 与中间面板标题栏 / 网格对齐，2026-07-27 用户定）/ 无底无边 / 子项间距 6 / 高 = 关闭按钮 28 + 16 */
.import-progress {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-16);
  box-sizing: border-box;
  height: 44px;
  flex: none;
}
/* 进度条（10_进度条）：高 6 胶囊 / border-1 底 / 品牌色-1 已加载，占剩余空间 */
.bar {
  position: relative;
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: var(--border-1);
  overflow: hidden;
}
.fill {
  height: 100%;
  border-radius: 3px;
  background: var(--brand-1);
  transition: width 0.2s ease;
}
/* 扫描中（总数未知）：品牌色块往返滑动不定态 */
.bar.scanning .fill {
  position: absolute;
  width: 30%;
  animation: slide 1.2s ease-in-out infinite alternate;
}
@keyframes slide {
  from {
    left: 0;
  }
  to {
    left: 70%;
  }
}
/* 标签与预计时间：副文本 font-icon-3 */
.label,
.eta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
  flex: none;
}
</style>
