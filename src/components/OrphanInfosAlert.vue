<script setup lang="ts">
// 无主资产警告弹窗（2026-09-02 资产污染修复配套）：开库自检扫出孤儿 info
//（数据层无记录、磁盘残留、库内不可见）→ 弹窗列清单。
// 2026-09-21 换代（用户定稿）：modals/ConfirmModal warning 单钮形态（cancelLabel 空 = 仅「知道了」；
// 48 warning 徽章废，§24 纯按钮色区分轻重）+ 列表行学 md 代码块（MCP 接入配置行同款口径：
// 圆角 6 / 底与描边 亮色/半透明-4 / padding 6·8）+ 行尾「在资源管理器中打开」
//（IconButton open-in-explorer bare sm → lib/libraryOps revealOrphanInfo；删除按钮仍未做）。
// 提示音不丢：ConfirmModal 不带音，组件内 watch 弹开态 → playAlert()（GroupManage 同先例）。
import { ref, watch } from "vue";
import ConfirmModal from "./modals/ConfirmModal.vue";
import IconButton from "./buttons/IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { orphanInfosState, revealOrphanInfo } from "../lib/libraryOps";
import { playAlert } from "../lib/sounds";

watch(
  () => orphanInfosState.list,
  (v) => {
    if (v) playAlert();
  },
);

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const listEl = ref<HTMLElement>();
</script>

<template>
  <ConfirmModal
    v-if="orphanInfosState.list"
    kind="warning"
    :title="`发现 ${orphanInfosState.list.length} 个无主资产文件`"
    confirm-label="知道了"
    cancel-label=""
    @confirm="orphanInfosState.list = null"
  >
    <p class="lead">以下文件残留在素材库目录中，但数据层没有对应记录（可能是删除中断的残留），不会显示在库内：</p>
    <div class="scroll-wrap">
      <div ref="listEl" class="orphan-list kit-scroll">
        <div v-for="o in orphanInfosState.list" :key="o.id" class="o-row">
          <span class="o-text"><b>{{ o.name || o.id }}</b>：{{ o.path }}</span>
          <IconButton
            name="open-in-explorer"
            variant="bare"
            size="sm"
            title="在资源管理器中打开"
            @click="revealOrphanInfo(o.path)"
          />
        </div>
      </div>
      <ScrollThumb :body="listEl" />
    </div>
  </ConfirmModal>
</template>

<style scoped>
/* 说明行：正文 13/18 三级（ConfirmModal body 档），与列表间距 8（沿旧） */
.lead {
  margin: 0 0 var(--n-space-8);
}
/* 滚动包裹层：relative 锚 ScrollThumb；负 margin 对（贴右缘口径）——
   margin-right 负扩 24 吃进壳 padding 贴到弹窗缘，.orphan-list padding-right 补回 */
.scroll-wrap {
  position: relative;
  margin-right: calc(-1 * var(--n-space-24));
}
/* 清单：行间距 8 沿旧；超高自滚（上限 160 沿旧）；padding-right 补回包裹层负扩 */
.orphan-list {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  max-height: 160px;
  padding-right: var(--n-space-24);
}
/* 清单行 = md 代码块口径（2026-09-20 用户定，MCP 接入配置行同款）：
   圆角 6 / 底色与描边 亮色/半透明-4（描边 inside 不占空间）/ padding 6·8 / 图标钮贴右 */
.o-row {
  display: flex;
  align-items: flex-start;
  gap: var(--n-space-6);
  padding: var(--n-space-6) var(--n-space-8);
  background: var(--n-l-a4);
  box-shadow: inset 0 0 0 1px var(--n-l-a4);
  border-radius: var(--n-radius-6);
}
.o-text {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.o-text b {
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.o-row .ic-btn {
  flex: none;
  margin-top: 1px; /* 与首行文字基线对齐（14 盒对 18 行高） */
}
</style>
