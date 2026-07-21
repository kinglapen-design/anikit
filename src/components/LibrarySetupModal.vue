<script setup lang="ts">
// 创建/打开资源库模态（03_UI 布局/06）：未打开库时强制选择，无"稍后"（设计稿仅两按钮）。
// 三态：initial（插画 + 创建/打开）→ create（07 名称表单）/ error（08 路径错误）。
// 创建：表单输名称 → 系统对话框选保存位置 → 在选中位置建以名称命名的库文件夹并打开；
// 打开：系统对话框选文件夹 → 校验资源库结构直开，失败进错误态（重试 / 取消回初始）。
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import AppIcon from "./AppIcon.vue";
import CreateLibraryForm from "./CreateLibraryForm.vue";
import PathErrorDialog from "./PathErrorDialog.vue";
import { getIcon } from "../lib/icons";

interface LibraryInfo {
  name: string;
  path: string;
  assetCount: number;
  folderCount: number;
}

const emit = defineEmits<{ done: [info: LibraryInfo] }>();

const illustration = getIcon("create-library-illustration");
type View = "initial" | "create" | "error";
const view = ref<View>("initial");
const createName = ref(""); // 名称提升保存：创建失败重试回到表单时不丢
const errorMsg = ref("");
const busy = ref(false);
let retryAction: "open" | "create" = "open";

async function pickFolder(): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}

function toError(e: unknown, action: "open" | "create"): void {
  retryAction = action;
  const text = String(e);
  // 结构校验失败用 08 设计文案；其余错误（损坏 / 版本过高 / IO）显示后端中文文案
  errorMsg.value = text.includes("不是 AniKit 资源库")
    ? "选择的路径中找不到 AniKit 的资源文件，请确认路径是否正确。"
    : text;
  view.value = "error";
}

async function openLibraryFlow(): Promise<void> {
  const path = await pickFolder();
  if (!path || busy.value) return;
  busy.value = true;
  try {
    emit("done", await invoke<LibraryInfo>("open_library", { path }));
  } catch (e) {
    toError(e, "open");
  } finally {
    busy.value = false;
  }
}

async function onCreateSubmit(name: string): Promise<void> {
  const parent = await pickFolder();
  if (!parent || busy.value) return;
  busy.value = true;
  try {
    emit("done", await invoke<LibraryInfo>("create_library", { path: `${parent}/${name}` }));
  } catch (e) {
    toError(e, "create");
  } finally {
    busy.value = false;
  }
}

function onRetry(): void {
  if (retryAction === "create") {
    view.value = "create"; // 名称保留，重新点"选择保存路径"
  } else {
    view.value = "initial";
    void openLibraryFlow(); // 打开失败：直接再弹系统对话框
  }
}
</script>

<template>
  <div class="setup-mask">
    <div v-if="view === 'initial'" class="setup-card">
      <div class="illustration" v-html="illustration" />
      <div class="actions">
        <button class="btn primary" :disabled="busy" @click="view = 'create'">
          <AppIcon name="create-library-large" :size="24" />
          <span>创建资源库</span>
        </button>
        <button class="btn secondary" :disabled="busy" @click="openLibraryFlow">
          <AppIcon name="open-library-large" :size="24" />
          <span>打开资源库</span>
        </button>
      </div>
    </div>
    <CreateLibraryForm
      v-else-if="view === 'create'"
      v-model:name="createName"
      :busy="busy"
      @submit="onCreateSubmit"
      @cancel="view = 'initial'"
    />
    <PathErrorDialog
      v-else
      :message="errorMsg"
      :busy="busy"
      @retry="onRetry"
      @cancel="view = 'initial'"
    />
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
/* 初始卡片：686×500 / C5 / 1px C4-15 描边（inset box-shadow，规范 4）+ 二级窗口投影 / 圆角 8 */
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
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--c4-15);
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
/* 按钮组（01_按钮#06 大标题按钮-大按钮）：带图标 250×52 / 圆角 8 */
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
}
.btn.secondary:hover {
  background: var(--c4-15);
}
.btn:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
