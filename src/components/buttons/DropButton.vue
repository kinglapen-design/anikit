<script setup lang="ts">
// 拉下按钮-横向——docs/06_UI重构/Design Token/组件/按钮/拉下按钮-横向：
// 容器 通用/六级底 + 亮色/半透明-3 1px 内描边，高 28 padding 6 圆角 6，**宽度自适应内容、随展开收起一起变**；
// 常态只显示选中项 + 展开箭头朝右；点箭头展开全部项（箭头转朝左），点项 = 选中并收起；
// 展开/收起动画 = 250ms 小位移（--n-dur-250）：**JS 实测各项内容宽 + wrap 显式 width 过渡**——
//   grid 0fr→1fr 推挤只适用外部定宽容器（item 卡片 240 有效），本组件内容自适应下 0fr 轨道仍把
//   max-content 计入容器宽度（收起态隐藏项占位撑宽、间距错乱，2026-08-29 走查实锤），故改实测方案；
// 间距模型：容器 padding 6 + 各项内层左 padding 6（首项视觉前距 12 = 稿值 6+6），缝恒由后项提供；
// 箭头 = 标题同款 expand（∨ 朝下，∓90° 旋转得朝右/朝左），hover 升 亮色/二级；
// 项文字 = 次要文字-加粗：选中 亮色/一级、未选 亮色/三级（hover 升二级）。
import { onMounted, ref } from "vue";
import AppIcon from "../AppIcon.vue";

const model = defineModel<string>({ required: true });
const props = withDefaults(
  defineProps<{ options: readonly string[]; initialExpanded?: boolean }>(),
  { initialExpanded: false },
);

const expanded = ref(props.initialExpanded);
const inners: (HTMLElement | null)[] = []; // v-for 函数 ref 收集（不参与响应式）
const widths = ref<number[]>([]);
const ready = ref(false); // 测量完成前禁过渡，防首帧闪动画

onMounted(() => {
  // inner 恒 width: max-content，任何状态下 offsetWidth = 内容实宽（含左 padding 6）
  widths.value = props.options.map((_, i) => inners[i]?.offsetWidth ?? 0);
  requestAnimationFrame(() => (ready.value = true));
});

function pick(v: string): void {
  model.value = v;
  expanded.value = false;
}
</script>

<template>
  <div class="drop-btn" :class="[{ expanded }, ready ? 'anim' : '']">
    <span
      v-for="(o, i) in options"
      :key="o"
      class="opt-wrap"
      :style="{ width: expanded || o === model ? `${widths[i] ?? 0}px` : '0px' }"
    >
      <span class="opt-inner" :ref="(el) => (inners[i] = el as HTMLElement | null)">
        <button v-if="o !== model" class="opt" @click="pick(o)">{{ o }}</button>
        <span v-else class="opt on">{{ o }}</span>
      </span>
    </span>
    <button class="arrow" @click="expanded = !expanded">
      <AppIcon name="expand" :size="14" />
    </button>
  </div>
</template>

<style scoped>
.drop-btn {
  display: inline-flex;
  align-items: center;
  padding: var(--n-space-6);
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
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.opt.on {
  color: var(--n-l1);
}
button.opt:hover {
  color: var(--n-l2);
}
/* 项宽度展开：wrap 受控显式宽度（JS 实测）+ overflow hidden；间距 = 内层左 padding 6 */
.opt-wrap {
  overflow: hidden;
  flex: none;
}
.anim .opt-wrap {
  transition: width var(--n-dur-250) var(--n-ease-in-out);
}
.opt-inner {
  display: flex;
  justify-content: flex-start;
  width: max-content;
  padding-left: var(--n-space-6);
}
/* 箭头：常态朝右，展开朝左（expand 图形 ∨ 朝下，∓90° 旋转）；左 padding 6 = 与末项间距 */
.arrow {
  display: inline-flex;
  align-items: center;
  justify-content: flex-start;
  width: 20px;
  height: 14px;
  padding: 0 0 0 var(--n-space-6);
  flex: none;
  color: var(--n-l3);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.arrow:hover {
  color: var(--n-l2);
}
.arrow :deep(.app-icon) {
  transition: transform var(--n-dur-250) var(--n-ease-in-out);
  transform: rotate(-90deg);
}
.expanded .arrow :deep(.app-icon) {
  transform: rotate(90deg);
}
</style>
