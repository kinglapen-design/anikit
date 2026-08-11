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
import { onBeforeUnmount, onMounted, watch } from "vue";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const SIDEBAR_ANIM_MS = 180;
function easeInOut(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;
}
interface SideAnim {
  el: HTMLElement;
  from: number;
  to: number;
  start: number;
}

export function useSidebarAnim(opts: {
  leftEl: () => HTMLElement | undefined;
  rightEl: () => HTMLElement | undefined;
  dockEl: () => HTMLElement | undefined;
}): void {
  const ui = useUiStore();
  const tabs = useTabsStore();
  let sideAnims: SideAnim[] = [];
  let sideAnimRaf = 0;
  let dockResizeObs: ResizeObserver | undefined;

  function sidebarAnimFrame(now: number): void {
    let active = false;
    for (const a of sideAnims) {
      const t = Math.min(1, (now - a.start) / SIDEBAR_ANIM_MS);
      a.el.style.width = `${a.from + (a.to - a.from) * easeInOut(t)}px`;
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

  function startSidebarAnim(el: HTMLElement | undefined, collapsed: boolean, fullWidth: number): void {
    if (!el) return;
    sideAnims = sideAnims.filter((a) => a.el !== el);
    // 起点 = 当前视觉宽度（动画途中反向切换从此续接，不跳变）
    sideAnims.push({
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
      [() => ui.leftCollapsed, () => ui.rightCollapsed],
      ([l, r], [pl, pr]) => {
        if (l !== pl) {
          startSidebarAnim(opts.leftEl(), l, ui.leftWidth);
        }
        if (r !== pr) {
          startSidebarAnim(opts.rightEl(), r, ui.rightWidth);
        }
      },
    );
  });
  onBeforeUnmount(() => {
    cancelAnimationFrame(sideAnimRaf);
    dockResizeObs?.disconnect();
  });
}
