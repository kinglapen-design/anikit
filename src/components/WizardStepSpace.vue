<script setup lang="ts">
// 向导第一步（38 稿）：输入储存空间名称 + 选择保存路径（选定后路径显示在按钮左侧，标签档截断）。
import TextInput from "./inputs/TextInput.vue";
import ActionButton from "./buttons/ActionButton.vue";

const name = defineModel<string>("name", { required: true });
const parent = defineModel<string>("parent", { required: true });
defineProps<{ busy: boolean; error: string }>();
const emit = defineEmits<{ pick: [] }>();
</script>

<template>
  <div class="step-space">
    <div class="row">
      <span class="field-label">输入储存空间名称:</span>
      <TextInput v-model="name" placeholder="输入储存空间名称..." />
    </div>
    <div class="row path-row">
      <span v-if="parent" class="chosen-path" :title="parent">{{ parent }}</span>
      <ActionButton label="选择保存路径" variant="primary" size="md" :disabled="busy" @click="emit('pick')" />
    </div>
    <p v-if="error" class="step-error">{{ error }}</p>
  </div>
</template>

<style scoped>
/* 内容区（38 稿）：column / justify center / padding 0 48 / gap 12 */
.step-space {
  position: relative;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 0 var(--space-48);
  gap: var(--n-space-12);
  width: 100%;
  flex: 1;
  min-height: 0;
}
.row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-12);
  width: 100%;
}
.field-label {
  flex: none;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.path-row {
  justify-content: flex-end;
}
/* ActionButton md = 宽 100% 填充式（稿 flex-grow 拉伸语境）；38 稿此钮自适应内容——
   压回 auto + 左右 12（旧 AppButton large padding 8/12 口径沿） */
.path-row :deep(.act-btn) {
  width: auto;
  padding: var(--n-space-8) var(--n-space-12);
}
/* 已选路径：标签档截断（稿无此元素，样式全取稿内档位） */
.chosen-path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
/* 步骤内联错误行（Eagle 式，同 CreateLibraryForm 口径）：绝对定位不挤压稿内布局；
   pointer-events none——元素盒横跨内容区与按钮行重叠，纯展示不吃点击（2026-08-12 走查） */
.step-error {
  position: absolute;
  left: var(--space-48);
  right: var(--space-48);
  bottom: var(--n-space-8);
  margin: 0;
  pointer-events: none;
  text-align: center;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-danger);
}
</style>
