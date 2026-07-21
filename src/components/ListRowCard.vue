<script setup lang="ts">
// 列表行（02_通用组件/02_item 10）：缩略图（可随滑块缩放）+ 名称 + 尺寸 / 格式 / 文件大小 / 修改日期。
// C3 / C2 交替行底色；1px C4-7（inset）/ 圆角 8 / 内边距 6 8。
import { computed } from "vue";
import type { Box } from "../lib/layouts";
import type { FakeAsset } from "../stores/content";
import { useUiStore } from "../stores/ui";

const props = withDefaults(
  defineProps<{ asset: FakeAsset; box: Box; odd?: boolean; selected?: boolean }>(),
  { odd: false, selected: false },
);
const ui = useUiStore();

// 缩略图随滑块缩放（80×45 基准 × 行高 / 220），等高不等宽限定容器内不裁剪
const thumbW = computed(() => Math.max(30, Math.round((80 * ui.targetRowHeight) / 220)));
const thumbH = computed(() => Math.max(17, Math.round((45 * ui.targetRowHeight) / 220)));
const emit = defineEmits<{ select: [] }>();
void emit;
void props;
</script>

<template>
  <div
    class="list-row"
    :class="{ odd, selected }"
    :style="{ left: `${box.left}px`, top: `${box.top}px`, width: `${box.width}px`, height: `${box.height}px` }"
    @click="emit('select')"
  >
    <div class="main">
      <div class="thumb" :style="{ width: `${thumbW}px`, height: `${thumbH}px`, background: asset.color }" />
      <span class="name">{{ asset.name }}</span>
    </div>
    <div class="details">
      <span class="col size">{{ asset.w }} x {{ asset.h }}</span>
      <span class="col format">{{ asset.format.toUpperCase() }}</span>
      <span class="col filesize">123 MB</span>
      <span class="col date">2026/07/12 14:26</span>
    </div>
  </div>
</template>

<style scoped>
.list-row {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c2);
  box-shadow: inset 0 0 0 1px var(--c4-7);
  cursor: default;
}
/* C3 / C2 交替行底色 */
.list-row.odd {
  background: var(--c3);
}
.list-row:hover {
  background: var(--c4-7);
}
.list-row.selected {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
/* 标题容器：缩略图 + 名称，间距 8 */
.main {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
}
.thumb {
  flex: none;
  border-radius: var(--radius-4);
  box-shadow: inset 0 0 0 1px var(--c4-15);
}
.name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 详情容器：尺寸 / 格式 / 文件大小 / 修改日期，正文 C4-40，固定列宽与列头对齐 */
.details {
  display: flex;
  align-items: center;
  gap: var(--space-16);
  flex: none;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4-40);
}
.col {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.size {
  width: 90px;
}
.format {
  width: 50px;
}
.filesize {
  width: 70px;
}
.date {
  width: 130px;
}
</style>
