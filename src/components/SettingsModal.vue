<script setup lang="ts">
// 偏好设置弹窗（2026-09-19，MCP 二期写入闸配套，PRD 08-06 形态 A）。
// 壳 = modals/BaseModal framed 变体（标题栏 54 带底线与关闭 X——设置类弹窗无确认/取消语义，
// X 为关闭通道；遮罩点击不关闭口径不变）；宽 400 就近套创建资源库稿值（偏好设置稿未出，待补稿校）。
// 内容最小化：一节「AI 外接通道（MCP）」= 段标 + 开关行 + 说明。
// 段标 = SectionTitle block 标题档同源 token（12/16 Bold 四级）——静态段不带折叠交互，
// 不用 SectionTitle 组件本体（其折叠钮 / hover 动作组 / grab 指针均为可折叠区块 affordance）；
// 开关行 = buttons/SwitchButton + 标签（开关按钮稿 auto layout：开关左 + 文字右 gap 8；
// 标签字档稿未给，就近定档 正文 13/18 二级）；说明 = 正文 13/18 三级（ConfirmModal 正文 / 合并稿 hint 同档）。
// 业务零逻辑：开关直绑 ui.mcpWriteEnabled（持久化走 ui store 既有防抖写回 settings.ui 通道）；
// MCP 写工具侧门禁 = server/mcp write_gate 逐次直读 config.json。
import { onBeforeUnmount, onMounted } from "vue";
import { useUiStore } from "../stores/ui";
import BaseModal from "./modals/BaseModal.vue";
import SwitchButton from "./buttons/SwitchButton.vue";

const ui = useUiStore();
const emit = defineEmits<{ close: [] }>();

// Esc 关闭（RecycleConfirmDialog 同先例；遮罩点击不关闭口径不变）
function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey, true));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey, true));
</script>

<template>
  <BaseModal title="偏好设置" :width="400" framed @close="emit('close')">
    <section class="sec">
      <h3 class="sec-label">AI 外接通道（MCP）</h3>
      <div class="row">
        <SwitchButton v-model="ui.mcpWriteEnabled" />
        <span class="row-label">允许 MCP 写入</span>
      </div>
      <p class="sec-desc">开启后外部 Agent 可新建文档 / 写文本 / 改名；文本内容编辑不进撤销。</p>
    </section>
  </BaseModal>
</template>

<style scoped>
/* framed 内容段自承 padding 0 24（移动稿口径）；底 24 呼吸 = 非 framed 壳 padding 24 同档；
   段内 标→行→说明 列 gap 8 */
.sec {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  padding: 0 var(--n-space-24) var(--n-space-24);
}
/* 段标 = SectionTitle block 标题档（12/16 Bold 四级）同源 token，静态段不带折叠交互 */
.sec-label {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l4);
}
/* 开关行（开关按钮稿 auto layout）：开关左 + 文字右 gap 8 */
.row {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
}
.row-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
/* 说明 = 正文 13/18 三级（ConfirmModal 正文 / 合并稿 hint 同档） */
.sec-desc {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
</style>
