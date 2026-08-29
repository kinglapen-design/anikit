<script setup lang="ts">
// 拉下按钮-横向——docs/UI 重构/Design Token/组件/按钮/拉下按钮-横向：
// 容器 通用/六级底 + 亮色/半透明-3 1px 内描边，高 28 padding 6/6/6/12 gap 6 圆角 6；
// 常态只显示选中项 + 箭头朝右，点箭头 inline 展开全部项（箭头转朝左），点项 = 选中并收起；
// 项文字 = 次要文字-加粗：选中 亮色/一级、未选 亮色/三级；箭头 hover 升 亮色/二级。
import { ref } from "vue";
import AppIcon from "../AppIcon.vue";

const model = defineModel<string>({ required: true });
const props = withDefaults(
  defineProps<{ options: readonly string[]; initialExpanded?: boolean }>(),
  { initialExpanded: false },
);

const expanded = ref(props.initialExpanded);

function pick(v: string): void {
  model.value = v;
  expanded.value = false;
}
</script>

<template>
  <div class="drop-btn" :class="{ expanded }">
    <template v-if="expanded">
      <button
        v-for="o in options"
        :key="o"
        class="opt"
        :class="{ on: o === model }"
        @click="pick(o)"
      >
        {{ o }}
      </button>
    </template>
    <span v-else class="opt on">{{ model }}</span>
    <button class="arrow" @click="expanded = !expanded">
      <AppIcon name="chevron-small" :size="14" />
    </button>
  </div>
</template>

<style scoped>
.drop-btn {
  display: inline-flex;
  align-items: center;
  padding: var(--n-space-6) var(--n-space-6) var(--n-space-6) var(--n-space-12);
  gap: var(--n-space-6);
  height: 28px;
  flex: none;
  border-radius: var(--n-radius-6);
  background: var(--n-g6);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
.opt {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
  white-space: nowrap;
}
.opt.on {
  color: var(--n-l1);
}
/* 箭头：常态朝右，展开朝左（chevron-small 图形朝上，±90° 旋转） */
.arrow {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  padding: 0;
  flex: none;
  color: var(--n-l3);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.arrow:hover {
  color: var(--n-l2);
}
.arrow :deep(.app-icon) {
  transition: transform var(--n-dur-250) var(--n-ease-in-out);
  transform: rotate(90deg);
}
.expanded .arrow :deep(.app-icon) {
  transform: rotate(-90deg);
}
</style>
