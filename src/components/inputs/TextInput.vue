<script setup lang="ts">
// 输入框——docs/06_UI重构/Design Token/组件/输入框/ 大 / 小 两稿合一：
// size：lg = 高 34 / padding 8/12 / 常态描边 通用三级；sm = 高 26 / padding 4/12 / 常态描边 亮色/半透明-3。
// **文字一律左对齐**（2026-09-21 用户定：所有输入全部左对齐——sm 稿 justify-content center
// 年/月/日小框语境特化废，日期筛选小框同改）。
// 底色通用二级 + 圆角 8；文字 正文档 13/18 亮色/二级；占位符 亮色/四级（2026-08-29 用户定）。
// 聚焦 = 亮色/半透明-1 描边，线条走卡片同款双层伪元素 inset 0（禁真 border 禁负 inset，速查表 §16）：
// ::after 常态层瞬时、::before 聚焦层 opacity 过渡 150ms（聚焦不做颜色过渡，踩坑 036）。
// hover 稿无 → 不做；禁用态 = 30% 透明度（2026-09-20 用户定控件族口径，速查 §39②）。
// trailingIcon = 右置装饰图标 18（亮色/四级，详情面板链接框稿，7.1 二十三轮扩）；
// leadingIcon = 左置图标 18（亮色/三级，移动弹窗搜索框稿，2026-09-08 功能模态窗二轮扩）；
// readonly 透传（只读展示口径）。
// clearable（2026-09-20 R2 烧 AppInput/AppSearchBar 补能力）：有内容且非禁用/只读时出清除钮
// （clear 12 亮色/四级 hover 升三级，150ms；沿旧 AppInput/AppSearchBar 既定交互，sm 档暂无 clear 消费方）。
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
    leadingIcon?: string; // 左置装饰图标（移动弹窗搜索框稿：18 实寸 亮色/三级）
    trailingIcon?: string;
    focusable?: boolean; // false = 聚焦描边不显（2026-08-31 用户定：快速访问固定入口框不给聚焦边框——
    // 亮了误导人以为能输入；文件夹/文件名称框保留聚焦态，改名业务语义上可输入）
    clearable?: boolean; // 有内容时出清除钮（烧 AppInput/AppSearchBar 补的能力，2026-09-20）
    forceState?: "" | "focus";
  }>(),
  { size: "lg", placeholder: undefined, disabled: false, readonly: false, leadingIcon: undefined, trailingIcon: undefined, focusable: true, clearable: false, forceState: "" },
);
const model = defineModel<string>({ default: "" });

const inputEl = ref<HTMLInputElement>();
function onClear(): void {
  model.value = "";
  inputEl.value?.focus();
}
// 壳化后外部聚焦入口：转发内嵌 input
defineExpose({
  focus: () => inputEl.value?.focus(),
  select: () => inputEl.value?.select(),
});
</script>

<template>
  <div class="ti-root" :class="[size, { disabled, 'has-lead': !!leadingIcon, 'no-focus': !focusable }, forceState ? `force-${forceState}` : '']">
    <!-- 左置装饰图标（移动弹窗搜索框稿：18 实寸 亮色/三级，纯装饰不交互） -->
    <AppIcon v-if="leadingIcon" :name="leadingIcon" :size="18" class="ti-lead" />
    <input
      ref="inputEl"
      v-model="model"
      class="ti-inner"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
    />
    <!-- 清除钮（clearable：有内容且可交互时出；clear 12 亮色/四级 hover 升三级） -->
    <button
      v-if="clearable && model && !disabled && !readonly"
      class="ti-clear"
      v-tip="'清除'"
      tabindex="-1"
      @click="onClear"
    >
      <AppIcon name="clear" :size="12" />
    </button>
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
/* 禁用态 = 30% 透明度（2026-09-20 用户定，控件族通用口径，速查 §39②） */
.ti-root.disabled {
  opacity: 0.3;
}
.ti-root.disabled .ti-inner {
  pointer-events: none;
}

/* 内嵌 input：透明去框（底色 / 描边 / 聚焦全在壳）；文字一律左对齐（2026-09-21 用户定——
   sm 稿居中特化废，原 .sm text-align:center 删除） */
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
.ti-inner::placeholder {
  color: var(--n-l4);
}
.ti-inner:disabled {
  cursor: default;
}
/* 左置装饰图标：亮色/三级、不抢点击（移动弹窗搜索框稿）；与输入文字间距 6（稿 gap 6） */
.ti-lead {
  flex: none;
  color: var(--n-l3);
  pointer-events: none;
}
/* 右置装饰图标：亮色/四级、不抢点击（链接框稿） */
.ti-trail {
  flex: none;
  color: var(--n-l4);
  pointer-events: none;
}
/* 清除钮（clearable）：亮色/四级，hover 升三级 150ms */
.ti-clear {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--n-l4);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.ti-clear:hover {
  color: var(--n-l3);
}
.ti-root.has-lead,
.ti-root:has(.ti-clear),
.ti-root:has(.ti-trail) {
  gap: var(--n-space-6);
}
</style>
