<script setup lang="ts">
// 中间内容展示区（C1）：1.2 起 = dockview panel 组件，每个标签页一个实例，按 params.viewKey
// 渲染各自视口（标题栏 + 卡片列表（统一滚动：文件夹区块 + 素材网格）+ 搜索框）。
// 预览模式（4.3/5.2）：preview.assetKey 非空且本 panel 激活时整栏切换——预览是全局单例状态，
// 只在激活标签页内发生，非激活 panel 保持原视口；全屏时 preview-root fixed 铺满窗口、隐藏标题栏。
// 通知条（ToastStack）已上移到 DockHost 单例（多 panel 下每 panel 一份会重复弹出）。
import { computed, onBeforeUnmount, ref } from "vue";
import type { DockviewPanelApi, IDockviewPanelProps } from "dockview-vue";
import ContentTitleBar from "./ContentTitleBar.vue";
import AssetGrid from "./AssetGrid.vue";
import SequencePackView from "./SequencePackView.vue";
import ArchiveSidePanel from "./ArchiveSidePanel.vue";
import SearchBar from "./SearchBar.vue";
import ImportProgressBar from "./ImportProgressBar.vue";
import PreviewTitleBar from "./PreviewTitleBar.vue";
import PreviewPanel from "./PreviewPanel.vue";
import PreviewPlayBar from "./PreviewPlayBar.vue";
import PreviewFrameStrip from "./PreviewFrameStrip.vue";
import PreviewNotes from "./PreviewNotes.vue";
import { findFolderNode } from "../lib/detailSections";
import { useContentStore, type FolderNode } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { usePreviewStore } from "../stores/preview";
import { useSearchStore } from "../stores/search";

// props 运行时声明（rolldown 环境 SFC 编译器无法解析 node_modules 类型，defineProps 泛型须字面量）。
// dockview-vue 传参约定（v7 源码 VueRenderer.init）：framework props 整组包在一个 params prop 里——
// 组件实际收到 params = { params: 用户参数, api, containerApi, tabLocation }，updateParameters 后整体换新（响应式）。
const raw = defineProps<{ params: Record<string, unknown> }>();
const frame = computed(() => raw.params as unknown as IDockviewPanelProps);
const panelApi = frame.value.api as DockviewPanelApi; // api 引用 init 时确定、不变
const content = useContentStore();
const picker = usePickerStore();
const preview = usePreviewStore();
const search = useSearchStore();

// 本标签页视口（frame.params = addPanel/updateParameters 的用户参数）
const viewKey = computed(() => (frame.value.params?.viewKey as string) ?? "quick:all");

// 序列帧包内视图（5.2）：viewKey = seq:{assetId}——素材区整区换包内帧卡片列表
//（数据不走 placementsFor：帧不是 Placement；搜索/排序/分组对包内不适用，组件内自管）
const seqPackAssetId = computed(() => (viewKey.value.startsWith("seq:") ? viewKey.value.slice(4) : null));

// 本 panel 激活态：预览整栏切换只对激活标签页生效
const active = ref(panelApi.isActive);
const disposeActive = panelApi.onDidActiveChange(() => {
  active.value = panelApi.isActive;
});
onBeforeUnmount(() => disposeActive.dispose());

// 当前视口资源（"全部"= 全部 Placement；文件夹 = 该文件夹直接 Placement，
// 递归显示开关 ui.showSubfolders 打开时含全部后代文件夹，2026-08-10 接入）。
// 3.6 搜索态（用户定：结果替换素材区，清空恢复；排序由 placementsFor 带入保持生效）：
// 命中键过滤叠加在范围基底上——全局范围（chip / ~ 前缀）基底 = 全库（quick:all 不过滤即全量，
// 回收站本就不在 placements 内），当前范围基底 = 本视口（placementsFor 已含递归 / quick 口径）。
//（复验修复：原恒取本视口基底——空文件夹全局搜索时命中键在基底外被滤光，探针 S14 钉）
const searching = computed(() => search.isActive(panelApi.id));
const items = computed(() => {
  if (!searching.value) return content.placementsFor(viewKey.value);
  const hits = search.hitsFor(panelApi.id);
  const base = search.effectiveGlobal(panelApi.id)
    ? content.placementsFor("quick:all")
    : content.placementsFor(viewKey.value);
  return base.filter((p) => hits.has(`${p.assetId}|${p.folderId}`));
});
// 搜索态命中档案节点列表（AssetGrid 档案区数据源；无效 id 兜底剔除）
const searchArchiveNodes = computed<FolderNode[]>(() => {
  if (!searching.value) return [];
  const out: FolderNode[] = [];
  for (const id of search.hitArchivesFor(panelApi.id)) {
    const n = findFolderNode(content.folders, id);
    if (n) out.push(n);
  }
  return out;
});

// 档案二级侧边栏（2026-08-15 迁入视口）：本标签页视口 = 档案文件夹时，渲染在视口内容区最左
// （标签栏不被推挤；分屏 / 多标签各管各）
const archiveNode = computed<FolderNode | null>(() => {
  const k = viewKey.value;
  if (!k.startsWith("folder:")) return null;
  const n = findFolderNode(content.folders, k.slice(7));
  return n?.kind === "archive" ? n : null;
});
</script>

<template>
  <main class="content-area">
    <template v-if="preview.assetKey && active">
      <div class="preview-root" :class="{ fullscreen: preview.fullscreen }">
        <PreviewTitleBar v-if="!preview.fullscreen" />
        <section class="preview-section">
          <PreviewPanel />
        </section>
        <footer v-if="preview.hasPlayBar" class="preview-footer">
          <PreviewPlayBar />
          <PreviewFrameStrip />
          <PreviewNotes />
        </footer>
      </div>
    </template>
    <template v-else>
      <!-- 视口主体行：档案视口时面板插最左（视口内部组成，标签栏不被推挤）；搜索栏通栏在其下 -->
      <div class="viewport-body">
        <!-- 选择模式（6.2，2026-08-24 白名单收窄）：档案二级侧栏含数据编辑入口，本模式不渲染 -->
        <ArchiveSidePanel v-if="archiveNode && !picker.active" :node="archiveNode" :panel-active="active" />
        <div class="viewport-main">
          <ContentTitleBar :view-key="viewKey" :panel-id="panelApi.id" />
          <ImportProgressBar />
          <section class="asset-section">
            <SequencePackView v-if="seqPackAssetId" :asset-id="seqPackAssetId" :panel-active="active" />
            <AssetGrid
              v-else
              :items="items"
              :view-key="viewKey"
              :panel-active="active"
              :searching="searching"
              :search-archives="searchArchiveNodes"
            />
          </section>
        </div>
      </div>
      <footer class="search-wrap">
        <SearchBar :panel-id="panelApi.id" :view-key="viewKey" />
      </footer>
    </template>
  </main>
</template>

<style scoped>
.content-area {
  display: flex;
  flex-direction: column;
  height: 100%; /* 1.2：dockview panel 内容容器由 dock 管理尺寸，铺满即可 */
  min-width: 0;
  /* 无底色（7.1 十九轮二轮用户定）：透 dockview 内容容器卡片底 通用/一级，
     与选中标签页（同 g1）融合可辨；旧 c1 实底盖住卡片底色已废 */
  /* 三栏面板无圆角无边框、外层无 padding（figma-css 02_布局/42_中间面板，2026-08-10）：
     工具栏 / 搜索框自带 padding 12 16、列表容器 padding 16 16 0，各自承担 */
  box-sizing: border-box;
}
/* 视口主体行（2026-08-15 档案面板迁入）：面板 + 内容列横向排，行占满工具栏与搜索栏之间 */
.viewport-body {
  display: flex;
  flex: 1;
  min-height: 0;
}
.viewport-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}
/* 卡片列表（7.1 内容容器轮贴稿——列表容器 padding 12 12 0）：上 12 / 左 12 / 下 0；
   右 0——滚动条紧贴右边框（卡片右缘距边框 12 由布局 PAD.right 12 承担，layouts.ts） */
.asset-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: var(--n-space-12) 0 0 var(--n-space-12);
}
/* 搜索框（1.2 新稿）：通栏行（C3 + 上边框横贯中栏）——外层无 padding，自然通栏 */
.search-wrap {
  flex: none;
}
/* 预览模式根：占满中栏；全屏时 fixed 铺满窗口（2026-07-25 定案：显示器全屏） */
.preview-root {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.preview-root.fullscreen {
  position: fixed;
  inset: 0;
  z-index: 1500;
  background: var(--c1);
}
/* 全屏时播放条/帧条脱离文档流浮在画面底部（2026-07-26 用户：画面按整个窗口铺满，挡住就挡住） */
.preview-root.fullscreen .preview-footer {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1;
}
.preview-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 播放条 + 帧条容器：通栏贴底无内边距（13_播放条稿 2026-08-20 用户定：播放条与搜索栏同位同尺寸，
   直角 border-top 通栏——原 8 16 16 浮条内边距作废）；relative 供笔记浮层（bottom:100%）锚定 */
.preview-footer {
  position: relative;
  flex: none;
}
</style>
