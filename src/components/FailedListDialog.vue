<script setup lang="ts">
// 失败清单弹窗（导入 / 导出共用，2026-08-14 用户定）——2026-09-21 换代 modals/ConfirmModal
// danger（§24 嵌套确认映射的 alert 系落地：alert error → kind="danger"，48 error 徽章废、
// 纯靠按钮色区分轻重；次左主右 = 退出左 / 重试右），原 AppModal alert 壳 + AppButton 退役
//（alert 系仅剩 OrphanInfosAlert 待稿）。描述 = 失败文件名依次竖排、间距 8（用户定沿旧），
// path 缺失项回退显示原因防空行。提示音不丢：ConfirmModal 不带音，宿主 App.vue
// watch 弹开态 → playAlert()（§40 口径，deleteLibraryState 同先例）。
import { ref } from "vue";
import ConfirmModal from "./modals/ConfirmModal.vue";
import ScrollThumb from "./ScrollThumb.vue";

defineProps<{
  title: string; // "导入未完成" / "导出未完成"
  failed: { path: string; reason: string }[];
}>();
const emit = defineEmits<{ close: []; retry: [] }>();

function fileName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const listEl = ref<HTMLElement>();
</script>

<template>
  <ConfirmModal
    :title="title"
    kind="danger"
    confirm-label="重试"
    cancel-label="退出"
    @confirm="emit('retry')"
    @cancel="emit('close')"
  >
    <div class="scroll-wrap">
      <div ref="listEl" class="fail-list kit-scroll">
        <p v-for="(f, i) in failed" :key="i" class="fail-item">{{ f.path ? fileName(f.path) : f.reason }}</p>
      </div>
      <ScrollThumb :body="listEl" />
    </div>
  </ConfirmModal>
</template>

<style scoped>
/* 滚动包裹层：relative 锚 ScrollThumb（悬浮滚轮悬停显隐，主窗口统一交互）；
   负 margin 对（2026-09-13 实机走查贴右缘口径沿旧）——margin-right 负扩 24 吃进壳
   padding 贴到弹窗缘，.fail-list padding-right 补回，文字原位不动 */
.scroll-wrap {
  position: relative;
  margin-right: calc(-1 * var(--n-space-24));
}
/* 失败清单：文件名竖排间距 8（用户定）；超高自滚（上限 160 沿旧弹窗）；
   padding-right 补回包裹层负扩 */
.fail-list {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  max-height: 160px;
  padding-right: var(--n-space-24);
}
.fail-item {
  margin: 0;
  word-break: break-all;
}
</style>
