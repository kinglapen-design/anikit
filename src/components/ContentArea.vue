<script setup lang="ts">
// 中间内容展示区（C1）：1.2 起 = dockview panel 组件，每个标签页一个实例，按 params.viewKey
// 渲染各自视口（标题栏 + 卡片列表（统一滚动：文件夹区块 + 素材网格）+ 多功能悬浮栏）。
// 悬浮栏 = FloatBarHost 根部单例（2026-09-09 二轮宿主合并，稿 prose「同一容器切换」：预览进出 =
// 搜索栏 ↔ 播放条 morph；底部居中距底 12、两侧 12 呼吸；旧通栏 SearchBar 与预览 footer 三件套均废）；
// 列表底部遮挡补偿 66 = layouts.ts PAD.bottom（12 栏距底 + 42 栏高 + 12 呼吸距；JS 滚动 translateY，
// CSS padding 不影响可滚距离）。
// 预览模式（4.3/5.2）：preview.assetKey 非空且本 panel 激活时整栏切换——预览是全局单例状态，
// 只在激活标签页内发生，非激活 panel 保持原视口；全屏时 preview-root fixed 铺满窗口、隐藏标题栏。
// 通知条（ToastStack）已上移到 DockHost 单例（多 panel 下每 panel 一份会重复弹出）。
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { DockviewPanelApi, IDockviewPanelProps } from "dockview-vue";
import ContentTitleBar from "./ContentTitleBar.vue";
import AssetGrid from "./AssetGrid.vue";
import SequencePackView from "./SequencePackView.vue";
import FloatBarHost from "./FloatBarHost.vue";
import ImportProgressBar from "./ImportProgressBar.vue";
import PreviewTitleBar from "./PreviewTitleBar.vue";
import PreviewPanel from "./PreviewPanel.vue";
import { findFolderNode } from "../lib/detailSections";
import { useContentStore, type FolderNode } from "../stores/content";
import { usePreviewStore } from "../stores/preview";
import { useSearchStore } from "../stores/search";
import { useUiStore } from "../stores/ui";

// props 运行时声明（rolldown 环境 SFC 编译器无法解析 node_modules 类型，defineProps 泛型须字面量）。
// dockview-vue 传参约定（v7 源码 VueRenderer.init）：framework props 整组包在一个 params prop 里——
// 组件实际收到 params = { params: 用户参数, api, containerApi, tabLocation }，updateParameters 后整体换新（响应式）。
const raw = defineProps<{ params: Record<string, unknown> }>();
const frame = computed(() => raw.params as unknown as IDockviewPanelProps);
const panelApi = frame.value.api as DockviewPanelApi; // api 引用 init 时确定、不变
const content = useContentStore();
const preview = usePreviewStore();
const search = useSearchStore();
const ui = useUiStore();

// 「始终显示搜索框」关（ui.showSearchBar=false，2026-09-06 视图菜单轮，用户定）时的唤出态：
// Ctrl+F（search.requestFocus 本 panel）唤出；Esc 清空失焦后 FloatBarHost request-hide 收回。
// 开关恒开时本态无关（v-if 条件恒真）；显隐动画 = 下浮入 / 下沉出（sf-reveal）
const searchRevealed = ref(false);
watch(
  () => search.focusSeq,
  () => {
    if (search.focusPanelId === panelApi.id) searchRevealed.value = true;
  },
);

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
// 预览态 = preview.assetKey 非空且本 panel 激活（预览是全局单例，非激活 panel 保持原视口）
const previewActive = computed(() => !!preview.assetKey && active.value);

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

</script>

<template>
  <main class="content-area">
    <template v-if="previewActive">
      <div class="preview-root" :class="{ fullscreen: preview.fullscreen }">
        <PreviewTitleBar v-if="!preview.fullscreen" />
        <section class="preview-section">
          <PreviewPanel />
        </section>
      </div>
    </template>
    <template v-else>
      <!-- 视口主体列：工具栏 + 卡片列表；多功能悬浮栏浮于下方（不占布局） -->
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
    </template>
    <!-- 多功能悬浮栏宿主：根部单例——预览进出 = 搜索栏 ↔ 播放条同一容器形态切换 morph
         （稿 prose，2026-09-09 二轮落地）；定位底部居中距底 12、两侧 12 呼吸。
         「始终显示搜索框」关 = 非预览态平时隐藏，Ctrl+F 唤出 / Esc 收回（sf-reveal 出入场） -->
    <Transition name="sf-reveal">
      <FloatBarHost
        v-if="previewActive || ui.showSearchBar || searchRevealed"
        :panel-id="panelApi.id"
        :view-key="viewKey"
        :preview-active="previewActive"
        @request-hide="searchRevealed = false"
      />
    </Transition>
  </main>
</template>

<style scoped>
.content-area {
  position: relative; /* 悬浮栏（fbh-anchor）定位基准 */
  display: flex;
  flex-direction: column;
  height: 100%; /* 1.2：dockview panel 内容容器由 dock 管理尺寸，铺满即可 */
  min-width: 0;
  /* 无底色（7.1 十九轮二轮用户定）：透 dockview 内容容器卡片底 通用/一级，
     与选中标签页（同 g1）融合可辨；旧 c1 实底盖住卡片底色已废 */
  /* 三栏面板无圆角无边框、外层无 padding（figma-css 02_布局/42_中间面板，2026-08-10）：
     工具栏自带 padding、列表容器 padding 12 12 0，各自承担 */
  box-sizing: border-box;
}
/* 视口主体列：占满工具栏与搜索栏之间（档案二级侧边栏 2026-08-30 删除后无横向排布） */
.viewport-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
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
/* 预览模式根：占满中栏；全屏时 fixed 铺满窗口（2026-07-25 定案：显示器全屏）。
   悬浮栏宿主（FloatBarHost）挂 content-area 根部两分支外（预览进出不卸载 = morph 成立前提）；
   全屏时 anchor 上溯 fixed 本根为包含块，距底/两侧 12 两态同位（2026-09-09 二轮） */
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
/* 悬浮栏显隐动画（「始终显示搜索框」开关，2026-09-06 用户定）：
   显示 = 下至上浮入 / 隐藏 = 上至下沉出——位移 8 + 透明度，250ms 缓入缓出（悬浮栏族动画档 §18） */
.sf-reveal-enter-active,
.sf-reveal-leave-active {
  transition:
    transform var(--n-dur-250) var(--n-ease-in-out),
    opacity var(--n-dur-250) var(--n-ease-in-out);
}
.sf-reveal-enter-from,
.sf-reveal-leave-to {
  transform: translateY(8px);
  opacity: 0;
}
.preview-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
</style>
