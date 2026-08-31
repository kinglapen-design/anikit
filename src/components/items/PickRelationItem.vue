<script setup lang="ts">
// 选择关系 item——docs/UI 重构/Design Token/组件/item/选择模式 item/选择关系
// 卡片 240×118 / padding 12 / 列 gap 8：
//   行1 选中文件框（通用二级底 + 五级描边）+ 关闭钮（恒显）；
//   行2 关系双框（同款底，label + 箭头-小：源图形朝上，左框转 180 = ↓、右框原样 = ↑）+ 关闭占位（透明，对齐用）；
//   行3 原始文件框（通用四级底 + 六级描边，文字三级 = 只读态）+ 关闭占位。
// 输入框聚焦（:focus-within）= 亮色/半透明-1 描边。
import "./items.css";
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    picked: string; // 选中文件显示名
    fromLabel: string; // 左框称谓（源侧，如「父亲」）
    toLabel: string; // 右框称谓（目标侧，如「儿子」）
    origin: string; // 原始文件（发起方）显示名
    pickedPlaceholder?: string; // 选中文件框占位（空态占位组「点击卡片选择」，7.1 二十三轮补）
    disabled?: boolean; // 关系双框禁用（空态占位组口径，同上）
    closable?: boolean; // 关闭钮显隐（空态占位组无删除钮，旧口径沿）；默认 true
    forceState?: "" | "hover";
  }>(),
  { pickedPlaceholder: undefined, disabled: false, closable: true, forceState: "" },
);

const emit = defineEmits<{
  close: [];
  "update:fromLabel": [v: string];
  "update:toLabel": [v: string];
}>();
</script>

<template>
  <div
    class="pr ni-card"
    :class="forceState ? `force-${forceState}` : ''"
  >
    <div class="row">
      <span class="box">
        <input class="box-input" :value="picked" :placeholder="pickedPlaceholder" readonly />
      </span>
      <button v-if="closable" class="ni-close" @click.stop="emit('close')">
        <AppIcon name="close-small" :size="14" />
      </button>
    </div>
    <div class="row">
      <span class="box rel">
        <input
          class="box-input"
          :value="fromLabel"
          placeholder="源称谓"
          :disabled="disabled"
          maxlength="20"
          @input="emit('update:fromLabel', ($event.target as HTMLInputElement).value)"
        />
        <AppIcon class="arrow down" name="chevron-small" :size="14" />
      </span>
      <span class="box rel">
        <input
          class="box-input"
          :value="toLabel"
          placeholder="目标称谓"
          :disabled="disabled"
          maxlength="20"
          @input="emit('update:toLabel', ($event.target as HTMLInputElement).value)"
        />
        <AppIcon class="arrow" name="chevron-small" :size="14" />
      </span>
      <span class="ni-close placeholder" aria-hidden="true" />
    </div>
    <div class="row">
      <span class="box origin">
        <span class="origin-name">{{ origin }}</span>
      </span>
      <span class="ni-close placeholder" aria-hidden="true" />
    </div>
  </div>
</template>

<style scoped>
.pr {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--n-space-8);
  width: 240px;
  height: 118px;
  padding: var(--n-space-12);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
}
.box {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-g2);
  border: 1px solid var(--n-g5);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  cursor: text;
}
.box:focus-within {
  border-color: var(--n-l-a1);
}
.box-input {
  flex: 1;
  min-width: 0;
  padding: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  background: transparent;
}
.arrow {
  flex: none;
  color: var(--n-l4);
}
.arrow.down {
  transform: rotate(180deg); /* chevron-small 源图形朝上，左框 ↓ */
}
.box.origin {
  background: var(--n-g4);
  border-color: var(--n-g6);
  cursor: default;
}
.origin-name {
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.placeholder {
  opacity: 0;
  pointer-events: none;
}
</style>
