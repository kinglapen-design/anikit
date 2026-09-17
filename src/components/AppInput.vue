<script setup lang="ts">
// 输入框（figma-css 11_输入框）：small = padding 4/8、C2 底、28 高；medium = padding 6/8、C1 底、32 高；
// large = padding 8/12、C2 底、36 高；圆角 6（figma 导出值，2026-08-07 用户定：token 8px 口径作废）。
// 边框：常态 = 壳 box-shadow inset 1px 暗-悬停（无过渡）；聚焦 = 壳 ::after 品牌色 1.5px
// **opacity 淡入淡出**（00_要求淡入淡出保留；2026-08-19 残留修复，踩坑 036：box-shadow/border-color
// 颜色过渡是重绘型动画，圆角 + 1.5px 分数像素下 GPU 合成损伤区偶发漏覆盖 → 失焦后品牌色像素残留；
// opacity 走合成器无光栅损伤）。
// 2026-08-19 统一壳化（原"非 clearable 裸 input 根"分支废除）：壳统一承担底色 / 边框 / 聚焦层，
// 内嵌 input 透明去框——聚焦层 opacity 机制必须有壳（input 无伪元素）；
// 外部聚焦改用 expose 的 focus()（壳化后 $el 是壳 div 不再可 focus）。
// clearable（2026-08-18，36/37 稿输入框内"清除 ×"）：有内容时尾部 20×20 清除钮
//（AppSearchBar 同款：标签档、hover 暗-悬停罩）；**tabindex -1 不进 Tab 序**
//（2026-09-13 实机走查：Tab 应直达下一输入框，不停在 × 上）。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";

const props = withDefaults(
  defineProps<{ size?: "small" | "medium" | "large"; placeholder?: string; disabled?: boolean; clearable?: boolean }>(),
  { size: "medium", placeholder: undefined, disabled: false, clearable: false },
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
  <div class="app-input-shell" :class="[size, { disabled }]">
    <input
      ref="inputEl"
      v-model="model"
      class="app-input inner"
      :placeholder="placeholder"
      :disabled="disabled"
    />
    <button v-if="clearable && model && !disabled" class="clear" tabindex="-1" v-tip="'清除'" @click="model = ''">
      <AppIcon name="clear" :size="12" />
    </button>
  </div>
</template>

<style scoped>
/* 壳：底色 / 常态边框（无过渡） / 聚焦层挂点（relative） */
.app-input-shell {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-6);
  width: 100%;
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.app-input-shell.small {
  padding: var(--space-4) var(--space-8);
  background: var(--c2);
}
.app-input-shell.medium {
  padding: var(--space-6) var(--space-8);
  background: var(--c1);
}
.app-input-shell.large {
  padding: var(--space-8) var(--space-12);
  background: var(--c2);
}
/* 聚焦层（品牌色 1.5px）：opacity 淡入淡出（踩坑 036 残留修复）；伪元素 border 内画不占布局 */
.app-input-shell::after {
  content: "";
  position: absolute;
  inset: 0;
  border: var(--border-strong) solid var(--brand);
  border-radius: var(--radius-6);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease;
}
.app-input-shell:focus-within::after {
  opacity: 1;
}
.app-input-shell.disabled {
  color: var(--text-faint);
}
.app-input-shell.disabled::after {
  display: none;
}
/* 内嵌 input：透明去框（底色 / 边框 / 聚焦全在壳） */
.app-input.inner {
  flex: 1;
  min-width: 0;
  padding: 0;
  background: transparent;
  border-radius: 0;
  box-shadow: none;
  color: inherit;
  font-size: inherit;
  line-height: inherit;
}
.app-input.inner::placeholder {
  color: var(--text-faint);
}
.app-input.inner:disabled {
  cursor: default;
}
/* 清除钮（36/37 稿）：20×20 标签档，hover 暗-悬停罩 */
.app-input-shell .clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-6);
  color: var(--text-label);
  flex: none;
  transition: background 0.15s ease;
}
.app-input-shell .clear:hover {
  background: var(--line-hover);
}
</style>
