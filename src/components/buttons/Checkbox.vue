<script setup lang="ts">
// 复选框——docs/06_UI重构/Design Token/组件/按钮/复选框.md：
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
    class="kit-checkbox"
    :class="[{ on: model }, forceState ? `force-${forceState}` : '']"
    @click="model = !model"
  >
    <span class="box">
      <!-- 对勾常驻 DOM、scale 切换（踩坑 026：v-if 移除图标会让 .box 变空 flex 容器，
           基线推导变化 → 行盒高度差 1px） -->
      <AppIcon name="check-tiny" :size="10" class="mark" />
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
.box {
  width: 14px;
  height: 14px;
  border-radius: var(--n-radius-4);
  background: var(--n-g3);
  box-shadow: inset 0 0 0 1px var(--n-g5);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--n-l1);
  line-height: 0;
  flex: none;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    box-shadow var(--n-dur-150) var(--n-ease-in-out);
}
.kit-checkbox:hover:not(.on) .box,
.kit-checkbox.force-hover:not(.on) .box {
  box-shadow: inset 0 0 0 1px var(--n-g6);
}
.kit-checkbox.on .box {
  background: var(--n-brand);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
}
.mark {
  transform: scale(0);
  transition: transform var(--n-dur-150) var(--n-ease-in);
}
.kit-checkbox.on .mark {
  transform: scale(1);
  transition: transform var(--n-dur-250) cubic-bezier(0.34, 1.56, 0.64, 1);
}
</style>
