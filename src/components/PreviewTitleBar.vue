<script setup lang="ts">
// 预览态标题栏（4.3/5.2）：返回 + 索引 x/y / 缩放滑块+百分比 / 工具按钮组。
// 布局模式照 ContentTitleBar（space-between 等距；图标统一 亮色/三级，2026-08-30 用户实机定——
// 旧 80% 口径作废；返回 = chevron-large 原样朝左；索引 mono 13/18、百分比 mono 12/16）。
// image/sequence/anim/video：缩放组 + 适合窗口 + 原图大小（源文件 PSD·AI·SVG 去旋转翻转、加默认应用打开；
// anim/video 无默认应用打开——**8.1 例外：伴生代理资产（fla 预览按伴生 WebP 显示为 anim）有，
// 本质工程文件，打开 fla 本身**）；audio：仅返回、索引、插件、新窗口打开、上/下一个；
// font/text（5.6）：同 other 组（含默认应用打开；字体安装/卸载在字体预览「其它」页内，不占标题栏）；
// other：返回、索引、插件、默认应用打开、新窗口打开、上/下一个。
// window 模式（新窗口打开）：返回 + 索引替换为文件名标题，加置顶与完整窗口控制按钮（PRD 4.3）。
import { computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import IconButton from "./buttons/IconButton.vue";
import SideSlider from "./SideSlider.vue";
import WindowControls from "./WindowControls.vue";
import { usePreviewStore } from "../stores/preview";
import { useUiStore } from "../stores/ui";
import { openPluginPanel } from "../lib/pluginPanel";

withDefaults(defineProps<{ window?: boolean }>(), { window: false });

const preview = usePreviewStore();
const ui = useUiStore();
const appWindow = getCurrentWindow();

// 滑块 = 缩放百分比（2%~800% 线性），瞄点在视图中心（2026-07-25 定案）
const zoomPct = computed({
  get: () => Math.round(preview.zoom * 100),
  set: (v: number) => preview.zoomAt(v / 100, preview.viewW / 2, preview.viewH / 2),
});

// 新窗口打开（4.3）：同资产复用聚焦
async function openInNewWindow(): Promise<void> {
  if (!preview.assetKey) return;
  await invoke("open_preview_window", { assetKey: preview.assetKey });
}

// 工作空间磁盘实况（PRD 02 步骤 1）：预览目标是 ws 合成项（assetId 假 id）——
// 「默认应用打开」走 assetId 命令、「新窗口打开」子窗口按 assetId 链拉数据，本阶段均不下发/不开，
// 两钮隐藏（其余按钮纯前端 / 按路径，照常）
const isWsCurrent = computed(() => !!preview.current?.assetId.startsWith("ws:"));

async function togglePin(): Promise<void> {
  ui.alwaysOnTop = !ui.alwaysOnTop;
  await appWindow.setAlwaysOnTop(ui.alwaysOnTop);
}
</script>

<template>
  <div class="preview-titlebar" data-tauri-drag-region>
    <div class="nav-group">
      <template v-if="!window">
        <!-- 返回 = chevron-large 原样朝左（2026-08-30 实机修正：图标文件实为朝左，
             速查表 §6「朝右」口径与文件相反已修正——旧 rot-180 旋转后朝右 = 反了） -->
        <IconButton variant="plain" name="chevron-large" title="返回" @click="preview.close()" />
        <!-- 索引计数（预览工具栏稿）：数字-大 mono 13/18，当前/分隔 二级、总数 三级 -->
        <span class="index-text">
          <span class="idx-cur">{{ preview.index + 1 }}</span>
          <span class="idx-sep">/</span>
          <span class="idx-total">{{ preview.total }}</span>
        </span>
      </template>
      <span v-else class="index-name">{{ preview.current?.name ?? "预览" }}</span>
    </div>
    <div v-if="preview.viewable" class="zoom-group">
      <SideSlider v-model="zoomPct" :min="2" :max="800" />
      <span class="zoom-text">{{ Math.round(preview.zoom * 100) }}%</span>
    </div>
    <div class="btn-group">
      <!-- 插件（2026-09-06 接业务 = 插件列表面板浮层，预览子窗口同可开——宿主 App.vue 层） -->
      <IconButton variant="plain" name="plugin" title="插件" @click="openPluginPanel" />
      <template v-if="preview.viewable">
        <IconButton v-if="!preview.isSource" variant="plain" name="rotate-cw" title="旋转" @click="preview.rotateCw()" />
        <IconButton v-if="!preview.isSource" variant="plain" name="flip-h" title="水平翻转" @click="preview.toggleFlip()" />
        <IconButton variant="plain" name="fit-screen" title="适合窗口" @click="preview.fit()" />
        <IconButton variant="plain" name="original-size" title="原图大小" @click="preview.actual()" />
      </template>
      <!-- 默认应用打开：源格式（PSD 等）/ other / font / text + **伴生代理资产（8.1：fla 预览
           按伴生 WebP 显示为 anim，但本质是工程文件——入口打开 fla 本身）**；
           工作空间合成项隐藏（assetId 假 id 禁发命令，见 script isWsCurrent） -->
      <IconButton
        v-if="(preview.isSource || preview.kind === 'other' || preview.kind === 'font' || preview.kind === 'text' || !!preview.current?.playAssetId) && !isWsCurrent"
        variant="plain"
        name="open-default"
        title="默认应用打开"
        @click="preview.openDefaultApp()"
      />
      <IconButton
        v-if="!window && !isWsCurrent"
        variant="plain"
        name="open-new-window"
        title="新窗口打开"
        @click="openInNewWindow"
      />
      <!-- 上一个 / 下一个 = angle-bracket 尖括号（原样朝左 / rot-180 朝右），与内容工具栏
           后退/前进同款（2026-08-30 内容容器轮统一，旧 nav-history 引用清零） -->
      <IconButton
        variant="plain"
        name="angle-bracket"
        title="上一个"
        :disabled="preview.index <= 0"
        @click="preview.step(-1)"
      />
      <IconButton
        variant="plain"
        name="angle-bracket"
        title="下一个"
        class="rot-180"
        :disabled="preview.index >= preview.total - 1"
        @click="preview.step(1)"
      />
      <template v-if="window">
        <IconButton
          variant="plain"
          :name="ui.alwaysOnTop ? 'pin-off' : 'pin-on'"
          title="窗口置顶"
          @click="togglePin"
        />
        <WindowControls />
      </template>
    </div>
  </div>
</template>

<style scoped>
.preview-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  /* 内边距 12 16（与 ContentTitleBar 一致；2026-08-10 随 42 稿外层 padding 归零补右左 16） */
  padding: var(--n-space-12) var(--n-space-16);
  flex: none;
}
/* 中栏标题栏图标按钮统一 亮色/三级（2026-08-30 用户实机定，与内容工具栏 IconButton plain 同档；
   旧 --font-icon-2 80% 口径作废） */
.preview-titlebar :deep(.icon-btn) {
  color: var(--n-l3);
}
.nav-group {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
}
.btn-group {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  flex: none;
}
.zoom-group {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  flex: none;
}
/* 上/下一个 = angle-bracket rot-180 朝右（翻转类沿用） */
.rot-180 :deep(svg) {
  transform: rotate(180deg);
}
/* 索引计数（预览工具栏稿）：数字-大 mono 13/18，gap 4；当前/分隔 二级、总数 三级 */
.index-text {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  white-space: nowrap;
}
.idx-cur,
.idx-sep {
  color: var(--n-l2);
}
.idx-total {
  color: var(--n-l3);
}
/* window 模式文件名（正文档 一级） */
.index-name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
  white-space: nowrap;
}
/* 缩放百分比（2026-08-30 用户实机定）：数字档 mono 12/16 三级（稿未画此件，沿用元素改字档） */
.zoom-text {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  white-space: nowrap;
  min-width: 38px;
  text-align: right;
}
</style>
