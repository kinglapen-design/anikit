<script setup lang="ts">
// 标签页 item——docs/06_UI重构/Design Token/组件/item/标签页 item/ + 布局/中间面板-标签栏
// 选中：通用/一级底 + 圆角 8 8 0 0 + 与中间面板无缝衔接（底部两侧反弧肩膀，radial-gradient 反圆角）；
// 未选中 hover：6px 圆角矩形罩（inset 上下 4 / 左右 0——上下一致文字居中，相邻罩间距 = gap 4
// 全向统一，2026-10-02 用户定）；关闭钮常态仅选中显示。
// 锁定（稿未画）：pin-small 14 替代关闭钮，色 = 未选中亮/三级、选中亮/二级（2026-08-30 用户定）。
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    label: string;
    selected?: boolean;
    closable?: boolean; // 业务态可不显示关闭钮（锁定走 locked，pin 替代）
    locked?: boolean;
    forceState?: "" | "hover" | "selected"; // 预览页强制态
  }>(),
  { selected: false, closable: true, locked: false, forceState: "" },
);

const emit = defineEmits<{ click: []; close: [] }>();
</script>

<template>
  <div
    class="tab"
    :class="[{ selected, locked }, forceState ? `force-${forceState}` : '']"
    @click="emit('click')"
  >
    <span class="label">{{ label }}</span>
    <span v-if="locked" class="pin" v-tip="'已锁定'">
      <AppIcon name="pin-small" :size="14" />
    </span>
    <button v-else-if="closable" class="x" @click.stop="emit('close')">
      <AppIcon name="close-small" :size="14" />
    </button>
  </div>
</template>

<style scoped>
.tab {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 34px; /* 2026-10-01 用户修订：38 → 34 = 文字 18 + 上下 8（标题栏降占位联动） */
  padding: 0 var(--n-space-16);
  min-width: 0;
  border-radius: var(--n-radius-8) var(--n-radius-8) 0 0;
  color: var(--n-l3);
  cursor: pointer;
}
.label {
  position: relative;
  z-index: 1;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  flex: 1;
}
.x {
  position: relative;
  z-index: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex: none;
  padding: 0;
  color: var(--n-l4);
  opacity: 0;
  transition:
    color var(--n-dur-100) var(--n-ease-in-out),
    opacity var(--n-dur-100) var(--n-ease-in-out);
}
.x:hover {
  color: var(--n-l2);
}
/* 锁定 pin（稿未画）：pin-small 14 替代关闭钮（与关闭钮同 14 档，2026-08-30 用户供小图）；
   色档（同日用户定）：未选中 = 亮色/三级，选中 = 亮色/二级 */
.pin {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex: none;
  color: var(--n-l3);
}
.tab.selected .pin,
.tab.force-selected .pin {
  color: var(--n-l2);
}
/* 未选中 hover：6px 圆角矩形罩（亮色/半透明-3）——inset 上下 4 / 左右 0
   （2026-10-02 用户定：上下一致文字居中——30 高顶 4 底 4 版文字偏下被否；
   左右贴 tab 盒缘，相邻标签页罩间距 = 标签页 gap 4 = 罩 bottom 4，全向统一） */
.tab:not(.selected):not(.force-selected)::after {
  content: "";
  position: absolute;
  inset: var(--n-space-4) 0;
  border-radius: var(--n-radius-6);
  background: transparent;
  transition: background var(--n-dur-100) var(--n-ease-in-out);
  pointer-events: none;
}
.tab:not(.selected):not(.force-selected):hover::after,
.tab.force-hover::after {
  background: var(--n-l-a3);
}
.tab:hover .x,
.tab.force-hover .x {
  opacity: 1;
}
/* 选中：通用/一级底与中间面板同色无缝；底部两侧反弧肩膀（4px 反圆角方块——
   2026-08-28 走查定：弧径 = 标签间隙 4，hover 罩层（四周 4px 内缩）右下间距才能统一为 4） */
.tab.selected,
.tab.force-selected {
  background: var(--n-g1);
  color: var(--n-l1);
}
.tab.selected::before,
.tab.selected::after,
.tab.force-selected::before,
.tab.force-selected::after {
  content: "";
  position: absolute;
  bottom: 0;
  width: 4px;
  height: 4px;
  pointer-events: none;
}
.tab.selected::before,
.tab.force-selected::before {
  left: -4px;
  background: radial-gradient(circle at 0 0, transparent 3.5px, var(--n-g1) 4px);
}
.tab.selected::after,
.tab.force-selected::after {
  right: -4px;
  background: radial-gradient(circle at 100% 0, transparent 3.5px, var(--n-g1) 4px);
}
.tab.selected .x,
.tab.force-selected .x {
  opacity: 1;
}
</style>
