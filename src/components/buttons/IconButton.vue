<script setup lang="ts">
// 图标按钮——docs/06_UI重构/Design Token/组件/按钮/ 图标按钮-通用 / 大 / 小 / 立体 / 立体-半透明 五稿合一：
// variant：plain = 30×30 padding 6 圆角 6，图标 18 亮色/三级（hover 亮色/半透明-3 底 + 图标升二级）；
//           bare = 裸图标无容器（size lg = 18 / sm = 14，hover 图标升二级）；
//           solid = 通用/四级 + 立体感-通用，高 34 填充式圆角 8，图标 18 恒 亮色/二级（hover 通用/六级）；
//           float = 通用/半透明-2 + 立体感-通用，30×30 圆角 6 浮于内容上，图标 18 恒 亮色/三级
//                   （hover 通用/四级底）；
// 图标尺寸 = 稿 icon 容器实寸（图标按实际尺寸导出，勿缩放——2026-08-29 走查）；
// hover 过渡 = 150ms 缓入缓出（--n-dur-150）。
// title = v-tip 悬浮 tip（内容容器轮补，与旧根目录 IconButton 对齐；禁用态视觉稿未画不做，§15）。
import { computed } from "vue";
import AppIcon from "../AppIcon.vue";
import type { TipValue } from "../../lib/tip";

const props = withDefaults(
  defineProps<{
    name: string;
    variant?: "plain" | "bare" | "solid" | "float";
    size?: "lg" | "sm"; // 仅 bare：大 18 / 小 14
    forceState?: "" | "hover";
    title?: string | TipValue;
    disabled?: boolean; // 功能禁用（转码中等场景）；禁用态视觉稿未画（§15），仅去交互不改色
  }>(),
  { variant: "plain", size: "lg", forceState: "", title: undefined, disabled: false },
);
defineEmits<{ click: [e: MouseEvent] }>();

const iconSize = computed(() => (props.variant === "bare" && props.size === "sm" ? 14 : 18));
</script>

<template>
  <button
    class="ic-btn"
    :class="[variant, size, forceState ? `force-${forceState}` : '']"
    :disabled="disabled"
    v-tip="title"
    @click="$emit('click', $event)"
  >
    <AppIcon :name="name" :size="iconSize" />
  </button>
</template>

<style scoped>
.ic-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
/* 禁用态 = 30% 透明度（2026-09-20 用户定，控件族通用口径） */
.ic-btn:disabled {
  cursor: default;
  pointer-events: none;
  opacity: 0.3;
}
/* plain / float：30×30 方钮 */
.ic-btn.plain,
.ic-btn.float {
  width: 30px;
  height: 30px;
  padding: var(--n-space-6);
  border-radius: var(--n-radius-6);
}
.ic-btn.plain {
  color: var(--n-l3);
}
.ic-btn.plain:hover,
.ic-btn.plain.force-hover {
  background: var(--n-l-a3);
  color: var(--n-l2);
}
/* bare：裸图标，大 18 / 小 14 */
.ic-btn.bare {
  padding: 0;
  color: var(--n-l3);
}
.ic-btn.bare.lg {
  width: 18px;
  height: 18px;
}
.ic-btn.bare.sm {
  width: 14px;
  height: 14px;
}
.ic-btn.bare:hover,
.ic-btn.bare.force-hover {
  color: var(--n-l2);
}
/* solid：立体填充式（宽随父容器） */
.ic-btn.solid {
  width: 100%;
  height: 34px;
  padding: var(--n-space-8) 0;
  border-radius: var(--n-radius-8);
  background: var(--n-g4);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l2);
}
.ic-btn.solid:hover,
.ic-btn.solid.force-hover {
  background: var(--n-g6);
}
/* float：半透明浮钮（浮于缩略图等内容上） */
.ic-btn.float {
  background: var(--n-g-a2);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l3);
}
.ic-btn.float:hover,
.ic-btn.float.force-hover {
  background: var(--n-g4);
}
</style>
