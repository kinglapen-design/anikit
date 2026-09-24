<script setup lang="ts">
// 设计系统预览页（阶段七）：Token.md 全部 token 的验收面（色板 / 字阶 / 效果 / 圆角间距 / 动画）。
// 仅开发环境经 #design-system 哈希进入（main.ts 动态 import，不进生产构建）；纯前端渲染，不调后端命令。
// 组件新增 / 扩展 / 改样式时同次更新本页（ui-refactor 规范第 7 条）。
import "./preview.css";
import PreviewInventory from "./sections/PreviewInventory.vue";
import PreviewColors from "./sections/PreviewColors.vue";
import PreviewType from "./sections/PreviewType.vue";
import PreviewMarkdown from "./sections/PreviewMarkdown.vue";
import PreviewEffects from "./sections/PreviewEffects.vue";
import PreviewShape from "./sections/PreviewShape.vue";
import PreviewMotion from "./sections/PreviewMotion.vue";
import PreviewItems from "./sections/PreviewItems.vue";
import PreviewButtons from "./sections/PreviewButtons.vue";
import PreviewInputs from "./sections/PreviewInputs.vue";
import PreviewCards from "./sections/PreviewCards.vue";
import PreviewBars from "./sections/PreviewBars.vue";
import PreviewMenus from "./sections/PreviewMenus.vue";
import PreviewModals from "./sections/PreviewModals.vue";
import PreviewStates from "./sections/PreviewStates.vue";
import PreviewPlugins from "./sections/PreviewPlugins.vue";
import PreviewDialogs from "./sections/PreviewDialogs.vue";
import PreviewOverlays from "./sections/PreviewOverlays.vue";
import PreviewWindows from "./sections/PreviewWindows.vue";
import { provide } from "vue";
import { previewSession } from "../stores/preview";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

// 预览会话占位 provide（per-panel 会话化配套：展区内真实实例 FloatBarHost / WaveProgress 注入必需；
// "__design__" 会话永不 open（assetKey 恒 null），仅供组件取数不崩）
provide(PREVIEW_SESSION_KEY, previewSession("__design__"));

const nav = [
  { id: "inventory", label: "组件台账" },
  { id: "colors", label: "颜色" },
  { id: "type", label: "字体" },
  { id: "markdown", label: "MarkDown 排版" },
  { id: "effects", label: "效果" },
  { id: "shape", label: "圆角 / 间距" },
  { id: "motion", label: "动画" },
  { id: "items", label: "组件 item" },
  { id: "buttons", label: "组件 按钮" },
  { id: "inputs", label: "组件 输入框" },
  { id: "cards", label: "组件 卡片" },
  { id: "bars", label: "组件 悬浮栏" },
  { id: "menus", label: "组件 上下文菜单" },
  { id: "modals", label: "组件 模态窗口" },
  { id: "states", label: "组件 空态" },
  { id: "plugins", label: "面板 插件" },
  { id: "dialogs", label: "业务弹窗" },
  { id: "overlays", label: "浮层与滚动" },
  { id: "windows", label: "独立窗口" },
];
</script>

<template>
  <div class="pv-root">
    <header class="pv-head">
      <h1 class="pv-title">AniKit 设计系统</h1>
      <p class="pv-sub">
        源头：docs/06_UI重构 / Design Token / Token.md（阶段七，2026-08-28 起）· 仅开发环境可见
      </p>
      <nav class="pv-nav">
        <a v-for="n in nav" :key="n.id" class="pv-nav-item" :href="`#design-system-${n.id}`">
          {{ n.label }}
        </a>
      </nav>
    </header>
    <main class="pv-main">
      <PreviewInventory />
      <PreviewColors />
      <PreviewType />
      <PreviewMarkdown />
      <PreviewEffects />
      <PreviewShape />
      <PreviewMotion />
      <PreviewItems />
      <PreviewButtons />
      <PreviewInputs />
      <PreviewCards />
      <PreviewBars />
      <PreviewMenus />
      <PreviewModals />
      <PreviewStates />
      <PreviewPlugins />
      <PreviewDialogs />
      <PreviewOverlays />
      <PreviewWindows />
    </main>
  </div>
</template>
