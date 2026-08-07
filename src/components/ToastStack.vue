<script setup lang="ts">
// 通知栏（figma-css 07_通知栏）：胶囊形 C2 底 + C5-底色边框 + 投影；
// 状态圆形图标 24（成功/失败，figma：按情况切换成功和失败图标）+ 行为文字（正文常态）
// + 可选动作（撤销，标题档高亮）+ 删除（20×20 圆角 10，clear 标签档）。
// 挂点与层级沿用：浮于中间面板工具栏下方（top 64），z-1600 高于全屏预览。
import AppIcon from "./AppIcon.vue";
import { useToastStore } from "../stores/toast";

const toast = useToastStore();
</script>

<template>
  <div class="toast-stack">
    <TransitionGroup name="toast">
      <div v-for="t in toast.items" :key="t.id" class="toast">
        <AppIcon v-if="t.icon === 'success'" name="success" :size="24" class="state-icon" />
        <span class="message">{{ t.message }}</span>
        <button v-if="t.actionLabel" class="action" @click="toast.act(t.id)">
          {{ t.actionLabel }}
        </button>
        <button class="close" title="关闭" @click="toast.close(t.id)">
          <AppIcon name="clear" :size="12" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
/* 浮层容器：居中于父级内容区，顶部对齐标题栏下缘（64，不挡工具栏，2026-07-28 用户定）；
   z-1600 高于全屏预览（1500） */
.toast-stack {
  position: absolute;
  top: 64px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1600;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-8);
  pointer-events: none; /* 空隙不挡卡片；通知条自身恢复交互 */
}
/* 通知栏：胶囊 C2 底 + C5-底色 1px 边框 + 投影（figma 07） */
.toast {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  padding: var(--space-12);
  background: var(--c2);
  box-shadow:
    inset 0 0 0 1px var(--c5-base),
    var(--shadow-pop);
  border-radius: var(--radius-pill);
  pointer-events: auto;
}
.state-icon {
  flex: none;
}
.message {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
}
/* 动作（撤销）：标题档 高亮色 */
.action {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-highlight);
  white-space: nowrap;
  flex: none;
}
/* 删除：20×20 圆角 10（figma 07），clear 标签档，hover 叠暗-悬停 */
.close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 10px;
  color: var(--text-label);
  flex: none;
  transition: background 0.15s ease;
}
.close:hover {
  background: var(--line-hover);
}
/* 出入场：上浮淡入淡出 */
.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 0.15s ease,
    transform 0.15s ease;
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(calc(-1 * var(--space-8)));
}
</style>
