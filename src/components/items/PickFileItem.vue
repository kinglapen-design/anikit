<script setup lang="ts">
// 选择文件-通用 item——docs/UI 重构/Design Token/组件/item/选择模式 item/选择文件-通用
// 卡片 240×34 / padding 8 12：名称（正文 二级）+ 关闭钮（恒显，选择模式 = 移出备选）；
// hover = 通用四级底 + 通用六级描边 + 输入指针；输入态（选中进入）= 通用二级底 + 亮色/半透明-1 描边。
import "./items.css";
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    name: string;
    editing?: boolean; // 输入态（名称框本身就是输入框，速查表 §12.2）
    placeholder?: string; // 输入态占位（空态行「点击卡片选择」，7.1 二十三轮补）
    readonly?: boolean; // 输入态只读（空态行禁用口径，同上）
    closable?: boolean; // 关闭钮显隐（空态占位组无删除钮，旧口径沿）；默认 true
    forceState?: "" | "hover";
  }>(),
  { editing: false, placeholder: undefined, readonly: false, closable: true, forceState: "" },
);

const emit = defineEmits<{ click: []; close: []; "update:name": [v: string] }>();
</script>

<template>
  <div
    class="pk ni-card"
    :class="[{ editing }, forceState ? `force-${forceState}` : '']"
    @click="emit('click')"
  >
    <input
      v-if="editing"
      class="pk-input"
      :value="name"
      :placeholder="placeholder"
      :readonly="readonly"
      @input="emit('update:name', ($event.target as HTMLInputElement).value)"
      @click.stop
    />
    <span v-else class="nm">{{ name }}</span>
    <button v-if="closable" class="ni-close" @click.stop="emit('close')">
      <AppIcon name="close-small" :size="14" />
    </button>
  </div>
</template>

<style scoped>
.pk {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  width: 240px;
  height: 34px;
  padding: var(--n-space-8) var(--n-space-12);
  cursor: text;
}
.pk.editing {
  background: var(--n-g2);
  border-color: var(--n-l-a1);
}
.nm {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.pk-input {
  flex: 1;
  min-width: 0;
  padding: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  background: transparent;
}
</style>
