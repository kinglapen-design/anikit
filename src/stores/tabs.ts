// 多标签页（阶段一 1.2 主窗口重构，figma-css 15_标签栏 / 39_右键菜单-标签页）：
// dockview 为唯一布局源，本 store 是命令层（规范第 1 条）+ 响应式镜像。
// 行为约定（2026-08-08 随计划定案）：
// - 标签页 = 一个文件视口（viewKey，同 content.sidebarSelected 取值）；
// - 侧边栏导航：当前标签页未锁定 → 替换其视口；已锁定 → 新开标签页（Eagle 式）；
// - 锁定标签页：无关闭按钮（显示锁定图标）、右键"关闭标签页"禁用；不禁止拖拽排序 / 分屏；
// - 数据共享：所有标签页共用全局 content / selection store，标签页只记自己的 viewKey。
// 布局持久化：onDidLayoutChange 防抖 500ms → ui.tabLayout（settings.ui 子树，ui store 既有写回通道）。
import { defineStore } from "pinia";
import { markRaw, ref } from "vue";
import type { DockviewApi, IDockviewPanel, SerializedDockview } from "dockview-vue";
import { useContentStore } from "./content";
import { useUiStore } from "./ui";

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

  function panelInfo(p: IDockviewPanel): TabInfo {
    return {
      id: p.id,
      viewKey: (p.params?.viewKey as string) ?? "quick:all",
      locked: !!p.params?.locked,
    };
  }

  // 镜像刷新 + 激活视口回写（侧边栏高亮跟随激活标签页）
  function syncMirror(): void {
    if (!api) return;
    tabs.value = api.panels.map(panelInfo);
    const active = api.activePanel;
    activeTabId.value = active?.id ?? "";
    if (active) {
      const vk = panelInfo(active).viewKey;
      if (content.sidebarSelected !== vk) content.sidebarSelected = vk;
    }
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

  // 侧边栏 / 文件夹卡片等导航统一入口：锁定标签页不替换（新开），未锁定替换视口
  function openView(viewKey: string): void {
    if (!api) {
      content.sidebarSelected = viewKey;
      return;
    }
    const active = api.activePanel;
    if (!active || active.params?.locked) {
      addTab(viewKey);
      return;
    }
    if (panelInfo(active).viewKey === viewKey) return;
    active.api.updateParameters({ viewKey });
    active.api.setTitle(content.viewTitle(viewKey));
    content.sidebarSelected = viewKey; // 即时回写（onDidLayoutChange 再核对一遍）
  }

  function addTab(viewKey = "quick:all", referenceGroupId?: string): void {
    if (!api) return;
    api.addPanel({
      id: `tab-${Date.now()}-${seq++}`,
      component: "content",
      tabComponent: "tab",
      title: content.viewTitle(viewKey),
      params: { viewKey, locked: false },
      ...(referenceGroupId
        ? { position: { referenceGroup: referenceGroupId, direction: "within" as const } }
        : {}),
    });
  }

  function closeTab(id: string): void {
    const panel = api?.getPanel(id);
    if (!panel || panel.params?.locked) return;
    panel.api.close(); // 同步移除
    // 本轮约定：标签栏不留空——关完自动补默认"全部"（空状态形态待用户体验后定，记待办）
    if (api && api.panels.length === 0) addTab("quick:all");
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
  function splitTab(id: string, direction: "right" | "below"): void {
    const panel = api?.getPanel(id);
    if (!panel) return;
    const viewKey = (panel.params?.viewKey as string) ?? "quick:all";
    const locked = !!panel.params?.locked;
    api!.addPanel({
      id: `tab-${Date.now()}-${seq++}`,
      component: "content",
      tabComponent: "tab",
      title: content.viewTitle(viewKey),
      params: { viewKey, locked },
      position: { referencePanel: panel, direction },
    });
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

  return { tabs, activeTabId, bind, openView, addTab, closeTab, closeAllTabs, closeOtherTabs, toggleLock, splitTab, forceLayout };
});
