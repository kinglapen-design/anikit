<script setup lang="ts">
// 拉下按钮-横向——docs/UI 重构/Design Token/组件/按钮/拉下按钮-横向：
// 容器 通用/六级底 + 亮色/半透明-3 1px 内描边，高 28 padding 6/6/6/12 gap 6 圆角 6，宽度自适应内容；
// 常态只显示选中项 + 展开箭头朝右；点箭头展开全部项（箭头转朝左），点项 = 选中并收起；
// 展开/收起 = 未选项 grid 0fr→1fr 推挤式宽度展开（item 族 .ni-slide 同款机制），250ms 小位移（--n-dur-250）；
// 箭头 = 标题同款 expand（∨ 朝下，旋转 ±90° 得朝右/朝左），hover 升 亮色/二级；
// 项文字 = 次要文字-加粗：选中 亮色/一级、未选 亮色/三级（hover 升二级，2026-08-29 走查定）。
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
    <template v-for="o in options" :key="o">
      <span v-if="o === model" class="opt on">{{ o }}</span>
      <span v-else class="opt-wrap" :class="{ hide: !expanded }">
        <span class="opt-inner">
          <button class="opt" @click="pick(o)">{{ o }}</button>
        </span>
      </span>
    </template>
    <button class="arrow" @click="expanded = !expanded">
      <AppIcon name="expand" :size="14" />
    </button>
  </div>
</template>

<style scoped>
.drop-btn {
  display: inline-flex;
  align-items: center;
  padding: var(--n-space-6) var(--n-space-6) var(--n-space-6) var(--n-space-12);
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
/* 未选项推挤式宽度展开（间距 = 内层左 padding 6，收起时随内层一并裁掉） */
.opt-wrap {
  display: grid;
  grid-template-columns: 1fr;
  transition: grid-template-columns var(--n-dur-250) var(--n-ease-in-out);
}
.opt-wrap.hide {
  grid-template-columns: 0fr;
}
.opt-inner {
  overflow: hidden;
  min-width: 0;
  display: flex;
  justify-content: flex-start;
  padding-left: var(--n-space-6);
  transition: padding-left var(--n-dur-250) var(--n-ease-in-out);
}
.opt-wrap.hide .opt-inner {
  padding-left: 0;
}
/* 箭头：常态朝右，展开朝左（expand 图形 ∨ 朝下，∓90° 旋转）；左 padding 6 = 与末项间距（gap 会因 0fr 隐藏项留缝，不可用） */
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
