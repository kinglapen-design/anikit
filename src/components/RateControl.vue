<script setup lang="ts">
// 倍率控件（5.2，序列帧/动图/音频/视频播放条共用）：按钮显示当前倍率（如 1x），
// 悬停出竖排档位弹窗（YouTube 同款：上 8x → 下 0.25x，当前档位左侧 ✓ 标记），
// 点击选项设置倍率并关闭弹窗。容器样式同音量弹层（C3 / inset 1px border-1 / 圆角 8 /
// --shadow-pop / 内边距 4），底部 4px 透明桥接区（padding）防指针滑入弹窗时断开 hover。
import { ref } from "vue";
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
        <button v-for="r in RATES" :key="r" class="rate-item" @click="pick(r)">
          <span class="check">{{ r === rate ? "✓" : "" }}</span>{{ r }}x
        </button>
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
/* 档位项：高 28 / 圆角 4 / 正文文字 font-icon-1 / 悬停 font-icon-6（5%） */
.rate-item {
  display: flex;
  align-items: center;
  height: 28px;
  padding: 0 var(--space-8);
  border-radius: var(--radius-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  transition: background 0.12s ease;
}
.rate-item:hover {
  background: var(--font-icon-6);
}
/* ✓ 标记位定宽，无标记的档位文字对齐 */
.check {
  width: 14px;
  flex: none;
}
</style>
