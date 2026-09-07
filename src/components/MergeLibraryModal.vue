<script setup lang="ts">
// 合并资源库弹窗（2026-09-08 = 布局/模态窗口/合并资源库 稿，纯 UI 轮——用户当场定：只换新样式，
// 合并业务本体后排期）：壳 = modals/BaseModal（498 宽稿值）。
// 列表 = 当前空间已登记的其余库（requestMergeLibrary 加载时排除当前库；容量列 = 稿 320GB 位，
// 无数据源暂不渲染——扫盘统计随功能轮补）；行 = 34 高 padding 8/12 圆角 6 space-between，
// 未选 通用/四级底 / 选中 通用/六级底 + 1px 亮色/半透明-3 + 尾对勾（check 18 亮色/一级，
// 未选 opacity 0 占位——稿口径）；对勾 = 行级各自切换（稿两行独立对勾位按多选做，单/多选语义待功能轮定）。
// 底行 = 「找到 N 个资源库」（正文 13/18 三级 + N 数字-大品牌色）space-between + 按钮组；
// 确认钮不接动作（合并后端未实现——禁用态稿未画不自创，§24 口径仅不响应）。
// 列表超量滚动稿未画——库量级小（2026-09-07 用户：资源库不至于多到需要排序），自然撑高不预设。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";

const props = defineProps<{ names: string[] }>();
const emit = defineEmits<{ close: [] }>();

const selected = ref<Set<string>>(new Set());
function toggle(name: string): void {
  const next = new Set(selected.value);
  if (next.has(name)) next.delete(name);
  else next.add(name);
  selected.value = next;
}
</script>

<template>
  <BaseModal title="合并资源库" :width="498">
    <div class="lib-list">
      <button
        v-for="n in props.names"
        :key="n"
        class="lib-item"
        :class="{ on: selected.has(n) }"
        @click="toggle(n)"
      >
        <span class="lib-name">{{ n }}</span>
        <AppIcon name="check" :size="18" class="lib-check" />
      </button>
    </div>
    <div class="foot">
      <span class="hint">找到 <span class="num">{{ props.names.length }}</span> 个资源库</span>
      <span class="btns">
        <ActionButton label="取消" variant="cancel" size="lg" @click="emit('close')" />
        <ActionButton label="确认" variant="primary" size="lg" />
      </span>
    </div>
  </BaseModal>
</template>

<style scoped>
/* 资源库列表（稿）：列 gap 6；行 34 高 padding 8/12 圆角 6 space-between */
.lib-list {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-6);
}
.lib-item {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
  box-sizing: border-box;
  height: 34px;
  padding: var(--n-space-8) var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g4);
  cursor: default;
}
/* 选中 = 通用/六级底 + 1px 亮色/半透明-3 真 border（稿 border 口径）+ 对勾现身 */
.lib-item.on {
  background: var(--n-g6);
  border: 1px solid var(--n-l-a3);
}
.lib-name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.lib-check {
  flex: none;
  color: var(--n-l1);
  opacity: 0; /* 未选对勾占位（稿 opacity 0 口径） */
}
.lib-item.on .lib-check {
  opacity: 1;
}
/* 底行（稿「提示文字+按钮」）：space-between，提示 正文 13/18 三级 + N 数字-大品牌色 */
.foot {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-24);
}
.hint {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
}
.hint .num {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-brand);
}
.btns {
  display: flex;
  flex-direction: row;
  gap: var(--n-space-8);
  flex: none;
}
</style>
