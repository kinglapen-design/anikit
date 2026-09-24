<script setup lang="ts">
// other 不支持预览立姿（原 PreviewPanel other 分支，2026-09-21 R5 抽入 previewSpecs；
// R4 按稿 2026-09-21 落地：布局-中间面板-内容容器-此文件不支持预览）= 居中立姿——
// 格式图标 200 + 文件名（标题档二级）+ 提示「暂不支持此文件格式预览，请用其他应用打开」（正文三级）
// + 按钮组（ActionButton solid md：默认应用打开 / 其他应用打开；
// ws 合成项按钮组隐藏——assetId 假 id 禁发命令，同 PreviewTitleBar isWsCurrent 口径）。
import { computed, inject } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import ActionButton from "./buttons/ActionButton.vue";
import { openWithDialog } from "../lib/placementOps";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

const preview = inject(PREVIEW_SESSION_KEY)!;
// 格式图标（current.thumbPath，200 实寸稿值；kind==='other' 由注册表命中结构保证）
const iconUrl = computed(() => {
  const p = preview.current;
  return p?.thumbPath ? convertFileSrc(p.thumbPath) : null;
});
// ws 合成假 id 禁发 assetId 类命令（PRD 02 步骤 1），打开按钮组隐藏——同 PreviewTitleBar isWsCurrent 口径
const isWsCurrent = computed(() => !!preview.current?.assetId.startsWith("ws:"));
// 其他应用打开 = 系统"打开方式"对话框（失败 toast 在 openWithDialog 内）；默认应用打开直走 store
function onOpenOther(): void {
  const p = preview.current;
  if (!p || isWsCurrent.value) return;
  void openWithDialog(p.assetId);
}
</script>

<template>
  <div class="unsupported">
    <div class="us-body">
      <img v-if="iconUrl" class="us-icon" :src="iconUrl" draggable="false" />
      <div class="us-texts">
        <span class="us-title">{{ preview.current?.name ?? "" }}</span>
        <span class="us-hint">暂不支持此文件格式预览，请用其他应用打开</span>
      </div>
      <div v-if="!isWsCurrent" class="us-actions">
        <ActionButton label="默认应用打开" icon="open-default" @click="preview.openDefaultApp()" />
        <ActionButton label="其他应用打开" icon="open-in-other-app" @click="onOpenOther" />
      </div>
    </div>
  </div>
</template>

<style scoped>
/* other 不支持预览态（稿：布局-中间面板-内容容器-此文件不支持预览）：
   根层铺满画面区内容居中；块 = column 居中 gap 24 + padding-bottom 24（稿「图标+提示+按钮」） */
.unsupported {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
.us-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-24);
  padding-bottom: var(--n-space-24);
}
/* shell-icon：200×200 稿值（格式图标方形缩略图 contain） */
.us-icon {
  width: 200px;
  height: 200px;
  object-fit: contain;
}
/* 标题+提示文字：column 居中 gap 8（稿） */
.us-texts {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-8);
}
/* 文件名 = 标题档 18/26 Bold 二级；超长截断稿未画——同 PreviewTitleBar .index-name 口径
   nowrap 不截断（stage overflow hidden 兜底） */
.us-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
}
.us-hint {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
/* 按钮组：稿值两钮各 150 + gap 8 = 308——ActionButton md/lg 宽 100% 由父容器定格，
   grid 双列均分贴稿（同 EmptyFolderState 居中根因口径，速查 §22） */
.us-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--n-space-8);
  width: 308px;
}
</style>
