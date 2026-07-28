<script setup lang="ts">
// 操作通知条（6.0 地基，参照 Eagle 通知条截图）：浮于中间面板工具栏下方（不挡工具栏），
// 构造 = 状态图标（成功 = 绿圆底白勾）+ 消息 + 可选动作按钮（撤销 / 重做）+ 关闭。
// 挂点：主窗口 ContentArea 根（居中于中间面板）、预览子窗口 App 根（整窗即内容区）；
// 位置由 .toast-stack 统一（top 对应标题栏高度），z-1600 高于全屏预览（1500）保证反馈可见。
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import { useToastStore } from "../stores/toast";

const toast = useToastStore();
</script>

<template>
  <div class="toast-stack">
    <TransitionGroup name="toast">
      <div v-for="t in toast.items" :key="t.id" class="toast">
        <span v-if="t.icon === 'success'" class="icon-success">
          <AppIcon name="check" :size="12" />
        </span>
        <span class="message">{{ t.message }}</span>
        <button v-if="t.actionLabel" class="action" @click="toast.act(t.id)">
          {{ t.actionLabel }}
        </button>
        <IconButton name="window-close" small title="关闭" @click="toast.close(t.id)" />
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
/* 浮层容器：居中于父级内容区，顶部对齐标题栏下缘 + 8（标题栏 = 外层 4 + padding 12×2 + 按钮 28 = 56，
   60 + 间距 4 = 64；不挡工具栏，2026-07-28 用户定）；z-1600 高于全屏预览（1500） */
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
/* 通知条：C3 / inset border-1 / 圆角 8 / --shadow-pop（与二级窗口同浮层体系） */
.toast {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-12);
  background: var(--c3);
  box-shadow:
    inset 0 0 0 var(--border-thin) var(--border-1),
    var(--shadow-pop);
  border-radius: var(--radius-8);
  pointer-events: auto;
}
/* 成功图标：--success 绿圆底 + 白勾（参照 Eagle 通知条截图） */
.icon-success {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--success);
  color: var(--font-icon-1);
  flex: none;
}
.message {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
}
/* 动作按钮（撤销 / 重做）：纯文字，悬停 font-icon-6 底（与菜单 item 同交互） */
.action {
  padding: 0 var(--space-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  border-radius: var(--radius-4);
  flex: none;
}
.action:hover {
  background: var(--font-icon-6);
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
