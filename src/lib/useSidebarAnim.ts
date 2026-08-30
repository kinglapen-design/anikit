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

// 时长与缓动（2026-08-30 用户实机定）：400ms 大幅度位移档（--n-dur-400）+
// token 缓入缓出 cubic-bezier(.32,.08,.1,1)（--n-ease-in-out 的 JS 求值——rAF 补间用不了 CSS 曲线）。
// 旧 180ms + easeInOutCubic 作废。
const SIDEBAR_ANIM_MS = 400;
function easeInOut(t: number): number {
  // cubic-bezier(.32,.08,.1,1) 求值：牛顿迭代解 x(u)=t 得 u，返回 y(u)；不收敛时二分兜底
  const [x1, y1, x2, y2] = [0.32, 0.08, 0.1, 1];
  const cx = 3 * x1, bx = 3 * (x2 - x1) - cx, ax = 1 - cx - bx;
  const cy = 3 * y1, by = 3 * (y2 - y1) - cy, ay = 1 - cy - by;
  const sx = (u: number): number => ((ax * u + bx) * u + cx) * u;
  const sy = (u: number): number => ((ay * u + by) * u + cy) * u;
  let u = t;
  for (let i = 0; i < 8; i++) {
    const x = sx(u) - t;
    if (Math.abs(x) < 1e-6) return sy(u);
    const d = (3 * ax * u + 2 * bx) * u + cx;
    if (Math.abs(d) < 1e-6) break;
    u -= x / d;
  }
  let lo = 0, hi = 1;
  u = t;
  while (hi - lo > 1e-6) {
    const x = sx(u);
    if (Math.abs(x - t) < 1e-6) break;
    if (x < t) lo = u;
    else hi = u;
    u = (lo + hi) / 2;
  }
  return sy(u);
}

/** 各侧栏实际显示宽度（px）：模板唯一绑定源，rAF 引擎逐帧写，禁其他写点（见文件头注释） */
export const sidebarShowWidths = reactive({ left: 0, right: 0 });

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
}): void {
  const ui = useUiStore();
  const tabs = useTabsStore();
  let sideAnims: SideAnim[] = [];
  let sideAnimRaf = 0;
  let sideAnimWatchdog: ReturnType<typeof setTimeout> | undefined;
  let dockResizeObs: ResizeObserver | undefined;

  // 初始显示宽度（模板首帧即正确，不经过 0/全宽跳变）
  sidebarShowWidths.left = ui.leftCollapsed ? 0 : ui.leftWidth;
  sidebarShowWidths.right = ui.rightCollapsed ? 0 : ui.rightWidth;

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
      clearTimeout(sideAnimWatchdog);
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
    // watchdog（2026-08-30：rAF 在窗口遮挡 / headless 节流下可暂停，布尔会恒卡 true——
    // pad-win 让位不解除、标签栏右端恒 160 空白的实机 BUG 根因）：超时强制收官写终值
    clearTimeout(sideAnimWatchdog);
    sideAnimWatchdog = setTimeout(() => {
      if (!sideAnims.length) return;
      cancelAnimationFrame(sideAnimRaf);
      for (const a of sideAnims) {
        sidebarShowWidths[a.key] = a.to;
        a.el.style.width = `${a.to}px`;
      }
      sideAnims = [];
      ui.sidebarAnimating = false;
    }, SIDEBAR_ANIM_MS + 200);
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
        if (l !== pl) startSidebarAnim("left", opts.leftEl(), l, ui.leftWidth);
        if (r !== pr) startSidebarAnim("right", opts.rightEl(), r, ui.rightWidth);
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
    clearTimeout(sideAnimWatchdog);
    dockResizeObs?.disconnect();
  });
}
