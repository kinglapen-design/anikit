// 侧栏收展动画引擎 composable（2026-08-12 自 App.vue 拆出，规范第 3 条行数控制；
// 锚点模型 2026-08-08 定案 v6；同步机制 2026-08-10 走查修复改 RO）：
// 收左栏 = 中间面板右缘钉死、向左拉宽；收右栏 = 左缘钉死、向右拉宽（flex 布局天然如此）。
// 动画不用 CSS transition，同一 rAF 回调里 JS 逐帧写侧栏宽度。
// dockview 尺寸同步是难点：其自身 ResizeObserver 回调被 requestAnimationFrame 包裹
// （dockview-core dom.ts watchElementResize），且回调拿到的是触发时的旧尺寸——
// 同帧 rAF 里 forceLayout 灌当前宽，也会被紧随其后执行的 dockview 陈旧回调覆盖回上一帧
// （2026-08-10 CDP 逐帧实测：网格渲染宽恒等于 dock 宿主上一帧宽，滞后最多 ~75px——
// 顶栏断开露 C1、中栏右缘被往左拽再回弹，同此根因）。
// 故改用 ResizeObserver 驱动：RO 阶段在当帧全部 rAF 之后、绘制之前，此刻读到的就是
// 当帧最终宽，forceLayout 同步写网格——每个绘制帧侧栏 / dock / 网格严格同宽，零滞后。
//
// 显示宽度单一持有（2026-08-15 闪帧根治，任务 B）：模板只绑 sidebarShowWidths，
// 禁绑 collapsed 布尔——旧写法 `:style width: collapsed ? 0 : fullWidth` 在 toggle 当帧
// 模板立即跳全宽（展开）/ 跳 0（收起），rAF 引擎又从另一端逐帧灌宽，双写打架 =
// 首帧全宽 → 下一帧 0 → 再展开的闪帧（收起对称：首帧 0 → 弹回全宽 → 再收）。
// 现：toggle 只翻布尔，watch 起动画，rAF 逐帧写 sidebarShowWidths（当前宽 → 目标宽），
// 模板随它走，任何时刻只有一个写点，无跳变帧。
import { onBeforeUnmount, onMounted, reactive, watch } from "vue";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const SIDEBAR_ANIM_MS = 180;
function easeInOut(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;
}

/** 档案二级侧边栏宽（250，同右栏） */
export const ARCHIVE_PANEL_W = 250;

/** 各侧栏实际显示宽度（px）：模板唯一绑定源，rAF 引擎逐帧写，禁其他写点（见文件头注释） */
export const sidebarShowWidths = reactive({ left: 0, right: 0, archive: 0 });

type SideKey = keyof typeof sidebarShowWidths;
interface SideAnim {
  key: SideKey;
  el: HTMLElement;
  from: number;
  to: number;
  start: number;
}

export function useSidebarAnim(opts: {
  leftEl: () => HTMLElement | undefined;
  rightEl: () => HTMLElement | undefined;
  dockEl: () => HTMLElement | undefined;
  archiveEl?: () => HTMLElement | undefined;
}): void {
  const ui = useUiStore();
  const tabs = useTabsStore();
  let sideAnims: SideAnim[] = [];
  let sideAnimRaf = 0;
  let dockResizeObs: ResizeObserver | undefined;

  // 初始显示宽度（模板首帧即正确，不经过 0/全宽跳变）
  sidebarShowWidths.left = ui.leftCollapsed ? 0 : ui.leftWidth;
  sidebarShowWidths.right = ui.rightCollapsed ? 0 : ui.rightWidth;
  sidebarShowWidths.archive = ui.archivePanelOpen ? ARCHIVE_PANEL_W : 0;

  function sidebarAnimFrame(now: number): void {
    let active = false;
    for (const a of sideAnims) {
      const t = Math.min(1, (now - a.start) / SIDEBAR_ANIM_MS);
      const w = a.from + (a.to - a.from) * easeInOut(t);
      sidebarShowWidths[a.key] = w; // 单一写点：模板绑定随帧更新
      a.el.style.width = `${w}px`; // 直写兜底（防帧内其他原因触发的重渲染读到旧值）
      if (t < 1) active = true;
    }
    // dockview 网格宽度由 dockResizeObs 在 RO 阶段同步（见文件头注释），此处不再灌宽
    if (active) {
      sideAnimRaf = requestAnimationFrame(sidebarAnimFrame);
    } else {
      sideAnims = [];
      ui.sidebarAnimating = false;
    }
  }

  function startSidebarAnim(key: SideKey, el: HTMLElement | undefined, collapsed: boolean, fullWidth: number): void {
    if (!el) return;
    sideAnims = sideAnims.filter((a) => a.key !== key);
    // 起点 = 当前视觉宽度（动画途中反向切换从此续接，不跳变）
    sideAnims.push({
      key,
      el,
      from: el.getBoundingClientRect().width,
      to: collapsed ? 0 : fullWidth,
      start: performance.now(),
    });
    ui.sidebarAnimating = true;
    cancelAnimationFrame(sideAnimRaf);
    sideAnimRaf = requestAnimationFrame(sidebarAnimFrame);
  }

  onMounted(() => {
    // dockview 逐帧同步（机制见文件头注释）：dock 宿主尺寸变化在 RO 阶段（当帧 rAF 之后、
    // 绘制之前）以当前帧最终宽灌给网格；侧栏收展动画、窗口拖改尺寸统一走此路径
    const dockEl = opts.dockEl();
    if (dockEl) {
      dockResizeObs = new ResizeObserver((entries) => {
        const rect = entries[0]?.contentRect;
        if (rect) tabs.forceLayout(rect.width, rect.height);
      });
      dockResizeObs.observe(dockEl);
    }
    // 侧栏收展：状态翻转后由 JS 逐帧驱动宽度（见文件头注释）
    watch(
      [() => ui.leftCollapsed, () => ui.rightCollapsed, () => ui.archivePanelOpen],
      ([l, r, a], [pl, pr, pa]) => {
        if (l !== pl) startSidebarAnim("left", opts.leftEl(), l, ui.leftWidth);
        if (r !== pr) startSidebarAnim("right", opts.rightEl(), r, ui.rightWidth);
        if (a !== pa && opts.archiveEl) startSidebarAnim("archive", opts.archiveEl(), !a, ARCHIVE_PANEL_W);
      },
    );
    // SplitBar 拖改全宽：非动画期间显示宽度即时跟随（Collapsed 时不跟随，保持 0）
    watch([() => ui.leftWidth, () => ui.rightWidth], ([lw, rw]) => {
      if (!ui.leftCollapsed && !sideAnims.some((a) => a.key === "left")) sidebarShowWidths.left = lw;
      if (!ui.rightCollapsed && !sideAnims.some((a) => a.key === "right")) sidebarShowWidths.right = rw;
    });
  });
  onBeforeUnmount(() => {
    cancelAnimationFrame(sideAnimRaf);
    dockResizeObs?.disconnect();
  });
}
