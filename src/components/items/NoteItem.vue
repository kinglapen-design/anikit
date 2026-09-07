<script setup lang="ts">
// 笔记 item——docs/06_UI重构/Design Token/组件/item/笔记 item/
// 24 高 / padding 4 6 / gap 6 / 圆角 6；时间码 = 数字档 亮色/二级，文本 = 次要文字 亮色/三级；
// 关闭钮常态不显示，hover 从右缘向左滑入并把文本推左（items.css .ni-slide 推挤机制）；hover = 半透明-3，选中 = 半透明-2 + 文本升二级。
import "./items.css";
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    time: string; // 时间码（帧号 / 时长文本，如 "0123:"）
    text: string; // 笔记摘要（单行省略）
    selected?: boolean;
    forceState?: "" | "hover" | "selected"; // 预览页强制态
  }>(),
  { selected: false, forceState: "" },
);

const emit = defineEmits<{ click: []; close: [] }>();
</script>

<template>
  <div
    class="note ni-hover"
    :class="[{ selected }, forceState ? `force-${forceState}` : '']"
    @click="emit('click')"
  >
    <span class="tc">{{ time }}</span>
    <span class="txt">{{ text }}</span>
    <span class="ni-slide">
      <span class="ni-slide-inner">
        <button class="ni-close" @click.stop="emit('close')">
          <AppIcon name="close-small" :size="14" />
        </button>
      </span>
    </span>
  </div>
</template>

<style scoped>
.note {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 24px;
  padding: var(--n-space-4) var(--n-space-6);
  border-radius: var(--n-radius-6);
  cursor: pointer;
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
.note:hover,
.note.force-hover {
  background: var(--n-l-a3);
}
.note.selected,
.note.force-selected {
  background: var(--n-l-a2);
}
.tc {
  flex: none;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l2);
}
.txt {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.note.selected .txt,
.note.force-selected .txt {
  color: var(--n-l2);
}
</style>
