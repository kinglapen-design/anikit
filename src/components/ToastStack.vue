<script setup lang="ts">
// 通知栏（阶段七稿：docs/UI 重构/Design Token/组件/悬浮栏/通知栏）：
// 胶囊 = 通用/半透明-2 底 + 立体感-背景模糊-大阴影 + blur 小 + 圆角 99px（稿值字面量，同 .fb-shell 21px 先例）；
// 高 46 / padding 12 16 12 12 / gap 12。状态圆点 22（成功 --n-success + 对勾-小 / 失败 --n-danger + 关闭-小，
// 内图标 14 一级）+ 立体感-主要按钮；「失败和成功按需出现」= 每条单图标（2026-08-29 用户定，store 契约不变）。
// 行为文字 = 正文 一级；动作（撤销）= 按钮文字-大 一级；末尾关闭 = close-small 14 二级（hover 态稿未画，记 09 待办）。
// 定位归 .notice-anchor（base.css）：列表容器顶部居中、不挡工具栏；与选择模式横幅同族同锚点（横幅在上，2026-08-29 用户定）。
// 出入场 = 上→下位移 8px + 透明度，250ms 小位移 缓入缓出（稿 prose「上→下位移出现」，时长/缓动走动画 token）。
import AppIcon from "./AppIcon.vue";
import { useToastStore } from "../stores/toast";

const toast = useToastStore();
</script>

<template>
  <TransitionGroup name="toast" tag="div" class="toast-stack">
    <div v-for="t in toast.items" :key="t.id" class="toast">
      <span class="state" :class="t.icon">
        <AppIcon :name="t.icon === 'success' ? 'check-small' : 'close-small'" :size="14" />
      </span>
      <span class="message">{{ t.message }}</span>
      <button v-if="t.actionLabel" class="action" @click="toast.act(t.id)">{{ t.actionLabel }}</button>
      <button class="close" v-tip="'关闭'" @click="toast.close(t.id)">
        <AppIcon name="close-small" :size="14" />
      </button>
    </div>
  </TransitionGroup>
</template>

<style scoped>
/* 堆叠列：定位由 .notice-anchor 承担；空隙不挡卡片，通知条自身恢复交互 */
.toast-stack {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-8);
  pointer-events: none;
}
/* 通知条胶囊（稿：通用/半透明-2 + 大阴影 + inset 高光 + blur + 圆角 99；宽随内容 max-content） */
.toast {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  height: 46px;
  padding: var(--n-space-12) var(--n-space-16) var(--n-space-12) var(--n-space-12);
  gap: var(--n-space-12);
  border-radius: 99px;
  background: var(--n-g-a2);
  box-shadow: var(--n-effect-raised-blur);
  backdrop-filter: blur(var(--n-blur-sm));
  white-space: nowrap;
  pointer-events: auto;
}
/* 状态圆点 22：成功/失败底 + 立体感-主要按钮 + 内图标 14 一级（check-small 已转 currentColor） */
.state {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 15px;
  color: var(--n-l1);
  box-shadow: var(--n-effect-raised-primary);
  flex: none;
}
.state.success {
  background: var(--n-success);
}
.state.fail {
  background: var(--n-danger);
}
/* 行为文字 = 正文 一级 */
.message {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
}
/* 动作（撤销）= 按钮文字-大 一级；hover 态稿未画（09 待办），仅指针 */
.action {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  flex: none;
  cursor: pointer;
}
/* 末尾关闭 = close-small 14 二级，裸钮无 hover 底（稿未画） */
.close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--n-l2);
  flex: none;
  cursor: pointer;
}
/* 出入场：上→下位移 + 透明度，250ms 小位移 缓入缓出 */
.toast-enter-active,
.toast-leave-active {
  transition:
    opacity var(--n-dur-250) var(--n-ease-in-out),
    transform var(--n-dur-250) var(--n-ease-in-out);
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(calc(-1 * var(--n-space-8)));
}
</style>
