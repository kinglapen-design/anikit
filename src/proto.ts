// 任务 4 spike：10 万级滚动原型（自定义滚动 · transform 驱动）
//
// 为什么不用原生滚动条：浏览器对滚动条拖拽是合成器先行——拖块 1px，
// 十万条内容瞬移上千 px，主线程收到 scroll 事件时浏览器已画完一帧空白
// （棋盘格闪白），buffer 再大也盖不住瞬移。
// 自定义滚动把"渲染目标区域"和"挪动位置"放进同一个 JS 任务、同一帧提交，
// 从机制上消除起步闪白。代价：滚轮/拖拽/键盘滚动需自行实现。
import { invoke } from "@tauri-apps/api/core";
import justifiedLayout from "justified-layout";

interface FakeItem {
  id: number;
  name: string;
  w: number;
  h: number;
  color: string;
}

interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
}

interface Row {
  top: number;
  height: number;
  cards: { item: FakeItem; box: Box }[];
}

const TARGET_ROW_H = 220;
const BUFFER = 800;

export async function mountProto(root: HTMLElement, onBack: () => void): Promise<void> {
  root.innerHTML = `
    <div style="height:100vh;display:flex;flex-direction:column;background:#1e1e1e;color:#d4d4d4;font-family:'Microsoft YaHei',system-ui,sans-serif;">
      <div style="display:flex;align-items:center;gap:16px;padding:10px 16px;border-bottom:1px solid #333;flex:none;">
        <button id="back" style="padding:4px 12px;">← 返回</button>
        <span id="stats" style="opacity:.8;font-size:13px;">加载中……</span>
        <span style="margin-left:auto;opacity:.45;font-size:12px;">自定义滚动 · transform 驱动</span>
      </div>
      <div id="viewport" tabindex="0" style="flex:1;overflow:hidden;position:relative;outline:none;">
        <div id="canvas" style="position:absolute;left:0;top:0;right:0;will-change:transform;"></div>
        <div id="track" style="position:absolute;right:2px;top:0;bottom:0;width:10px;z-index:10;">
          <div id="thumb" style="position:absolute;left:0;width:100%;background:rgba(255,255,255,.3);border-radius:5px;cursor:grab;"></div>
        </div>
      </div>
    </div>`;

  const stats = root.querySelector<HTMLSpanElement>("#stats")!;
  const viewport = root.querySelector<HTMLDivElement>("#viewport")!;
  const canvas = root.querySelector<HTMLDivElement>("#canvas")!;
  const track = root.querySelector<HTMLDivElement>("#track")!;
  const thumb = root.querySelector<HTMLDivElement>("#thumb")!;
  root.querySelector<HTMLButtonElement>("#back")!.onclick = onBack;

  // 1) 加载（Rust 读盘 + 解析 + IPC）
  const t0 = performance.now();
  let items: FakeItem[];
  try {
    items = await invoke<FakeItem[]>("load_fake_items");
  } catch (e) {
    stats.textContent = String(e);
    return;
  }
  const tInvoke = performance.now() - t0;

  // 2) 布局预计算（等高不等宽，全部用已知宽高，不读图）
  const t1 = performance.now();
  const layout = () => {
    const geo = justifiedLayout(
      items.map((i) => ({ width: i.w, height: i.h })),
      { containerWidth: viewport.clientWidth, targetRowHeight: TARGET_ROW_H, boxSpacing: 8, containerPadding: 8 }
    );
    const rows: Row[] = [];
    const boxes = geo.boxes as unknown as Box[];
    for (let i = 0; i < boxes.length; i++) {
      const box = boxes[i];
      let row = rows[rows.length - 1];
      if (!row || row.top !== box.top) {
        row = { top: box.top, height: box.height, cards: [] };
        rows.push(row);
      }
      row.cards.push({ item: items[i], box });
    }
    return { rows, total: geo.containerHeight };
  };
  let { rows, total } = layout();
  const tLayout = performance.now() - t1;
  stats.textContent = `${items.length} 条 | 加载+IPC ${tInvoke.toFixed(0)}ms | 布局 ${tLayout.toFixed(0)}ms | ${rows.length} 行`;

  // 3) 行索引与增量渲染
  let tops = rows.map((r) => r.top);
  const lowerBound = (v: number): number => {
    let lo = 0,
      hi = tops.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (tops[mid] < v) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  };

  const rendered = new Map<number, HTMLDivElement>();
  const buildRow = (r: number): HTMLDivElement => {
    const rowDiv = document.createElement("div");
    for (const { item, box } of rows[r].cards) {
      const d = document.createElement("div");
      d.style.cssText =
        `position:absolute;left:${box.left}px;top:${box.top}px;width:${box.width}px;height:${box.height}px;` +
        `background:${item.color};border-radius:6px;box-sizing:border-box;padding:6px;` +
        `font-size:11px;color:rgba(255,255,255,.85);overflow:hidden;white-space:nowrap;text-overflow:ellipsis;`;
      d.textContent = item.name;
      rowDiv.appendChild(d);
    }
    return rowDiv;
  };

  const renderAt = (top: number) => {
    const y0 = top - BUFFER;
    const y1 = top + viewport.clientHeight + BUFFER;
    const r0 = Math.max(0, lowerBound(y0) - 1);
    const r1 = Math.min(rows.length, lowerBound(y1) + 1);
    for (const [idx, el] of rendered) {
      if (idx < r0 || idx >= r1) {
        el.remove();
        rendered.delete(idx);
      }
    }
    for (let r = r0; r < r1; r++) {
      if (rendered.has(r)) continue;
      const rowDiv = buildRow(r);
      canvas.appendChild(rowDiv);
      rendered.set(r, rowDiv);
    }
  };

  // 4) 自定义滚动：先渲染、后挪位置，同一任务同一帧
  let pos = 0;
  const viewH = () => viewport.clientHeight;
  const update = (next: number) => {
    pos = Math.max(0, Math.min(next, total - viewH()));
    renderAt(pos);
    canvas.style.transform = `translateY(${-pos}px)`;
    const th = Math.max(30, (viewH() / total) * viewH());
    const tt = total <= viewH() ? 0 : (pos / (total - viewH())) * (viewH() - th);
    thumb.style.height = `${th}px`;
    thumb.style.top = `${tt}px`;
  };

  viewport.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      update(pos + e.deltaY * 1.5);
    },
    { passive: false }
  );

  // 拖块
  let dragY = 0;
  let dragPos = 0;
  thumb.addEventListener("pointerdown", (e) => {
    dragY = e.clientY;
    dragPos = pos;
    thumb.setPointerCapture(e.pointerId);
    thumb.style.cursor = "grabbing";
    e.preventDefault();
  });
  thumb.addEventListener("pointermove", (e) => {
    if (!thumb.hasPointerCapture(e.pointerId)) return;
    const trackH = viewH() - thumb.clientHeight;
    update(dragPos + ((e.clientY - dragY) / trackH) * (total - viewH()));
  });
  thumb.addEventListener("pointerup", (e) => {
    thumb.releasePointerCapture(e.pointerId);
    thumb.style.cursor = "grab";
  });

  // 点轨道翻页
  track.addEventListener("pointerdown", (e) => {
    if (e.target === thumb) return;
    const dir = e.clientY > thumb.getBoundingClientRect().bottom ? 1 : -1;
    update(pos + dir * viewH() * 0.9);
  });

  // 键盘
  viewport.addEventListener("keydown", (e) => {
    const page = viewH() * 0.9;
    if (e.key === "PageDown") update(pos + page);
    else if (e.key === "PageUp") update(pos - page);
    else if (e.key === "Home") update(0);
    else if (e.key === "End") update(total);
    else if (e.key === "ArrowDown") update(pos + 120);
    else if (e.key === "ArrowUp") update(pos - 120);
    else return;
    e.preventDefault();
  });

  window.addEventListener("resize", () => {
    ({ rows, total } = layout());
    tops = rows.map((r) => r.top);
    rendered.clear();
    canvas.replaceChildren();
    update(pos);
  });

  update(0);
}
