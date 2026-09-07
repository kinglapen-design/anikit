<script setup lang="ts">
// 输入框——docs/06_UI重构/Design Token/组件/输入框/ 大 / 小 两稿合一：
// size：lg = 高 34 / padding 8/12 / 常态描边 通用三级；sm = 高 26 / padding 4/12 / 常态描边 亮色/半透明-3，
// 文字居中（稿 justify-content center，年/月/日小框语境）；lg 文字左对齐。
// 底色通用二级 + 圆角 8；文字 正文档 13/18 亮色/二级；占位符 亮色/四级（2026-08-29 用户定）。
// 聚焦 = 亮色/半透明-1 描边，线条走卡片同款双层伪元素 inset 0（禁真 border 禁负 inset，速查表 §16）：
// ::after 常态层瞬时、::before 聚焦层 opacity 过渡 150ms（聚焦不做颜色过渡，踩坑 036）。
// hover 稿无 → 不做；禁用态稿未画 → 属性透传不改视觉（随按钮族记 09 待办，不自创）。
// trailingIcon = 右置装饰图标 18（亮色/四级，详情面板链接框稿，7.1 二十三轮扩）；readonly 透传（只读展示口径）。
// focusable=false = 聚焦描边不显（2026-08-31 用户定：仅快速访问固定入口框用——亮边误导人以为能输入；
// 文件夹/文件名称框保留聚焦态，改名语义上可输入，勿全局禁用 readonly 聚焦）。
import { ref } from "vue";
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    size?: "lg" | "sm";
    placeholder?: string;
    disabled?: boolean;
    readonly?: boolean;
    trailingIcon?: string;
    focusable?: boolean; // false = 聚焦描边不显（2026-08-31 用户定：快速访问固定入口框不给聚焦边框——
    // 亮了误导人以为能输入；文件夹/文件名称框保留聚焦态，改名业务语义上可输入）
    forceState?: "" | "focus";
  }>(),
  { size: "lg", placeholder: undefined, disabled: false, readonly: false, trailingIcon: undefined, focusable: true, forceState: "" },
);
const model = defineModel<string>({ default: "" });

const inputEl = ref<HTMLInputElement>();
// 壳化后外部聚焦入口：转发内嵌 input
defineExpose({
  focus: () => inputEl.value?.focus(),
  select: () => inputEl.value?.select(),
});
</script>

<template>
  <div class="ti-root" :class="[size, { disabled, 'no-focus': !focusable }, forceState ? `force-${forceState}` : '']">
    <input
      ref="inputEl"
      v-model="model"
      class="ti-inner"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
    />
    <!-- 右置装饰图标（详情面板链接框稿：18 实寸 亮色/四级，纯装饰不交互） -->
    <AppIcon v-if="trailingIcon" :name="trailingIcon" :size="18" class="ti-trail" />
  </div>
</template>

<style scoped>
.ti-root {
  box-sizing: border-box;
  position: relative;
  width: 100%;
  display: flex;
  align-items: center;
  background: var(--n-g2);
  border-radius: var(--n-radius-8);
}
.ti-root.lg {
  height: 34px;
  padding: var(--n-space-8) var(--n-space-12);
}
.ti-root.sm {
  height: 26px;
  padding: var(--n-space-4) var(--n-space-12);
}

/* 常态 1px 描边层（lg = 通用三级 / sm = 亮色/半透明-3） */
.ti-root::after {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 1;
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
  pointer-events: none;
}
.ti-root.sm::after {
  border-color: var(--n-l-a3);
}
/* 聚焦描边层（亮色/半透明-1）：opacity 过渡 150ms */
.ti-root::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 2;
  border: 1px solid var(--n-l-a1);
  border-radius: var(--n-radius-8);
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.ti-root:focus-within::before,
.ti-root.force-focus::before {
  opacity: 1;
}
/* no-focus（focusable=false，快速访问固定入口框）：聚焦描边恒隐（2026-08-31 用户定） */
.ti-root.no-focus:focus-within::before {
  opacity: 0;
}

/* 内嵌 input：透明去框（底色 / 描边 / 聚焦全在壳） */
.ti-inner {
  flex: 1;
  min-width: 0;
  padding: 0;
  border: none;
  outline: none;
  background: transparent;
  font-family: var(--n-font-family);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.ti-root.sm .ti-inner {
  text-align: center;
}
.ti-inner::placeholder {
  color: var(--n-l4);
}
.ti-inner:disabled {
  cursor: default;
}
/* 右置装饰图标：亮色/四级、不抢点击（链接框稿） */
.ti-trail {
  flex: none;
  color: var(--n-l4);
  pointer-events: none;
}
</style>
