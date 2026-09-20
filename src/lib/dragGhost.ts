// 拖拽 ghost（6.1 C4 自 dragOperation 拆出，规范第 3 条行数控制）：
// 2026-07-28 复验反馈 6 美化（参照用户 Electron 版截图语义）——
//   缩略图短边约 96px / 整体 opacity var(--opacity-drag-ghost)（出现即定值，不做位移动画）/
//   圆角 8 / --shadow-pop 投影 / 1px border-1（图片铺满容器，inset shadow 会被盖——真 border，规范第 4 条例外）；
//   多选计数角标（品牌紫底白字）；无缩略图时给格式角标 + 名称纸片（同 96 短边规则）。
const GHOST_SHORT = 96; // ghost 缩略图 / 纸片短边

export function makeGhost(opts: {
  anchor: HTMLElement | null;
  kind: "asset" | "folder";
  count: number; // 拖拽项数（>1 出计数角标）
}): HTMLElement {
  const ghost = document.createElement("div");
  ghost.style.cssText = "position:fixed;z-index:3000;pointer-events:none;opacity:0.75;";
  // 取卡片已渲染 <img> 的 currentSrc（2026-07-29 复验二轮 3：anchor 已由引擎 closest 到卡片根，
  // currentSrc 涵盖 PSD 缓存图 / 波形 / 视频首帧 / 序列帧封面全部形态；无 img 才走纸片 fallback）
  const img = opts.anchor?.querySelector("img");
  const src = img?.currentSrc || img?.src;
  if (img && src) {
    const g = document.createElement("img");
    g.src = src;
    g.draggable = false;
    // 短边 ≈96：按源缩略图渲染宽高比缩放（布局已完成，直接读渲染尺寸免等加载）
    const r = img.getBoundingClientRect();
    if (r.width > 0 && r.height > 0) {
      const k = GHOST_SHORT / Math.min(r.width, r.height);
      g.style.width = `${Math.round(r.width * k)}px`;
      g.style.height = `${Math.round(r.height * k)}px`;
    } else {
      g.style.maxWidth = `${GHOST_SHORT}px`;
      g.style.maxHeight = `${GHOST_SHORT}px`;
    }
    g.style.cssText +=
      "display:block;object-fit:cover;border-radius:var(--n-radius-8);" +
      "border:1px solid var(--n-l-a2);box-shadow:var(--n-shadow-lg);";
    ghost.appendChild(g);
  } else {
    // 无缩略图（格式图标卡片 / 文件夹）：格式角标 + 名称纸片（同 96 短边规则）
    const name =
      opts.anchor?.getAttribute("data-drag-name") ?? (opts.kind === "folder" ? "文件夹" : "素材");
    const chip = document.createElement("div");
    chip.style.cssText =
      `position:relative;display:flex;align-items:center;justify-content:center;` +
      `min-width:${GHOST_SHORT}px;min-height:${GHOST_SHORT}px;max-width:200px;box-sizing:border-box;` +
      "padding:var(--n-space-16) var(--n-space-8) var(--n-space-8);border-radius:var(--n-radius-8);" +
      "background:var(--n-g4);box-shadow:var(--n-shadow-lg),inset 0 0 0 1px var(--n-l-a2);";
    const m = /\.([^.]+)$/.exec(name);
    if (opts.kind === "asset" && m) {
      const fmt = document.createElement("span");
      fmt.textContent = m[1].toUpperCase();
      fmt.style.cssText =
        "position:absolute;top:4px;left:4px;padding:0 var(--n-space-4);border-radius:var(--n-radius-4);" +
        "background:color-mix(in srgb, var(--n-g1) 40%, transparent);color:var(--n-l2);font-size:var(--n-fs-secondary);" +
        "line-height:var(--n-lh-secondary);font-weight:var(--n-fw-bold);";
      chip.appendChild(fmt);
    }
    const label = document.createElement("span");
    label.textContent = name;
    label.style.cssText =
      "color:var(--n-l1);font-size:var(--n-fs-secondary);line-height:var(--n-lh-secondary);" +
      "text-align:center;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;" +
      "overflow:hidden;word-break:break-all;";
    chip.appendChild(label);
    ghost.appendChild(chip);
  }
  if (opts.count > 1) {
    const badge = document.createElement("span");
    badge.textContent = String(opts.count);
    badge.style.cssText =
      "position:absolute;top:-6px;right:-6px;min-width:18px;height:18px;border-radius:9px;" +
      "background:var(--n-brand);color:var(--n-l1);font-size:var(--n-fs-secondary);" +
      "line-height:18px;text-align:center;padding:0 var(--n-space-4);";
    ghost.appendChild(badge);
  }
  document.body.appendChild(ghost);
  return ghost;
}

export function moveGhost(ghost: HTMLElement, x: number, y: number): void {
  ghost.style.left = `${x}px`;
  ghost.style.top = `${y}px`;
  ghost.style.transform = "translate(-50%, -50%)";
}
