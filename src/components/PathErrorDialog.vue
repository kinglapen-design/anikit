<script setup lang="ts">
// 路径错误卡片（03_UI 布局/08）：打开 / 创建资源库路径校验失败通用，向导内联展示（无遮罩）。
// 2026-09-21 换代新弹窗口径（用户指「不是新的弹窗样式」）：壳 = g3 底 + 1px 通用/五级真 border +
// 大阴影 + 圆角 8 + padding 24 gap 24（ConfirmModal 壳口径内嵌版；error 徽章废——§24 纯按钮色
// 区分轻重，alert 系徽章全废）；标题 18/26 Bold 一级、正文 13/18 三级；按钮 = ActionButton
// 宽自适应 + 左右 48 次左主右（取消左 / 重新选择路径右——主左次右与 padding 8/16 旧口径废）。
// 宽 fit-content 上限 378（按钮行 328 实需；08 旧 368 上限随换代更新），长路径消息换行。
import ActionButton from "./buttons/ActionButton.vue";

defineProps<{ message: string; busy: boolean }>();
const emit = defineEmits<{ retry: []; cancel: [] }>();
</script>

<template>
  <div class="error-card">
    <h2 class="title">无效路径</h2>
    <p class="hint">{{ message }}</p>
    <div class="actions">
      <ActionButton variant="cancel" size="lg" label="取消" :disabled="busy" @click="emit('cancel')" />
      <ActionButton variant="primary" size="lg" label="重新选择路径" :disabled="busy" @click="emit('retry')" />
    </div>
  </div>
</template>

<style scoped>
/* 卡片壳 = ConfirmModal 口径内嵌版：g3 底 + 1px 通用/五级真 border + 大阴影 + 圆角 8 +
   padding 24 / 列 gap 24；宽度随最宽子项、上限 378（按钮行 328 + padding 48 + 边 2） */
.error-card {
  box-sizing: border-box;
  width: fit-content;
  max-width: 378px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: var(--n-space-24);
  gap: var(--n-space-24);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
}
.title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.hint {
  margin: 0;
  max-width: 100%;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  word-break: break-all; /* 长路径超上限时换行 */
}
/* 按钮组右对齐 gap 8（次左主右）；宽自适应 + 左右 48（BaseModal 按钮罩同口径——
   ActionButton lg 基件 width:100% 压回） */
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--n-space-8);
  width: 100%;
}
.error-card :deep(.act-btn) {
  width: auto;
  padding: var(--n-space-8) 48px;
  flex: none;
}
</style>
