<script setup lang="ts">
// 创建资源库弹窗（2026-09-08 换代 = 布局/模态窗口/创建资源库 稿；旧 AppModal 16 稿口径随本件退役）：
// 菜单"新建资源库"入口（有库在用的场景；初次打开 / 无库场景走 38 向导）。
// 壳 = modals/BaseModal（400 宽）；输入 = inputs/TextInput lg（占位「名称」，自动聚焦 / Enter 确认）；
// 按钮 = buttons/ActionButton lg（取消左 cancel / 确认右 primary——次左主右新稿口径，旧确认左序废除）。
// 错误提示行稿未画——功能必需（名称校验 / 后端报错），就近正文档 13/18 语义色 --error。
// 业务照旧：创建即打开（afterOpen 收尾在 libraryOps.createLibrary），成功 toast 带音（重要场合口径）。
import { nextTick, onMounted, ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import TextInput from "./inputs/TextInput.vue";
import ActionButton from "./buttons/ActionButton.vue";
import { createLibrary, validateAssetName } from "../lib/libraryOps";
import { useToastStore } from "../stores/toast";

const emit = defineEmits<{ close: [] }>();
const toast = useToastStore();

const name = ref("");
const error = ref("");
const busy = ref(false);
const inputRef = ref<InstanceType<typeof TextInput>>();
onMounted(() => {
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
  <BaseModal title="创建资源库" :width="400">
    <div class="field">
      <TextInput
        ref="inputRef"
        v-model="name"
        placeholder="名称"
        @input="error = ''"
        @keydown.enter="onConfirm"
      />
      <p v-if="error" class="error-tip">{{ error }}</p>
    </div>
    <template #actions>
      <ActionButton label="取消" variant="cancel" size="lg" @click="emit('close')" />
      <ActionButton label="确认" variant="primary" size="lg" @click="onConfirm" />
    </template>
  </BaseModal>
</template>

<style scoped>
/* 输入段：输入框 + 错误行（稿未画错误态——功能必需，正文档 13/18 语义色） */
.field {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
.error-tip {
  margin: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-danger);
  word-break: break-all;
}
</style>
