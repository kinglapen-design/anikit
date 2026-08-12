<script setup lang="ts">
// 模态窗口骨架（figma-css 08 无关闭按钮 / 09 无标题栏 + 布局稿标准标题栏型）：
// 统一 C2 底 + 1.5px 亮-选中边框 + 投影 + 圆角 8，遮罩 C1-80；
// standard = 标题栏（图标 + 标题 + 关闭 X）→ 内容 → 按钮组；
// confirm  = 无关闭按钮（figma 08）：padding 24 gap 24、标题栏 30×30 品牌底图标、内容 gap 12、仅确认/取消；
// alert    = 无标题栏侵入式（figma 09）：内容行横排——48 圆形状态图标（error/warning 状态色底）+
// 右侧标题 + 描述（竖排 gap 8），按钮组右对齐；**2026-08-12 修正：初版做成居中竖排与 09 稿不符**。
// 按钮组由 actions slot 给（AppButton primary/panel），内容区默认 slot。
// alert 侵入式弹出配提示音（2026-08-10 用户供音频：侵入弹窗.wav，lib/sounds）。
import { onMounted } from "vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import { playAlert } from "../lib/sounds";

const props = withDefaults(
  defineProps<{
    title: string;
    icon?: string; // 标题栏图标（standard / confirm）
    variant?: "standard" | "confirm" | "alert";
    alertIcon?: "error" | "warning"; // alert 形态状态图标
  }>(),
  { icon: undefined, variant: "standard", alertIcon: "error" },
);
const emit = defineEmits<{ close: [] }>();

onMounted(() => {
  if (props.variant === "alert") playAlert();
});
</script>

<template>
  <div class="modal-mask" @pointerdown.self="emit('close')">
    <div class="app-modal" :class="[variant]" role="dialog" :aria-label="title">
      <!-- alert（figma 09 侵入式）：内容行横排——48 圆形状态图标 + 右侧标题/描述竖排 -->
      <template v-if="variant === 'alert'">
        <div class="alert-row">
          <span class="alert-badge" :class="alertIcon">
            <AppIcon :name="alertIcon === 'error' ? 'error-large' : 'warning'" :size="18" />
          </span>
          <div class="alert-text">
            <h2 class="title">{{ title }}</h2>
            <div class="desc"><slot /></div>
          </div>
        </div>
      </template>
      <!-- confirm：30×30 品牌底图标 + 标题，无关闭按钮 -->
      <template v-else-if="variant === 'confirm'">
        <div class="titlebar">
          <span v-if="icon" class="title-badge">
            <AppIcon :name="icon" :size="18" />
          </span>
          <h2 class="title">{{ title }}</h2>
        </div>
      </template>
      <!-- standard：标题栏 + 关闭 X -->
      <template v-else>
        <div class="titlebar">
          <AppIcon v-if="icon" :name="icon" :size="18" class="title-icon" />
          <h2 class="title">{{ title }}</h2>
          <span class="spacer" />
          <IconButton name="window-close" title="关闭" @click="emit('close')" />
        </div>
      </template>
      <div v-if="variant !== 'alert'" class="content">
        <slot />
      </div>
      <div v-if="$slots.actions" class="actions">
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-mask {
  position: fixed;
  inset: 0;
  z-index: 1800;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 骨架：C2 底 + 1.5px 亮-选中边框 + 投影 + 圆角 8（figma 08/09 与布局稿统一） */
.app-modal {
  display: flex;
  flex-direction: column;
  gap: var(--space-24);
  padding: var(--space-24);
  border-radius: var(--radius-8);
  background: var(--c2);
  box-shadow:
    inset 0 0 0 var(--border-strong) var(--line-active),
    var(--shadow-pop);
  max-width: calc(100vw - 96px);
  max-height: calc(100vh - 96px);
  /* 弹出动画：缩放 + 透明度（00_要求：窗口需要放大和缩小） */
  animation: modal-pop 0.16s cubic-bezier(0.16, 1, 0.3, 1);
}
@keyframes modal-pop {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
.titlebar {
  display: flex;
  align-items: center;
  gap: var(--space-12);
}
/* confirm 标题徽章：30×30 品牌底 + 亮-选中边框 + 圆角 6（figma 08） */
.title-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: var(--radius-6);
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-highlight);
  flex: none;
}
.title-icon {
  color: var(--text-normal);
  flex: none;
}
.title {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-highlight);
}
.spacer {
  flex: 1;
}
/* 内容区：gap 12 增加内容显示密度（figma 08） */
.content {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  min-height: 0;
  overflow: auto;
}
/* 按钮组：右对齐（确认 = 品牌色高亮 / 取消 = C4 底色，由 actions slot 给 AppButton） */
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-12);
}
/* alert（figma 09）：内容行横排——左 48 圆形状态徽章 + 右标题/描述竖排（gap 8） */
.alert-row {
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  gap: var(--space-12);
}
/* 状态徽章：48×48 圆形 / 状态色底 / 1px 亮-选中边框（inset，规范 4）/ 图标 18 常态色 */
.alert-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: var(--radius-circle);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-normal);
  flex: none;
}
.alert-badge.error {
  background: var(--error);
}
.alert-badge.warning {
  background: var(--warning);
}
.alert-text {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--space-8);
  min-height: 48px;
}
.desc {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
</style>
