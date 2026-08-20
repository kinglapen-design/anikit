<script setup lang="ts">
// 时间显示切换控件（2026-07-28 用户复验定）：视频播放条时间文本悬停出竖排切换弹窗——
// 样式/交互复刻 RateControl 倍率弹窗（容器 C3 / inset border-1 / 圆角 8 / --shadow-pop /
// 内边距 4，底部 4px 桥接防 hover 断开，未选中 font-icon-3 / 选中 font-icon-1）。
// 「帧数」「时间码」二选一，选中即切 ui.timeDisplay（与查看方式菜单"时间显示"同一值，天然联动）。
import { ref } from "vue";
import MenuItem from "./MenuItem.vue";
import { useUiStore } from "../stores/ui";

defineProps<{ text: string }>();
const ui = useUiStore();
const open = ref(false);

const MODES = [
  { value: "frames", label: "帧数" },
  { value: "timecode", label: "时间码" },
] as const;

function pick(m: "frames" | "timecode"): void {
  ui.timeDisplay = m;
  open.value = false;
}
</script>

<template>
  <div class="tdc" @pointerenter="open = true" @pointerleave="open = false">
    <button class="tdc-btn" v-tip="'时间显示'">{{ text }}</button>
    <div v-if="open" class="tdc-pop-wrap">
      <div class="tdc-pop">
        <MenuItem
          v-for="m in MODES"
          :key="m.value"
          :label="m.label"
          checkable
          :checked="ui.timeDisplay === m.value"
          @select="pick(m.value)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.tdc {
  position: relative;
  display: inline-flex;
  flex: none;
}
/* 触发按钮（13_播放条稿时间码位：30 高 padding 0 6、14/20 常态色；悬停 font-icon-6 底——同 rate-btn） */
.tdc-btn {
  padding: 0 var(--space-6);
  height: 30px;
  border-radius: var(--radius-6);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  transition: background 0.12s ease;
}
.tdc-btn:hover {
  background: var(--font-icon-6);
}
/* 弹窗定位壳：按钮正上方，padding-bottom 4 作指针桥接（同 RateControl） */
.tdc-pop-wrap {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding-bottom: var(--space-4);
  z-index: 10;
}
/* 弹窗容器：C3 / inset 1px border-1 / 圆角 8 / --shadow-pop / 内边距 4（同倍率弹窗） */
.tdc-pop {
  display: flex;
  flex-direction: column;
  padding: var(--space-4);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
}
/* 未选中 60%、选中实色（与倍率弹窗同口径） */
.tdc-pop :deep(.menu-item) {
  color: var(--font-icon-3);
}
.tdc-pop :deep(.menu-item.checked) {
  color: var(--font-icon-1);
}
</style>
