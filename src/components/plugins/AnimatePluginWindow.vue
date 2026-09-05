<script setup lang="ts">
// Animate 插件详情子窗口（稿 = docs/UI 重构/Design Token/布局/主窗口/工具栏展开窗口/Animate 插件.md，
// 544×538 非模态独立窗，可移动/缩放、单例聚焦——open_plugin_window 后端命令）：
// 经 URL ?plugin=animate 进入（App.vue query 分发，?preview= 先例）；窗口壳（fx 云母/实色、圆角、
// 描边、阴影）由 App.vue .window 承担（预览子窗口同口径），本组件 = 内容面板：padding 24 / 列 gap 24。
// 标题行 = 窗口拖拽区（data-tauri-drag-region，按钮区不挂）；「关闭」钮与 Esc = 关窗口（window.close）；
// 内容溢出（窗口缩放）时中部组区纵向 ScrollThumb 悬浮滚轮（速查 §2）。
// 安装 / 卸载业务移植自退役的 modals/CepSettingsDialog（状态查询 / 安装 / 卸载 + toast，
// 封装 lib/plugins）；按钮族复用 ActionButton lg——关闭 = cancel 变体（稿 通用/五级底 128×38）、
// 卸载 = danger 变体（稿 状态/禁止底 128×38，未安装 disabled）、安装 = primary（稿 品牌/常态 200×38，
// 已安装文案「重新安装 / 升级」）。
// 文案贴稿：使用教程第 1 条稿原文「左下角」改「右下角」（稿按钮实际在右下，2026-09-06 任务定）；
// 路径组 = CEP 安装路径（get_cep_install_status.installDir 后端新字段）+ Temp（库 root\temp\cep-out，
// 未开库显示「未开库」）。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ActionButton from "../buttons/ActionButton.vue";
import ScrollThumb from "../ScrollThumb.vue";
import {
  getCepInstallStatus,
  installCepPlugin,
  uninstallCepPlugin,
  type CepInstallStatus,
} from "../../lib/plugins";
import type { LibraryInfo } from "../../lib/libraryOps";

const install = ref<CepInstallStatus | null>(null);
const libPath = ref<string | null>(null); // 库 root（Temp 行）；null = 未开库 / 未取到
const busy = ref(false);
const bodyRef = ref<HTMLElement>();

onMounted(async () => {
  install.value = await getCepInstallStatus();
  try {
    const lib = await invoke<LibraryInfo | null>("get_current_library");
    libPath.value = lib?.path ?? null;
  } catch {
    libPath.value = null;
  }
  window.addEventListener("keydown", onKeydown, true);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown, true);
});

// Esc = 关窗口（与「关闭」钮同径；本窗无菜单/输入框争用 Esc）
function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") void getCurrentWindow().close();
}
function closeWin(): void {
  void getCurrentWindow().close();
}

async function doInstall(): Promise<void> {
  busy.value = true;
  try {
    const st = await installCepPlugin();
    if (st) install.value = st;
  } finally {
    busy.value = false;
  }
}

async function doUninstall(): Promise<void> {
  busy.value = true;
  try {
    const st = await uninstallCepPlugin();
    if (st) install.value = st;
  } finally {
    busy.value = false;
  }
}

// 路径组文案：install_dir 无法定位系统配置目录时后端给空串；状态未返回 = 读取中
const installDirText = computed(() => {
  if (!install.value) return "读取中…";
  return install.value.installDir || "未检测到";
});
const tempText = computed(() => (libPath.value ? `${libPath.value}\\temp\\cep-out` : "未开库"));
</script>

<template>
  <div class="plugin-window">
    <!-- 标题（稿：18/26 Bold 一级）；标题行区域 = 窗口拖拽区 -->
    <h1 class="pw-title" data-tauri-drag-region>Animate 插件</h1>
    <!-- 组区（稿 介绍文本：列 gap 24；组 = 标题 16/22 Bold 二级 + 行组 gap 6 的 13/18 三级） -->
    <div class="pw-scroll-wrap">
      <div ref="bodyRef" class="pw-body kit-scroll">
        <section class="pw-group">
          <h2 class="pw-sub">使用教程</h2>
          <ul class="pw-lines">
            <li>点击右下角的 <b>[安装]</b> 按钮，安装 Animate CEP 桥接插件。</li>
            <li>可以在 Animate <b>[窗口/扩展/AniKit]</b> 里打开 AniKit 面板。</li>
            <li>安装后，右键菜单会添加 <b>[发送至 Animate]</b> 的菜单项。</li>
          </ul>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">功能列表</h2>
          <ul class="pw-lines">
            <li>选中 Animate 的元件， CEP 里导出元件为新 fla 文档并附带缩略图。</li>
            <li>
              在 CEP 面板可选择快速导出当前 fla 文档的缩略图，若 fla 文档本就在资源库中，则缩略图会自动导入至素材库，并链接至
              fla 文档；若 fla 文档不在资源库中，则缩略图会保存在原 fla 文档的同一路径下。
            </li>
            <li>对资源库里的 fla 文档右键，则可看见 <b>[发送至 Animate]</b> 的菜单，一键将 fla 资产快速导入至 Animate 舞台。</li>
          </ul>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">路径</h2>
          <ul class="pw-lines">
            <li class="pw-path">CEP 安装路径: {{ installDirText }}</li>
            <li class="pw-path">Temp: {{ tempText }}</li>
          </ul>
        </section>
      </div>
      <ScrollThumb :body="bodyRef" />
    </div>
    <!-- 按钮行（稿：右对齐 gap 8 / 高 38；关闭 128 + 卸载 128 + 安装 200 稿值定宽槽，
         ActionButton lg 填充式宽 100% 由槽定格） -->
    <div class="pw-actions">
      <span class="pw-btn" style="width: 128px">
        <ActionButton label="关闭" variant="cancel" size="lg" @click="closeWin" />
      </span>
      <span class="pw-btn" style="width: 128px">
        <ActionButton label="卸载" variant="danger" size="lg" :disabled="busy || !install?.installed" @click="doUninstall" />
      </span>
      <span class="pw-btn" style="width: 200px">
        <ActionButton
          :label="install?.installed ? '重新安装 / 升级' : '安装 CEP 插件'"
          variant="primary"
          size="lg"
          :disabled="busy || install?.sourceAvailable === false"
          @click="doInstall"
        />
      </span>
    </div>
  </div>
</template>

<style scoped>
/* 内容面板（稿 544×538 窗口：padding 24 / 列 gap 24）；窗口壳的底色/描边/圆角/阴影 = App.vue .window
   口径（云母 = DWM 独裁前端零绘制；实色 = 前端自绘，§19）——本组件不画窗口层样式；
   flex:1 必填（App .window 是横向 flex，PreviewWindow 同口径） */
.plugin-window {
  flex: 1;
  min-width: 0;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  padding: var(--n-space-24);
}
.pw-title {
  flex: none;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
/* 组区滚动包裹：relative = ScrollThumb 锚点（§2 悬浮滚轮结构） */
.pw-scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}
.pw-body {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
}
.pw-group {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
.pw-sub {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
/* 行组（稿 提示文字 gap 6）：13/18 三级；bullet 圆点与方括号强调按稿截图（bullet 悬挂缩进 12 取间距档） */
.pw-lines {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-6);
}
.pw-lines li {
  position: relative;
  padding-left: var(--n-space-12);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.pw-lines li::before {
  content: "•";
  position: absolute;
  left: 0;
  top: 0;
}
/* 方括号强调（稿截图 [安装]/[窗口/扩展/AniKit]/[发送至 Animate] 加粗提亮——提亮档待用户走查） */
.pw-lines b {
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
/* 路径行：Windows 路径允许选中复制不做（body 全局 user-select:none 沿现状）；长路径折行 */
.pw-path {
  overflow-wrap: anywhere;
}
/* 按钮行（稿：右对齐 gap 8 / 高 38） */
.pw-actions {
  flex: none;
  height: 38px;
  display: flex;
  justify-content: flex-end;
  gap: var(--n-space-8);
}
.pw-btn {
  flex: none;
  display: block;
  height: 38px;
}
</style>
