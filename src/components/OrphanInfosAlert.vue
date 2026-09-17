<script setup lang="ts">
// 无主资产警告弹窗（2026-09-02 资产污染修复配套）：开库自检扫出孤儿 info
//（数据层无记录、磁盘残留、库内不可见）→ AppModal alert 警告模板列清单（figma 09）。
// 简单版：只展示 +「知道了」；完整管理入口（打开资源管理器查看 / 删除按钮）用户后续补稿。
// 结构照 FailedListDialog（悬浮滚轮 / 限宽 372 正文口径 / 清单超高自滚）。
import { ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { orphanInfosState } from "../lib/libraryOps";

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const listEl = ref<HTMLElement>();
</script>

<template>
  <AppModal
    v-if="orphanInfosState.list"
    variant="alert"
    alert-icon="warning"
    :title="`发现 ${orphanInfosState.list.length} 个无主资产文件`"
    class="orphan-alert"
    @close="orphanInfosState.list = null"
  >
    <p class="lead">以下文件残留在素材库目录中，但数据层没有对应记录（可能是删除中断的残留），不会显示在库内：</p>
    <div class="scroll-wrap">
      <div ref="listEl" class="orphan-list kit-scroll">
        <div v-for="o in orphanInfosState.list" :key="o.id" class="row">
          <span class="name">{{ o.name || o.id }}</span>
          <span class="path">{{ o.path }}</span>
        </div>
      </div>
      <ScrollThumb :body="listEl" />
    </div>
    <template #actions>
      <AppButton variant="primary" class="warn" label="知道了" @click="orphanInfosState.list = null" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 说明行：正文档常态色（同 AppConfirm 正文口径） */
.lead {
  margin: 0;
  max-width: 372px;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
/* 滚动包裹层：relative 锚 ScrollThumb；限宽 372（同 AppConfirm 正文口径）；
   负 margin 对（2026-09-13 实机走查贴右缘口径）——margin-right 负扩 24 吃进壳 padding
   贴到弹窗缘（alert 变体无 .content，路径无裁剪祖先），.orphan-list padding-right 补回 */
.scroll-wrap {
  position: relative;
  max-width: 372px;
  margin-top: var(--space-8);
  margin-right: calc(-1 * var(--space-24));
}
/* 清单：名称 + 路径双行竖排，间距 8；超高自滚（上限同 FailedListDialog 160）；
   padding-right 补回包裹层负扩，文字原位不动（2026-09-13 贴右缘口径） */
.orphan-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  max-height: 160px;
  padding-right: var(--space-24);
}
.row {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.path {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  word-break: break-all;
}
/* 确认钮 = 警告色（figma 09 警告弹窗稿：状态/警告 #DB880C = --n-warning）；
   88 min-width（速查 §1 按钮组定宽口径） */
.orphan-alert :deep(.actions) .app-btn {
  min-width: 88px;
}
.orphan-alert :deep(.app-btn.warn) {
  background: var(--n-warning);
}
</style>
