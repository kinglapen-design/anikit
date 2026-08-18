<script setup lang="ts">
// 创建资源库弹窗（16_创建资源库 稿，2.1）：菜单"新建资源库"入口（有库在用的场景；
// 初次打开 / 无库场景走 38 向导）。AppModal confirm 骨架：品牌徽章 + 标题 + 无 X；
// 内容 = 名称输入（自动聚焦 / Enter 确认）；按钮组稿序 = 确认左、取消右。
// 创建即打开（afterOpen 收尾在 libraryOps.createLibrary），成功 toast 带音（重要场合口径）。
import { nextTick, onMounted, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppInput from "./AppInput.vue";
import { createLibrary, validateAssetName } from "../lib/libraryOps";
import { useToastStore } from "../stores/toast";

const emit = defineEmits<{ close: [] }>();
const toast = useToastStore();

const name = ref("");
const error = ref("");
const busy = ref(false);
const inputRef = ref<InstanceType<typeof AppInput>>();
onMounted(() => {
  // AppInput 2026-08-19 统一壳化（踩坑 036）：$el 是壳 div 不可 focus——用组件 expose 的 focus()
  void nextTick(() => inputRef.value?.focus());
});

async function onConfirm(): Promise<void> {
  if (busy.value) return;
  const trimmed = name.value.trim();
  const msg = validateAssetName(trimmed);
  if (msg) {
    error.value = msg;
    return;
  }
  busy.value = true;
  try {
    const info = await createLibrary(trimmed); // 自动导入 = 空间级设置，建库不带（2026-08-12 定案）
    toast.notify({ message: `资源库「${info.name}」创建完成`, icon: "success", sound: true, duration: 3000 });
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <AppModal variant="confirm" icon="library-duotone" title="创建资源库" @close="emit('close')">
    <div class="create-library">
      <div class="field-row">
        <span class="field-label">输入资源库名称：</span>
        <AppInput
          ref="inputRef"
          v-model="name"
          placeholder="输入资源库名称..."
          @input="error = ''"
          @keydown.enter="onConfirm"
        />
      </div>
      <p v-if="error" class="error-tip">{{ error }}</p>
    </div>
    <template #actions>
      <AppButton label="确认" variant="primary" :disabled="busy" @click="onConfirm" />
      <AppButton label="取消" :disabled="busy" @click="emit('close')" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 内容区（16 稿）：宽 415（卡 463 - padding 24×2），标签 112 + gap 12 + 输入框自适应 */
.create-library {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  width: 415px;
}
.field-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--space-12);
}
.field-label {
  flex: none;
  width: 112px;
  text-align: right;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
/* 错误提示行（Eagle 式红字，沿用退役 CreateLibraryForm 口径） */
.error-tip {
  margin: 0;
  padding-left: 124px; /* 对齐输入框左缘（label 112 + gap 12） */
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--error);
}
</style>
