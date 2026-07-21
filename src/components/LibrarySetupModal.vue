<script setup lang="ts">
// 创建/打开资源库模态（03_UI 布局/06）：未打开库时强制选择，无"稍后"（设计稿仅两按钮）。
// 流程：创建 → 系统对话框选文件夹 → 中间行显示路径 → 确认路径建库 / 重新选择回初始；
//       打开 → 系统对话框选文件夹 → 校验资源库结构直接打开，失败显示错误行。
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import AppIcon from "./AppIcon.vue";
import { getIcon } from "../lib/icons";

interface LibraryInfo {
  name: string;
  path: string;
  assetCount: number;
  folderCount: number;
}

const emit = defineEmits<{ done: [info: LibraryInfo] }>();

const illustration = getIcon("create-library-illustration");
const confirmingPath = ref<string | null>(null); // 非空 = 确认路径态
const error = ref("");
const busy = ref(false);

async function pickFolder(): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}

async function onCreate(): Promise<void> {
  if (busy.value) return;
  error.value = "";
  const path = await pickFolder();
  if (path) confirmingPath.value = path;
}

async function onConfirm(): Promise<void> {
  if (!confirmingPath.value || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    const info = await invoke<LibraryInfo>("create_library", { path: confirmingPath.value });
    emit("done", info);
  } catch (e) {
    error.value = String(e);
    confirmingPath.value = null; // 失败回初始态，错误行保留
  } finally {
    busy.value = false;
  }
}

async function onOpen(): Promise<void> {
  if (busy.value) return;
  error.value = "";
  const path = await pickFolder();
  if (!path) return;
  busy.value = true;
  try {
    const info = await invoke<LibraryInfo>("open_library", { path });
    emit("done", info);
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="setup-mask">
    <div class="setup-card">
      <div class="illustration" v-html="illustration" />
      <p v-if="confirmingPath" class="message" :title="confirmingPath">{{ confirmingPath }}</p>
      <p v-else-if="error" class="message error">{{ error }}</p>
      <div class="actions">
        <template v-if="confirmingPath">
          <button class="btn primary text-only" :disabled="busy" @click="onConfirm">确认路径</button>
          <button class="btn secondary text-only" :disabled="busy" @click="confirmingPath = null">
            重新选择
          </button>
        </template>
        <template v-else>
          <button class="btn primary" :disabled="busy" @click="onCreate">
            <AppIcon name="create-library-large" :size="24" />
            <span>创建资源库</span>
          </button>
          <button class="btn secondary" :disabled="busy" @click="onOpen">
            <AppIcon name="open-library-large" :size="24" />
            <span>打开资源库</span>
          </button>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩：挡住主界面交互（C1-80 现有 token） */
.setup-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 卡片：686×500 / C5 底 / 1px C4-15 描边（规范 4：inset box-shadow 模拟）/ 圆角 8 */
.setup-card {
  box-sizing: border-box;
  width: 686px;
  min-height: 500px;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--space-16);
  gap: var(--space-16);
  background: var(--c5);
  box-shadow: inset 0 0 0 1px var(--c4-15);
  border-radius: var(--radius-8);
}
.illustration {
  width: 654px;
  height: 368px;
  flex: none;
}
.illustration :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
/* 路径 / 错误行：正文样式（确认路径 C4-70，错误用语义色 --error） */
.message {
  margin: 0;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  font-weight: var(--text-body-weight);
  color: var(--c4-70);
}
.message.error {
  color: var(--error);
  white-space: normal;
}
/* 按钮组 */
.actions {
  box-sizing: border-box;
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  justify-content: center;
  padding: var(--space-16) 0;
  gap: var(--space-16);
  width: 516px;
  min-height: 84px;
}
.btn {
  display: flex;
  flex-direction: row;
  justify-content: center;
  align-items: center;
  padding: 14px 0;
  gap: var(--space-8);
  width: 250px;
  height: 52px;
  border: none;
  border-radius: var(--radius-8);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.secondary {
  background: var(--c4-7);
  height: 50px;
}
.btn.secondary:hover {
  background: var(--c4-15);
}
/* 确认路径 / 重新选择：纯文字、宽 200（用户确认的调整） */
.btn.text-only {
  width: 200px;
}
.btn:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
