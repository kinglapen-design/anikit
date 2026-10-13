// 空格放大预览 composable（2026-08-12 自 AssetGrid 拆出，规范第 3 条行数控制；
// 2026-08-10 用户定：访达 Quick Look 逻辑，取代放大镜按钮）：
// **悬停优先**——鼠标悬停的卡片（无需选中）空格即览，无悬停回退单选项；
// 浮层开着时：悬停在另一张卡上按空格 = 切换查看（同日四轮修复浮层不重载后恢复），
// 同卡 / 无悬停 = 关闭；Esc 由浮层自处理；选择跟随：开启期间单选切换换图、多选 / 清空关闭。
// 悬停判定：按键瞬间 elementFromPoint 取鼠标位命中（穿透 pointer-events:none 浮层，
// 拖拽引擎同款手法）。滚动条悬停显隐（同日）共用 panelHover。
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useSelectionStore } from "../stores/selection";
import { placementKey, type PlacementView } from "../stores/content";
import { useUiStore } from "../stores/ui";
import { isEditableTarget } from "./useGlobalKeys";

export function useSpaceZoom(opts: { items: () => PlacementView[]; panelActive: () => boolean }) {
  const selection = useSelectionStore();
  const ui = useUiStore();
  const zoomAsset = ref<PlacementView | null>(null);
  const panelHover = ref(false);
  let lastPointer = { x: 0, y: 0 };
  function findByKey(key: string | null): PlacementView | null {
    return key ? (opts.items().find((i) => placementKey(i) === key) ?? null) : null;
  }
  // 指针位置追踪（网格 pointermove 时由组件调用，与框选移动同线）
  function trackPointer(e: PointerEvent): void {
    lastPointer = { x: e.clientX, y: e.clientY };
  }
  function hoverKeyAtPoint(): string | null {
    const el = document.elementFromPoint(lastPointer.x, lastPointer.y);
    const card = el?.closest(".asset-card, .list-row");
    return card?.getAttribute("data-key") ?? null;
  }
  function onGridMouseLeave(): void {
    panelHover.value = false;
  }
  function onGridKeydown(e: KeyboardEvent): void {
    if (e.key !== " ") return;
    if (isEditableTarget(e.target as HTMLElement)) return; // 重命名输入中不触发
    if (!opts.panelActive()) return; // window 级监听多 panel 重复注册：仅激活标签页响应（2026-08-10）
    // 浮层开着时交给其 window 级监听处理（同日：焦点不在网格时空格也要生效；此处不处理防双触发）
    if (zoomAsset.value) return;
    e.preventDefault();
    // 空格预览模式（2026-10-13 偏好设置「通用」区块，ui.spaceZoomMode 持久化）：
    // selected = 仅当前选中项（悬停不跟随；无选中 / 多选保守不动作）；
    // hover = 悬停优先、无悬停回退单选项（2026-08-10 原语义不变）
    const p =
      ui.spaceZoomMode === "selected"
        ? selection.count === 1
          ? findByKey(selection.first)
          : null
        : (findByKey(hoverKeyAtPoint()) ?? (selection.count === 1 ? findByKey(selection.first) : null));
    if (p) zoomAsset.value = p;
  }
  // 浮层空格（window 级，焦点任意）：**纯 toggle 再按即关**（2026-08-10 用户定二轮：
  // 悬停切换删除——浮层遮挡大、用户不知切换功能时，想关闭却误换图反而困惑）
  function onZoomSpace(): void {
    zoomAsset.value = null;
  }
  watch(
    () => [selection.first, selection.count],
    () => {
      if (!zoomAsset.value) return;
      if (selection.count !== 1) {
        zoomAsset.value = null;
        return;
      }
      zoomAsset.value = findByKey(selection.first);
    },
  );
  // 空格预览走 window 级（2026-08-10：焦点不在网格时也要能打开；active 闸防多 panel 重复响应）
  onMounted(() => window.addEventListener("keydown", onGridKeydown));
  onBeforeUnmount(() => window.removeEventListener("keydown", onGridKeydown));

  return { zoomAsset, panelHover, trackPointer, onGridMouseLeave, onZoomSpace };
}
