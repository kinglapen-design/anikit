<script setup lang="ts">
// 初次打开向导（38_向导稿，2.1）：主窗口内全屏层 + 600×559 卡片
//（C2 底 / 亮-选中 1.5px 边框 / 投影 / 圆角 8，标题栏 46 右侧 X）。
// 帧 0 入口（新建 / 打开储存空间）→ 第一步 配置储存空间 → 第二步 创建资源库（含自动导入开关）
// → 第三 / 四步 浏览器扩展、桥接插件占位（后续版本，PRD 01_储存空间 3.1）。
// startStep：0 = 初次打开完整流程；2 = 菜单"新建资源库"直进第二步（standalone：第二步"完成"即落库）。
// 打开已有空间：非空间结构进 error 态（PathErrorDialog 复用）；有库直开最近库，无库进第二步。
// 落盘时机（2026-08-12 用户走查定案）：前两步"下一步"仅校验暂存，第四步"完成"才
// create_space → create_library——中途关闭 = 什么都没建，重启再弹向导。
import { computed, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import IconButton from "./buttons/IconButton.vue";
import ActionButton from "./buttons/ActionButton.vue";
import PathErrorDialog from "./PathErrorDialog.vue";
import WizardStepSpace from "./WizardStepSpace.vue";
import WizardStepLibrary from "./WizardStepLibrary.vue";
import WizardStepPlaceholder, { type PlaceholderIcon } from "./WizardStepPlaceholder.vue";
import logoUrl from "../../assets/image/anikit-logo.png?url";
import { useToastStore } from "../stores/toast";
import {
  createSpace,
  openSpace,
  listSpaceLibraries,
  createLibrary,
  openLibrary,
  setAutoImport,
  validateAssetName,
  currentSpace,
  type LibraryEntry,
} from "../lib/libraryOps";

const props = withDefaults(defineProps<{ closable?: boolean; startStep?: number }>(), {
  closable: false,
  startStep: 0,
});
const emit = defineEmits<{ done: []; cancel: [] }>();

// 正式 LOGO（design/LOGO/02_Final_PNG 彩色版，2026-09-20 用户供稿入库 assets/image/）
const toast = useToastStore();

const standalone = props.startStep >= 2; // 菜单"新建资源库"模式：仅第二步
const step = ref(props.startStep);
const view = ref<"wizard" | "error">("wizard");
const busy = ref(false);
const errorMsg = ref(""); // error 态（打开空间失败）文案
const stepError = ref(""); // 步骤内联错误行（Eagle 式红字）

// 步骤数据（上一步回退不丢）
const spaceName = ref("");
const parentPath = ref("");
const libName = ref("");
const autoImport = ref(false);
// 空间是否已就绪（打开已有空间 / standalone 菜单新建库场景 = true）：finish 时跳过建空间只建库
const spaceReady = ref(props.startStep >= 2);

const STEP_TITLES = ["", "配置储存空间", "创建资源库", "安装浏览器插件", "安装桥接插件"];
const STEP_NO = ["", "一", "二", "三", "四"];
const isLastStep = computed(() => (standalone ? step.value === 2 : step.value === 4));
const prevVisible = computed(() => step.value > (standalone ? 2 : 1));

// 第三 / 四步占位图标（用户导出的 on/off 双态 SVG，2026-08-12）：第三步 chrome/firefox/edge，
// 第四步 An / Ps / Ae（稿序）
const BROWSER_ICONS: PlaceholderIcon[] = [
  { off: "chrome-off", on: "chrome-on" },
  { off: "firefox-off", on: "firefox-on" },
  { off: "edge-off", on: "edge-on" },
];
const BRIDGE_ICONS: PlaceholderIcon[] = [
  { off: "animate-off", on: "animate-on" },
  { off: "photoshop-off", on: "photoshop-on" },
  { off: "ae-off", on: "ae-on" },
];

// 关闭（2026-08-12 用户定）：closable（菜单触发，有库在用）= 关向导回主界面；
// 强制场景（无空间无库）= 直接退出程序——没完成连储存空间都没有，打开软件也没意义
function onClose(): void {
  if (props.closable) emit("cancel");
  else void invoke("exit_app");
}

async function pickDir(): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}

// 第一步：选择保存路径（父目录）
async function pickParent(): Promise<void> {
  const dir = await pickDir();
  if (dir) {
    parentPath.value = dir;
    stepError.value = "";
  }
}

// 帧 0：打开储存空间——有库开最近库，无库进第二步
async function openSpaceFlow(): Promise<void> {
  const path = await pickDir();
  if (!path || busy.value) return;
  busy.value = true;
  try {
    const space = await openSpace(path);
    spaceReady.value = true;
    autoImport.value = space.autoImport; // 第二步开关回显空间当前设置（空间级，2026-08-12）
    const libs = await listSpaceLibraries();
    if (libs.length === 0) {
      step.value = 2;
      return;
    }
    const recent = await invoke<LibraryEntry[]>("list_recent_libraries").catch(() => [] as LibraryEntry[]);
    const hit = recent.find((r) => r.path.startsWith(space.path) && libs.includes(r.name));
    const info = await openLibrary(hit?.name ?? libs[0]);
    toast.notify({ message: `已打开资源库「${info.name}」`, icon: "success", duration: 3000 });
    emit("done");
  } catch (e) {
    errorMsg.value = String(e); // 后端中文文案（"该文件夹不是 AniKit 储存空间" 等）
    view.value = "error";
  } finally {
    busy.value = false;
  }
}

// 第一步"下一步"：仅校验暂存不落盘（2026-08-12 用户走查定案：向导点"完成"才真正创建，
// 中途关闭 = 什么都没建、重启再弹向导）
function submitSpace(): void {
  const msg = validateAssetName(spaceName.value.trim());
  if (msg) {
    stepError.value = msg;
    return;
  }
  if (!parentPath.value) {
    stepError.value = "请点击下方按钮选择保存路径";
    return;
  }
  step.value = 2;
}

// 第二步"下一步"：完整流程仅校验暂存；standalone（菜单新建资源库）= 完成即落库
function submitLibrary(): void {
  const msg = validateAssetName(libName.value.trim());
  if (msg) {
    stepError.value = msg;
    return;
  }
  if (standalone) void finish();
  else step.value = 3;
}

function onNext(): void {
  stepError.value = "";
  if (step.value === 1) submitSpace();
  else if (step.value === 2) submitLibrary();
  else if (isLastStep.value) void finish();
  else step.value += 1;
}

function onPrev(): void {
  stepError.value = "";
  if (prevVisible.value) step.value -= 1;
}

// "完成"：此处才真正落盘——建空间（未就绪时）→ 建库；失败回对应步骤显示错误行
async function finish(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    if (!spaceReady.value) {
      try {
        await createSpace(parentPath.value, spaceName.value.trim(), autoImport.value);
        spaceReady.value = true;
      } catch (e) {
        stepError.value = String(e);
        step.value = 1;
        return;
      }
    } else if (autoImport.value !== currentSpace.autoImport) {
      // 空间已开（打开已有空间 / standalone）：开关变更写空间级设置
      if (!(await setAutoImport(autoImport.value))) return;
    }
    const name = libName.value.trim();
    try {
      await createLibrary(name);
    } catch (e) {
      stepError.value = String(e);
      step.value = 2;
      return;
    }
    toast.notify({ message: `资源库「${name}」创建完成`, icon: "success", sound: true, duration: 3000 });
    emit("done");
  } finally {
    busy.value = false;
  }
}

// error 态重试：回向导并直接再弹系统目录选择
function onErrorRetry(): void {
  view.value = "wizard";
  void openSpaceFlow();
}
</script>

<template>
  <div class="wizard-mask">
    <PathErrorDialog
      v-if="view === 'error'"
      :message="errorMsg"
      :busy="busy"
      @retry="onErrorRetry"
      @cancel="view = 'wizard'"
    />
    <div v-else class="wizard-card">
      <div class="titlebar">
        <IconButton name="window-close" title="关闭" @click="onClose" />
      </div>
      <div class="logo-area">
        <img class="logo" :src="logoUrl" alt="AniKit" />
        <div class="app-text">
          <span class="app-name">AniKit</span>
          <span class="app-sub">动画资产管理工具</span>
        </div>
      </div>
      <!-- 帧 0：入口（无步骤区） -->
      <div v-if="step === 0" class="content entry">
        <div class="entry-actions">
          <ActionButton label="新建储存空间" variant="primary" size="md" :disabled="busy" @click="step = 1" />
          <ActionButton label="打开储存空间" variant="solid" size="md" :disabled="busy" @click="openSpaceFlow" />
        </div>
      </div>
      <WizardStepSpace
        v-else-if="step === 1"
        v-model:name="spaceName"
        v-model:parent="parentPath"
        :busy="busy"
        :error="stepError"
        @pick="pickParent"
      />
      <WizardStepLibrary
        v-else-if="step === 2"
        v-model:name="libName"
        v-model:auto-import="autoImport"
        :error="stepError"
      />
      <WizardStepPlaceholder
        v-else
        :icons="step === 3 ? BROWSER_ICONS : BRIDGE_ICONS"
        :text="step === 3 ? '浏览器扩展安装将在后续版本提供' : '桥接插件安装将在后续版本提供'"
      />
      <!-- 步骤区：上一步 / 步骤标题 / 下一步·完成 + 进度条（帧 0 无） -->
      <div v-if="step > 0" class="step-area">
        <div class="step-row">
          <ActionButton
            label="上一步"
            variant="solid"
            size="sm"
            class="prev-btn"
            :class="{ ghost: !prevVisible }"
            :disabled="busy"
            @click="onPrev"
          />
          <span class="step-title">
            <span class="step-no">第{{ STEP_NO[step] }}步：</span>
            <span class="step-name">{{ STEP_TITLES[step] }}</span>
          </span>
          <ActionButton
            :label="isLastStep ? '完成' : '下一步'"
            variant="primary"
            size="sm"
            :disabled="busy"
            @click="onNext"
          />
        </div>
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: `${(step / 4) * 100}%` }" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩：向导期间不显示主窗口（2026-08-12 用户定），纯色 C1 不透明 */
.wizard-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-g1);
}
/* 卡片（38 稿）：600×559 / g3 底（二级面板）/ 1px 半透明-2 inset 描边（2026-09-20 R3 换代）/ 投影 / 圆角 8 */
.wizard-card {
  box-sizing: border-box;
  width: 600px;
  height: 559px;
  display: flex;
  flex-direction: column;
  align-items: center;
  background: var(--n-g3);
  box-shadow:
    var(--n-shadow-lg),
    inset 0 0 0 1px var(--n-l-a2);
  border-radius: var(--n-radius-8);
}
/* 标题栏：46 高，右对齐关闭钮 */
.titlebar {
  box-sizing: border-box;
  display: flex;
  flex-direction: row;
  justify-content: flex-end;
  align-items: flex-start;
  padding: var(--n-space-8);
  width: 100%;
  height: 46px;
  flex: none;
}
/* LOGO 区：271 高 / padding 24 / gap 12 */
.logo-area {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  padding: var(--n-space-24);
  gap: var(--n-space-12);
  width: 100%;
  height: 271px;
  flex: none;
}
.logo {
  width: 167px;
  height: 145px;
  flex: none;
  object-fit: contain;
}
.app-text {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-6);
}
.app-name {
  font-size: var(--n-fs-display);
  line-height: var(--n-lh-display);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.app-sub {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
/* 帧 0 内容区：撑满剩余，按钮组居中（gap 12） */
.content {
  flex: 1;
  width: 100%;
}
.entry {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  padding: var(--n-space-24);
  box-sizing: border-box;
}
.entry-actions {
  display: flex;
  flex-direction: row;
  gap: var(--n-space-12);
}
/* ActionButton md = 宽 100% 填充式（稿 flex-grow 拉伸语境）；38 稿入口两钮自适应内容——
   压回 auto + 左右 12（旧 AppButton large padding 8/12 口径沿） */
.entry-actions :deep(.act-btn) {
  width: auto;
  padding: var(--n-space-8) var(--n-space-12);
}
/* 步骤区：146 高 / padding 48 / gap 12（上行 28 + 进度条 10） */
.step-area {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  padding: var(--space-48);
  gap: var(--n-space-12);
  width: 100%;
  height: 146px;
  flex: none;
}
.step-row {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  height: 28px;
}
/* 第一步 / standalone 无上一步：opacity 0 占位保布局（38 稿） */
.prev-btn.ghost {
  opacity: 0;
  pointer-events: none;
}
.step-title {
  display: flex;
  flex-direction: row;
  gap: var(--n-space-4);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.step-no {
  color: var(--n-l3);
}
.step-name {
  color: var(--n-l1);
}
/* 进度条：504 通栏 × 10 / C1 轨道 / C5-底色填充，按 当前步/4 递增 */
.progress-track {
  width: 100%;
  height: 10px;
  background: var(--n-g1);
  border-radius: var(--n-radius-pill);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  background: var(--n-g4);
  border-radius: var(--n-radius-pill);
  transition: width 0.2s ease;
}
</style>
