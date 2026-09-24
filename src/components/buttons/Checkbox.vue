<script setup lang="ts">
// 复选框——docs/06_UI重构/Design Token/组件/按钮/复选框.md：
// 框体样式 = styles/checkbox.css 通用类 .kit-check-box（md 任务列表渲染共用，同款第二处抽象）：
// 框 14×14 圆角 4，未选中 通用/三级底 + 通用/五级描边（hover 未选中描边 通用/六级——
// 稿 prose 写「亮色/六级」，亮色系无此档，2026-09-01 用户定为通用/六级）；
// 选中 品牌/常态底 + 亮色/半透明-1 描边 + 对勾-极小（check-tiny，亮色/一级）；
// 对勾弹出 = 缩放 250ms 过冲（过冲缓动沿用旧 AppRadio 既定档 cubic-bezier(0.34,1.56,0.64,1)，
// token 未收），收回 150ms 缓入快收不回弹（2026-08-12 用户定案沿用）；
// 底/描边 = 透明度过渡 150ms（稿 prose）。文字 = 次要文字 12/16 亮色/二级，gap 6。
// 选中态 hover / 禁用态稿未给，待补稿。
import AppIcon from "../AppIcon.vue";

const model = defineModel<boolean>({ required: true });
withDefaults(defineProps<{ label?: string; forceState?: "" | "hover" }>(), { forceState: "" });
</script>

<template>
  <button
    class="kit-checkbox kit-check-host"
    :class="[{ on: model }, forceState ? `force-${forceState}` : '']"
    @click="model = !model"
  >
    <!-- 对勾常驻 DOM、scale 切换（踩坑 026：v-if 移除图标会让框变空 flex 容器，
         基线推导变化 → 行盒高度差 1px） -->
    <span class="kit-check-box" :class="{ on: model }">
      <AppIcon name="check-tiny" :size="10" class="kit-check-mark" />
    </span>
    <span v-if="label" class="label">{{ label }}</span>
  </button>
</template>

<style scoped>
.kit-checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-6);
  padding: 0;
  height: 16px;
  flex: none;
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
/* 框体 = styles/checkbox.css .kit-check-box 通用类（含 hover / on / mark 全态）；
   force-hover 演示态在此桥接（preview 展区强制 hover 渲染） */
.kit-checkbox.force-hover .kit-check-box:not(.on) {
  box-shadow: inset 0 0 0 1px var(--n-g6);
}
</style>
