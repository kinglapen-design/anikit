<script setup lang="ts">
// 编辑器贴图直链 详情子窗口（2026-10-23，PRD 05-01「编辑器贴图直链」条；版式照 McpPluginWindow——
// §37 四组版式 + 拖拽带 + 底部按钮行，零新样式）：
// Typora（anikit-paste.exe 自定义上传命令）/ Obsidian（粘贴插件）贴图 → 拷入收件箱自动入库，
// 文档当场写 by-name 链接（http 渲染 + anikit:// 点击拉起；Typora 受其上传命令边界只回纯 http 图）。
// 四组 = 说明 / Typora / Obsidian / 路径；底部 = 左件媒体服务状态（图片渲染依赖）+ 右件关闭。
// 状态零 ui store（子窗口铁律）：info 直走 get_paste_link_info，安装走 install_obsidian_plugin。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ActionButton from "../buttons/ActionButton.vue";
import IconButton from "../buttons/IconButton.vue";
import ScrollThumb from "../ScrollThumb.vue";
import {
  getMcpStatus,
  getPasteLinkInfo,
  installObsidianPluginFlow,
  type McpStatus,
  type PasteLinkInfo,
} from "../../lib/plugins";
import { useToastStore } from "../../stores/toast";

const info = ref<PasteLinkInfo | null>(null);
const media = ref<McpStatus | null>(null);
const installing = ref(false);
const bodyRef = ref<HTMLElement>();
const toast = useToastStore();

onMounted(async () => {
  info.value = await getPasteLinkInfo();
  media.value = await getMcpStatus();
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

const command = computed(() => info.value?.command ?? "读取中…");

async function copyCommand(): Promise<void> {
  try {
    await navigator.clipboard.writeText(command.value);
    toast.notify({ message: "Typora 自定义命令已复制", icon: "success" });
  } catch {
    toast.notify({ message: "复制失败", icon: "fail" });
  }
}

async function onInstall(): Promise<void> {
  if (installing.value) return;
  installing.value = true;
  try {
    await installObsidianPluginFlow(); // 目录选择器 + 安装 + toast 全在封装内
  } finally {
    installing.value = false;
  }
}

// 底部左件 = 媒体服务状态（图片渲染依赖；圆点 + 字重双指示同 MCP 窗 2026-09-20 口径）
const mediaStateText = computed(() => {
  const st = media.value;
  if (!st) return "读取中…";
  return st.running ? "媒体服务运行中" : "媒体服务已停止";
});
</script>

<template>
  <div class="plugin-window">
    <!-- 顶部拖拽带（MCP 窗同口径：通栏 62px，容器与 h1 双挂 data-tauri-drag-region） -->
    <div class="pw-dragband" data-tauri-drag-region>
      <h1 class="pw-title" data-tauri-drag-region>编辑器贴图直链</h1>
    </div>
    <div class="pw-scroll-wrap">
      <div ref="bodyRef" class="pw-body kit-scroll">
        <section class="pw-group">
          <h2 class="pw-sub">说明</h2>
          <ul class="pw-lines">
            <li>
              在 Typora / Obsidian 里照常粘贴（或拖入）图片：图片自动拷入 AniKit 收件箱并导入素材库，
              文档当场拿到稳定链接——附件集中进库，不再散落各处。
            </li>
            <li>链接按文件名解析（by-name）：粘贴即写链，无需等待导入完成；AniKit 没开也能贴，下次启动自动入库、链接即活。</li>
            <li>收件箱「自动导入」开关关闭时（设置 → 通用），贴图只落盘待导、开启后入库；图片渲染依赖本机媒体服务（AniKit 运行中可见）。</li>
            <li>
              Obsidian 侧链接可点击拉起 AniKit 定位到该图；对外分发文档用 AniKit 右键「打包导出 MD」转成离线图片。
            </li>
          </ul>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">Typora</h2>
          <ul class="pw-lines">
            <li>打开 Typora <b>偏好设置 → 图像</b>，「插入图片时…」选 <b>[上传图片]</b>。</li>
            <li>「上传服务」选 <b>[Custom Command]</b>，自定义命令粘贴下方文本（贴图工具随 AniKit 分发）：</li>
            <li class="cfg-row">
              <span class="cfg-text">{{ command }}</span>
              <IconButton name="copy" variant="bare" size="sm" title="复制" @click="copyCommand" />
            </li>
            <li>Typora 上传命令只能拿回图片地址，链接是纯图片形态（不能点击拉起 AniKit）；要可点击拉起用下方 Obsidian 插件。</li>
          </ul>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">Obsidian</h2>
          <ul class="pw-lines">
            <li>点下方按钮选择 Obsidian 库（vault）根目录，插件写入其 .obsidian/plugins/anikit-paste。</li>
            <li>重启 Obsidian → 设置 → 第三方插件，启用 <b>[AniKit 贴图直链]</b>（插件内可改收件箱目录与链接形态）。</li>
          </ul>
          <div>
            <ActionButton
              label="选择库安装插件…"
              icon="import-file"
              variant="solid"
              size="md"
              :disabled="installing"
              @click="onInstall"
            />
          </div>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">路径</h2>
          <ul class="pw-lines">
            <li class="pw-path">贴图工具: {{ info?.toolPath ?? "读取中…" }}</li>
            <li class="pw-path">收件箱目录: {{ info?.inboxDir ?? "未打开储存空间" }}</li>
            <li class="pw-path">端口文件: %APPDATA%\AniKit\mcp-server.port</li>
          </ul>
        </section>
      </div>
      <ScrollThumb :body="bodyRef" />
    </div>
    <!-- 按钮行（同 MCP 窗口径：左件状态 + 右件关闭） -->
    <div class="pw-actions">
      <span class="svc-status">
        <span class="svc-dot" :class="{ on: media?.running }" />
        <span class="svc-state" :class="{ on: media?.running }">{{ mediaStateText }}</span>
      </span>
      <span class="pw-btn">
        <ActionButton label="关闭" variant="cancel" size="lg" @click="closeWin" />
      </span>
    </div>
  </div>
</template>

<style scoped>
/* 版式与 McpPluginWindow 同口径（§37；窗口壳底色/描边/圆角 = App.vue .window 承担） */
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
  margin: 0;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.pw-dragband {
  flex: none;
  margin: calc(-1 * var(--n-space-24)) calc(-1 * var(--n-space-24)) calc(-1 * var(--n-space-12));
  padding: var(--n-space-24) var(--n-space-24) var(--n-space-12);
}
.pw-scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  margin-right: calc(-1 * var(--n-space-24));
}
.pw-body {
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  padding-right: var(--n-space-24);
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
.pw-lines b {
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pw-lines li.cfg-row {
  display: flex;
  align-items: flex-start;
  gap: var(--n-space-6);
  padding: var(--n-space-6) var(--n-space-8);
  background: var(--n-l-a4);
  box-shadow: inset 0 0 0 1px var(--n-l-a4);
  border-radius: var(--n-radius-6);
}
.pw-lines li.cfg-row::before {
  content: none;
}
.cfg-text {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.cfg-row .ic-btn {
  flex: none;
  margin-top: 1px;
}
.svc-status {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.svc-dot {
  width: 8px;
  height: 8px;
  border-radius: 99px;
  background: var(--n-l4);
  flex: none;
}
.svc-dot.on {
  background: var(--n-success);
}
.svc-state {
  flex: none;
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
}
.svc-state.on {
  color: var(--n-l2);
}
.pw-path {
  overflow-wrap: anywhere;
}
.pw-actions {
  flex: none;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
}
.pw-btn {
  flex: none;
  display: block;
  height: 38px;
}
.pw-actions .pw-btn :deep(.act-btn) {
  width: auto;
  padding: var(--n-space-8) 48px;
}
</style>
