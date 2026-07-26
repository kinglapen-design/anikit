<script setup lang="ts">
// 倍率控件（5.2，序列帧/动图/音频/视频播放条共用）：按钮显示当前倍率（如 1x），
// 悬停出竖排档位弹窗（YouTube 同款：上 8x → 下 0.25x），点击选项设置倍率并关闭弹窗。
// 档位项复用 MenuItem（checkable）：与"查看方式"下拉同勾位（16px 槽 + 间距 6）；
// 未选中文字 font-icon-3（60%）、选中 font-icon-1（2026-07-26 用户定：选中态要一眼可辨）。
// 容器样式同音量弹层（C3 / inset 1px border-1 / 圆角 8 / --shadow-pop / 内边距 4），
// 底部 4px 透明桥接区（padding）防指针滑入弹窗时断开 hover。
import { ref } from "vue";
import MenuItem from "./MenuItem.vue";
import { RATES } from "../stores/preview";

defineProps<{ rate: number }>();
const emit = defineEmits<{ select: [rate: number] }>();
const open = ref(false);

function pick(r: number): void {
  emit("select", r);
  open.value = false;
}
</script>

<template>
  <div class="rate" @pointerenter="open = true" @pointerleave="open = false">
    <button class="rate-btn" v-tip="'倍率'">{{ rate }}x</button>
    <div v-if="open" class="rate-pop-wrap">
      <div class="rate-pop">
        <MenuItem
          v-for="r in RATES"
          :key="r"
          :label="`${r}x`"
          checkable
          :checked="r === rate"
          @select="pick(r)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.rate {
  position: relative;
  display: inline-flex;
  flex: none;
}
/* 按钮本体：原 rate-btn 观感不变（24 高文字按钮，悬停 font-icon-6 底） */
.rate-btn {
  padding: 0 var(--space-4);
  height: 24px;
  border-radius: var(--radius-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  transition: background 0.12s ease;
}
.rate-btn:hover {
  background: var(--font-icon-6);
}
/* 弹窗定位壳：按钮正上方，padding-bottom 4 作指针桥接（同 VolumeControl） */
.rate-pop-wrap {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding-bottom: var(--space-4);
  z-index: 10;
}
/* 弹窗容器：C3 / inset 1px border-1 / 圆角 8 / --shadow-pop / 内边距 4 */
.rate-pop {
  display: flex;
  flex-direction: column;
  padding: var(--space-4);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
}
/* 未选中 60%、选中实色（选中态一眼可辨，2026-07-26 用户定） */
.rate-pop :deep(.menu-item) {
  color: var(--font-icon-3);
}
.rate-pop :deep(.menu-item.checked) {
  color: var(--font-icon-1);
}
</style>
