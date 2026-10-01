// 多标签页（阶段一 1.2 主窗口重构，figma-css 15_标签栏 / 39_右键菜单-标签页）：
// dockview 为唯一布局源，本 store 是命令层（规范第 1 条）+ 响应式镜像。
// 行为约定（2026-08-08 随计划定案）：
// - 标签页 = 一个文件视口（viewKey，同 content.sidebarSelected 取值）；
// - 侧边栏导航：当前标签页未锁定 → 替换其视口；已锁定 → 新开标签页（Eagle 式）；
// - 锁定标签页：无关闭按钮（显示锁定图标）、右键"关闭标签页"禁用；不禁止拖拽排序 / 分屏；
// - 数据共享：所有标签页共用全局 content / selection store，标签页只记自己的 viewKey。
// 布局持久化：onDidLayoutChange 防抖 500ms → ui.tabLayout（settings.ui 子树，ui store 既有写回通道）。
import { defineStore } from "pinia";
import { markRaw, nextTick, ref } from "vue";
import type { DockviewApi, IDockviewPanel, SerializedDockview } from "dockview-vue";
import { useContentStore } from "./content";
import { dropPreviewSession, eachPreviewSession, peekPreviewSession, previewSession } from "./preview";
import { useSelectionStore } from "./selection";
import { useUiStore } from "./ui";
import { findFolderNode } from "../lib/detailSections";

export interface TabInfo {
  id: string;
  viewKey: string;
  locked: boolean;
}

let seq = 0;

export const useTabsStore = defineStore("tabs", () => {
  const content = useContentStore();
  const ui = useUiStore();

  // dockview api 非响应式（内部对象大且自管理）；tabs / activeTabId 是面向组件的只读镜像
  let api: DockviewApi | null = null;
  const tabs = ref<TabInfo[]>([]);
  const activeTabId = ref("");

  // 导航历史（2026-08-20，工具栏面包屑后退/前进接入）：per-panel 双栈——openView 替换视口时
  // 旧 key 入 back 并清 fwd；后退/前进在两栈间移动当前 key。新开页 / 布局恢复 = 空历史；
  // 锁定页导航新开（Eagle 式）不经本栈。会话级不持久化。
  // 条目带选中快照（2026-08-24 用户定："返回的时候默认选中卡片"）——openView 压栈记当前选中、
  // goHistory 弹出恢复（详情面板不空，"回到触发处"语义贯通到历史导航）
  interface NavSel {
    asset?: string; // Placement key（assetId|folderId）
    folder?: string; // 文件夹选择域 folderId
  }
  interface NavEntry {
    key: string;
    sel: NavSel;
  }
  const histories = new Map<string, { back: NavEntry[]; fwd: NavEntry[] }>();

  // 当前选中快照（首项口径——详情面板单资产消费同 selection.first）
  function currentSel(): NavSel {
    const selection = useSelectionStore();
    if (selection.first) return { asset: selection.first };
    for (const id of selection.selectedFolders) return { folder: id };
    return {};
  }

  // 恢复选中：目标已不存在（期间被删）跳过；nextTick 让过 sidebarSelected watch 的清选择
  //（pre flush 先执行，FolderTree 入域同口径）
  function restoreSel(sel: NavSel): void {
    if (!sel.asset && !sel.folder) return;
    void nextTick(() => {
      const selection = useSelectionStore();
      if (sel.asset) {
        const k = sel.asset;
        const exists =
          content.placements.some((p) => `${p.assetId}|${p.folderId}` === k) ||
          content.recycleEntries.some((p) => `${p.assetId}|${p.folderId}` === k);
        if (exists) selection.selectOnly(k);
      } else if (sel.folder && findFolderNode(content.folders, sel.folder)) {
        selection.clickFolder(sel.folder);
      }
    });
  }
  // 按钮态响应式镜像（ContentTitleBar disabled 绑定）：panelId → 可后退 / 可前进
  const navCan = ref<Record<string, { back: boolean; fwd: boolean }>>({});

  function syncNavCan(): void {
    if (!api) return;
    const next: Record<string, { back: boolean; fwd: boolean }> = {};
    for (const p of api.panels) {
      const h = histories.get(p.id);
      next[p.id] = { back: (h?.back.length ?? 0) > 0, fwd: (h?.fwd.length ?? 0) > 0 };
    }
    navCan.value = next;
  }

  function panelInfo(p: IDockviewPanel): TabInfo {
    return {
      id: p.id,
      viewKey: (p.params?.viewKey as string) ?? "quick:all",
      locked: !!p.params?.locked,
    };
  }

  // 面板视口 key（2026-09-25 per-panel 预览会话化：预览会话按属主面板取列表源）——
  // 未绑定 api / panel 不存在 / params 缺 viewKey 均兜底 "quick:all"
  function viewKeyOf(panelId: string): string {
    return (api?.getPanel(panelId)?.params?.viewKey as string) ?? "quick:all";
  }

  // 镜像刷新 + 激活视口回写（侧边栏高亮跟随激活标签页）
  function syncMirror(): void {
    if (!api) return;
    // 已关闭 panel 的历史顺手清理（会话级，防 Map 只增不减）
    const alive = new Set(api.panels.map((p) => p.id));
    for (const id of [...histories.keys()]) {
      if (!alive.has(id)) histories.delete(id);
    }
    // 已关闭 panel 的预览会话联动回收（per-panel 注册表口径：先收集再删，防边遍历边改 Map；
    // dropPreviewSession 内含 close + $dispose + media/notes 会话联动销毁，2026-09-25）。
    // 只收 dockview 面板会话（"tab-" 前缀）——设计稿页 "__design__" 占位会话等特殊键不动
    const deadPanels: string[] = [];
    eachPreviewSession((s) => {
      if (s.panelId.startsWith("tab-") && !alive.has(s.panelId)) deadPanels.push(s.panelId);
    });
    for (const id of deadPanels) dropPreviewSession(id);
    tabs.value = api.panels.map(panelInfo);
    const active = api.activePanel;
    activeTabId.value = active?.id ?? "";
    if (active) {
      const vk = panelInfo(active).viewKey;
      if (content.sidebarSelected !== vk) content.sidebarSelected = vk;
    }
    syncNavCan();
  }

  // ---- 布局持久化（防抖 500ms，走 ui store 既有 subscribe 写回通道）----
  let persistTimer: ReturnType<typeof setTimeout> | undefined;
  function schedulePersist(): void {
    clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      if (api) ui.tabLayout = api.toJSON();
    }, 500);
  }

  // ---- 命令 ----

  // 导航即退出预览（2026-09-08 用户实机报：预览模式下点侧栏文件夹 / 快速访问，视口已切但
  // 中栏仍是预览画面，用户不知文件夹已换）。per-panel 会话化（2026-09-25）后 = 只退目标 panel
  // 自己的预览会话（peek 只查不建）；panelId 缺省 = 遍历全关（无 api 兜底分支用）。
  // 同视口点击（openView 早退）不算导航，预览保持。
  function exitPreview(panelId?: string): void {
    if (panelId) {
      const s = peekPreviewSession(panelId);
      if (s?.assetKey) s.close();
      return;
    }
    eachPreviewSession((s) => {
      if (s.assetKey) s.close();
    });
  }

  // 侧边栏 / 文件夹卡片等导航统一入口：锁定标签页不替换（新开），未锁定替换视口；
  // panelId 定向（2026-09-25 per-panel 会话化：包内视图跳回文件夹等指定目标 panel），
  // 缺省 = 激活 panel（行为与旧口径一致）。
  // 锁定新开分支不退预览（2026-10-02 用户实机报：锁定预览页点侧栏新开标签后回来预览没了——
  // 新开的是独立标签页，原页视口未替换，预览必须保留；「导航即退出预览」只管视口被替换的情形）
  function openView(viewKey: string, panelId?: string): void {
    if (!api) {
      content.sidebarSelected = viewKey;
      exitPreview();
      return;
    }
    const target = panelId ? api.getPanel(panelId) : api.activePanel;
    if (!target || target.params?.locked) {
      addTab(viewKey);
      return;
    }
    if (panelInfo(target).viewKey === viewKey) return;
    exitPreview(target.id);
    // 导航历史：旧视口（带选中快照）入后退栈、清前进栈（2026-08-20 面包屑接入；2026-08-24 快照）
    const h = histories.get(target.id) ?? { back: [], fwd: [] };
    h.back.push({ key: panelInfo(target).viewKey, sel: currentSel() });
    h.fwd = [];
    histories.set(target.id, h);
    target.api.updateParameters({ viewKey });
    target.api.setTitle(content.viewTitle(viewKey));
    // 侧边栏高亮只跟激活 panel（goHistory 同口径；onDidLayoutChange 再核对一遍）
    if (api.activePanel?.id === target.id) content.sidebarSelected = viewKey;
    syncNavCan();
  }

  // 面包屑后退 / 前进（2026-08-20 接入）：历史导航 = 本标签页内部行为，直接替换视口——
  // 不走 openView 的"锁定页新开"语义（历史是本页自己走过的路径，新开页反而迷失）；
  // 弹出条目的选中快照恢复（2026-08-24 用户定：返回默认选中卡片）
  function goHistory(panelId: string, dir: -1 | 1): void {
    if (!api) return;
    const panel = api.getPanel(panelId);
    const h = histories.get(panelId);
    if (!panel || !h) return;
    const [from, to] = dir === -1 ? [h.back, h.fwd] : [h.fwd, h.back];
    const target = from.pop();
    if (target === undefined) return;
    exitPreview(panelId);
    to.push({ key: panelInfo(panel).viewKey, sel: currentSel() });
    panel.api.updateParameters({ viewKey: target.key });
    panel.api.setTitle(content.viewTitle(target.key));
    if (api.activePanel?.id === panelId) content.sidebarSelected = target.key;
    restoreSel(target.sel);
    syncNavCan();
  }

  function addTab(viewKey = "quick:all", referenceGroupId?: string): string | null {
    if (!api) return null;
    const id = `tab-${Date.now()}-${seq++}`;
    api.addPanel({
      id,
      component: "content",
      tabComponent: "tab",
      title: content.viewTitle(viewKey),
      params: { viewKey, locked: false },
      ...(referenceGroupId
        ? { position: { referenceGroup: referenceGroupId, direction: "within" as const } }
        : {}),
    });
    return id;
  }

  // 资产「新标签页打开」（2026-09-25 per-panel 会话化配套接入）：新开同源视口的标签页并直接
  // 开该资产预览——会话即建即开（assetKey 先于 ContentArea 挂载就位，挂上即预览态）；
  // 新页标题经 TabLabel 会话感知派生 = 资产名（退出预览自动回视口名）
  function openPreviewInNewTab(sourcePanelId: string, assetKey: string): void {
    const id = addTab(viewKeyOf(sourcePanelId));
    if (!id) return;
    void previewSession(id).open(assetKey);
  }

  // 新标签页打开（文件夹/档案菜单"新标签页打开"，2026-08-20 接入）：不管当前页锁定与否恒新开——
  // 原实现误用 openView（未锁定页只替换视口，等于没开新页）。
  // 不动当前页预览（2026-10-02 同 openView 锁定分支口径：新开页是独立标签，原页预览保留）
  function openInNewTab(viewKey: string): void {
    if (!api) {
      content.sidebarSelected = viewKey;
      exitPreview();
      return;
    }
    addTab(viewKey);
  }

  function closeTab(id: string): void {
    const panel = api?.getPanel(id);
    if (!panel || panel.params?.locked) return;
    panel.api.close(); // 同步移除
    // 本轮约定：标签栏不留空——关完自动补默认"全部"（空状态形态待用户体验后定，记待办）
    if (api && api.panels.length === 0) addTab("quick:all");
  }

  // 激活标签页（2026-09-06 视图菜单「标签页」子菜单：点行切换激活；dockview setActive 标准路径）
  function activateTab(id: string): void {
    api?.getPanel(id)?.api.setActive();
  }

  // 关闭所有标签（≥2 时出现在右键菜单，2026-08-08 用户定）：锁定页尊重锁定语义跳过；关空补默认
  function closeAllTabs(): void {
    if (!api) return;
    for (const p of [...api.panels]) {
      if (!p.params?.locked) p.api.close();
    }
    if (api.panels.length === 0) addTab("quick:all");
  }

  // 关闭其他所有标签（排除右键目标项；锁定页同样跳过）
  function closeOtherTabs(id: string): void {
    if (!api) return;
    for (const p of [...api.panels]) {
      if (p.id !== id && !p.params?.locked) p.api.close();
    }
  }

  function toggleLock(id: string): void {
    const panel = api?.getPanel(id);
    if (!panel) return;
    panel.api.updateParameters({ locked: !panel.params?.locked });
  }

  // 左右 / 上下分屏（39 号稿，2026-08-08 定案：Obsidian 式——同视口在目标方向新开一个标签页，
  // 非移动。单标签页组也能分屏，且不碰"源组唯一 panel 移出即销毁"的边角）：
  // addPanel position { referencePanel, direction } 是 dockview 成熟路径（含均分尺寸）。
  // 前案 moveTo({position}) 不传 group 被强制 center（布局损坏全黑）；addGroup+moveTo 新组尺寸
  // undefined 为 0 不可见——均作废。
  // direction 四向全收（2026-09-07 单标签拖拽分屏拦截复用：左/右/上/下边缘拖入，
  // 右键菜单只用 right/below 两向）。
  // 预览态随视口一并复制（2026-10-02 用户实机报：预览中分屏新 pane 退出预览——分屏 = 当前视口
  // 完整复制，源 panel 在预览的资产在新 panel 直接进预览；会话即建即开，ContentArea 挂上即预览态，
  // 同 openPreviewInNewTab 链路；per-panel 会话化天然支持双预览并存）
  function splitTab(id: string, direction: "left" | "right" | "above" | "below"): void {
    const panel = api?.getPanel(id);
    if (!panel) return;
    const viewKey = (panel.params?.viewKey as string) ?? "quick:all";
    const locked = !!panel.params?.locked;
    const assetKey = peekPreviewSession(id)?.assetKey ?? null;
    const newPanel = api!.addPanel({
      id: `tab-${Date.now()}-${seq++}`,
      component: "content",
      tabComponent: "tab",
      title: content.viewTitle(viewKey),
      params: { viewKey, locked },
      position: { referencePanel: panel, direction },
    });
    if (assetKey) void previewSession(newPanel.id).open(assetKey);
  }

  // ---- 绑定与恢复 ----

  function restore(): void {
    if (!api) return;
    const saved = ui.tabLayout as SerializedDockview | null;
    if (!saved || typeof saved !== "object" || !("grid" in saved) || !("panels" in saved)) {
      addTab("quick:all");
      return;
    }
    try {
      api.fromJSON(saved);
      // 失效 viewKey 兜底（文件夹已删等）：params 不是字符串视口 key 才回退；
      // 文件夹存在性不校验——folders 未加载 / 已删时显示空视口 + "文件夹"标题，不崩（记待办：失效清理）
      for (const p of api.panels) {
        if (typeof p.params?.viewKey !== "string") {
          p.api.updateParameters({ viewKey: "quick:all", locked: false });
        }
        p.api.setTitle(content.viewTitle(panelInfo(p).viewKey));
      }
      if (api.panels.length === 0) addTab("quick:all");
    } catch (e) {
      console.warn("标签页布局恢复失败，回退默认：", e);
      try {
        api.clear();
      } catch {
        /* 忽略清理异常 */
      }
      addTab("quick:all");
    }
  }

  // DockHost onReady 调用：持有 api、订阅布局事件、等设置就绪后恢复上次布局。
  // 总是接受新实例：HMR 重挂 DockHost 时旧 dockview 已 dispose，api 必须换绑（否则新实例无 panel）
  async function bind(dockApi: DockviewApi): Promise<void> {
    api = markRaw(dockApi);
    dockApi.onDidLayoutChange(() => {
      syncMirror();
      schedulePersist();
    });
    dockApi.onDidActivePanelChange(() => syncMirror());
    await ui.initSettings(); // 幂等：App.vue 也在调，同一份就绪
    restore();
    syncMirror();
  }

  // dock 宿主尺寸同步的唯一入口（2026-08-10 走查修复）：由 App.vue 的 dockResizeObs 在 RO 阶段
  // （当帧 rAF 之后、绘制之前）以当前帧最终宽调用——dockview 自身 RO 回调被 rAF 包裹且持旧尺寸，
  // rAF 阶段灌宽会被其陈旧回调覆盖回上一帧（恒晚一帧 = 顶栏断开 / 中栏右缘拖拽根因）；
  // RO 阶段灌宽则每个绘制帧侧栏 / dock / 网格严格同宽（锚点不漂、无黑缝）
  function forceLayout(width: number, height: number): void {
    api?.layout(width, height, true);
  }

  return { tabs, activeTabId, navCan, bind, openView, openInNewTab, openPreviewInNewTab, goHistory, addTab, closeTab, activateTab, closeAllTabs, closeOtherTabs, toggleLock, splitTab, forceLayout, viewKeyOf };
});
