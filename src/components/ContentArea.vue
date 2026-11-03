<script setup lang="ts">
// 中间内容展示区（C1）：1.2 起 = dockview panel 组件，每个标签页一个实例，按 params.viewKey
// 渲染各自视口（标题栏 + 卡片列表（统一滚动：文件夹区块 + 素材网格）+ 多功能悬浮栏）。
// 悬浮栏 = FloatBarHost 根部单例（2026-09-09 二轮宿主合并，稿 prose「同一容器切换」：预览进出 =
// 搜索栏 ↔ 播放条 morph；底部居中距底 12、两侧 12 呼吸；旧通栏 SearchBar 与预览 footer 三件套均废）；
// 列表底部遮挡补偿 66 = layouts.ts PAD.bottom（12 栏距底 + 42 栏高 + 12 呼吸距；JS 滚动 translateY，
// CSS padding 不影响可滚距离）。
// 预览模式（4.3/5.2）：每 panel 一个预览会话（注册表懒建、provide 注入），本 panel 会话 assetKey
// 非空 = 本栏切预览；分屏各 panel 会话独立互不抢；属主 panel 关闭由 tabs.syncMirror 回收；
// 全屏时 preview-root fixed 铺满窗口、隐藏标题栏。
// 通知条（ToastStack）已上移到 DockHost 单例（多 panel 下每 panel 一份会重复弹出）。
import { computed, onBeforeUnmount, provide, ref, watch } from "vue";
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
import { previewSession } from "../stores/preview";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";
import { useSearchStore } from "../stores/search";
import { useUiStore } from "../stores/ui";
import { useWsStore } from "../stores/ws";

// props 运行时声明（rolldown 环境 SFC 编译器无法解析 node_modules 类型，defineProps 泛型须字面量）。
// dockview-vue 传参约定（v7 源码 VueRenderer.init）：framework props 整组包在一个 params prop 里——
// 组件实际收到 params = { params: 用户参数, api, containerApi, tabLocation }，updateParameters 后整体换新（响应式）。
const raw = defineProps<{ params: Record<string, unknown> }>();
const frame = computed(() => raw.params as unknown as IDockviewPanelProps);
const panelApi = frame.value.api as DockviewPanelApi; // api 引用 init 时确定、不变
const content = useContentStore();
// 本 panel 预览会话（注册表懒建；provide 给预览族组件 inject，属主恒 = 本 panel）
const session = previewSession(panelApi.id);
provide(PREVIEW_SESSION_KEY, session);
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

// 工作空间视口（PRD 02 步骤 1）：挂载 + viewKey 变化即确保对应目录已加载——已加载即跳过
//（2026-10-01 性能优化③：加载后由 fs-changed 精准重扫维持新鲜，挂载不再全量重读；
// 懒加载竞态守卫在 ws store；loadDir 完成自动补请求缺失缩略图；fs-changed 经
// initWsListeners 统一驱动 rescanDirs / 兜底 rescanLoaded，此处只管线首发）；
// 兼 watch ws.root（2026-10-02 启动空白修复）：启动 / 切空间时挂载加载可能跑在空间就绪前
//（loadDir 早退不入缓存），root 重新就绪（reseedAfterSpaceChange）必须补一次重引
const ws = useWsStore();
watch(
  [viewKey, () => ws.root],
  () => {
    if (viewKey.value.startsWith("ws:")) void ws.ensureDir(viewKey.value.slice(3));
  },
  { immediate: true },
);

// 本 panel 激活态（dockview 镜像，透传 AssetGrid/SequencePackView 的 panel-active）
const active = ref(panelApi.isActive);
const disposeActive = panelApi.onDidActiveChange(() => {
  active.value = panelApi.isActive;
});
onBeforeUnmount(() => disposeActive.dispose());
// 预览态 = 本 panel 会话 assetKey 非空（per-panel 会话：属主恒 = 本 panel，分屏各 panel 互不抢；
// 属主 panel 关闭由 tabs.syncMirror 回收会话）
const previewActive = computed(() => !!session.assetKey);

// 当前视口资源（"全部"= 全部 Placement；文件夹 = 该文件夹直接 Placement，
// 递归显示开关 ui.showSubfolders 打开时含全部后代文件夹，2026-08-10 接入）。
// 3.6 搜索态（用户定：结果替换素材区，清空恢复；排序由 placementsFor 带入保持生效）：
// 命中键过滤叠加在范围基底上——全局范围（chip / ~ 前缀）基底 = 全库（quick:all 不过滤即全量，
// 回收站本就不在 placements 内），当前范围基底 = 本视口（placementsFor 已含递归 / quick 口径）。
//（复验修复：原恒取本视口基底——空文件夹全局搜索时命中键在基底外被滤光，探针 S14 钉）
// 2026-10-03 ws 搜索轮：ws 视口搜索态数据源 = search store 的 wsEntries 合成（后端 Everything /
// 爬盘命中跨目录，不在本视口目录缓存内，不能走命中键过滤基底链；键口径与合成链一致）
const searching = computed(() => search.isActive(panelApi.id));
const wsView = computed(() => viewKey.value.startsWith("ws:"));
// ws 降级引导空态文案（2026-10-03 ws 搜索轮）：Everything 不可用且未发起爬盘 → 「按 Enter」引导；
// 库查询中显示「搜索中...」，本轮结束后空结果才由 AssetGrid 回退为「无搜索结果」。
const searchHint = computed(() => {
  if (wsView.value && search.wsNeedsEnter(panelApi.id)) {
    return "Everything 未运行——按 Enter 遍历搜索当前目录（全局搜索需 Everything 支持）";
  }
  return !wsView.value && search.isPending(panelApi.id) ? "搜索中..." : "";
});
const items = computed(() => {
  if (!searching.value) return content.placementsFor(viewKey.value);
  if (wsView.value) return search.wsPlacementsFor(panelApi.id);
  return search.resultPlacementsFor(panelApi.id, viewKey.value);
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

// 空白处点击的状态机（2026-09-17 实机走查两条定案）：
// ① 本 pane **已聚焦** = 不干预——空白 click 走原语义 = **取消选择**（setActive 一概不调——
//    复验石锤：激活链即时重渲染会把 in-flight click 目标元素换掉，调了就吞清选 click）；
// ② **未聚焦** = 本击**只聚焦不清空**（setActive + stopImmediatePropagation 拦下后续手势——
//    capture 阶段先拦，框选不启动、选择域不动；下一击即按已聚焦走清空语义）。
// 交互件（卡片/树行/按钮/输入框/菜单/分组标题/滚动条等）上永不激活，card click 链不受影响。
function onRootPointerdown(e: PointerEvent): void {
  if (panelApi.isActive) return;
  const t = e.target as HTMLElement;
  if (
    t.closest(
      ".asset-card, .list-row, .folder-card, .archive-card, .tree-item, button, input, textarea, .app-menu, a, .group-header, .sticky-group-header, .free-empty-hint, .grid-track",
    )
  ) {
    return;
  }
  panelApi.setActive();
  e.stopImmediatePropagation(); // 本击只聚焦（拦下框选等后续手势，不清空选中）
}
</script>

<template>
  <!-- 空白处点击状态机（onRootPointerdown，capture 先拦）：已聚焦 = 空白取消选择；
       未聚焦 = 本击只聚焦不清空；交互件上永不激活（复验石锤：整区激活吞 click） -->
  <main class="content-area" @pointerdown.capture="onRootPointerdown">
    <template v-if="previewActive">
      <div class="preview-root" :class="{ fullscreen: session.fullscreen }">
        <PreviewTitleBar v-if="!session.fullscreen" />
        <section class="preview-section">
          <PreviewPanel />
        </section>
      </div>
    </template>
    <template v-else>
      <!-- 视口主体列：工具栏 + 卡片列表；多功能悬浮栏浮于下方（不占布局） -->
      <div class="viewport-main">
        <ContentTitleBar :view-key="viewKey" :panel-id="panelApi.id" />
        <ImportProgressBar :panel-id="panelApi.id" />
        <section class="asset-section">
          <SequencePackView v-if="seqPackAssetId" :asset-id="seqPackAssetId" :panel-active="active" />
          <AssetGrid
            v-else
            :items="items"
            :view-key="viewKey"
            :panel-active="active"
            :searching="searching"
            :search-archives="searchArchiveNodes"
            :search-hint="searchHint"
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
/* 卡片列表（7.1 内容容器轮贴稿——列表容器 padding 12 12 0）：上 12 / 右 0 / 下 0 / 左 0；
   右 0——滚动条紧贴右边框；左右 12 边距统一由布局 PAD 承担（layouts.ts——左 12 于 2026-10-15
   自本区移入布局：viewport 满宽后框选矩形可覆盖左边距带、首列卡 hover 投影不再被
   viewport overflow 左缘裁剪，与右侧同构） */
.asset-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: var(--n-space-12) 0 0 0;
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
  background: var(--n-g1);
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
