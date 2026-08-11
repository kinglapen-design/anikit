// 主窗口几何持久化（2026-08-12 自 App.vue 拆出，规范第 3 条行数控制；
// 2026-07-28 用户定："用户设置要持久化，包括窗口大小，位置"）：
// resize/move 高频事件防抖 500ms 整体写回全局 config（settings.window 子树）；
// 仅主窗口记录（预览子窗口不记录不恢复，enabled = false）；最大化时后端只刷标志位、几何保留还原尺寸
import { onBeforeUnmount, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useWindowStatePersistence(opts: { enabled: boolean }): void {
  const appWindow = getCurrentWindow();
  let winStateTimer: ReturnType<typeof setTimeout> | undefined;
  let unlistenMoved: (() => void) | undefined;
  let unlistenResized: (() => void) | undefined;

  function scheduleSaveWindowState(): void {
    clearTimeout(winStateTimer);
    winStateTimer = setTimeout(() => void saveWindowState(), 500);
  }
  async function saveWindowState(): Promise<void> {
    try {
      // 2026-07-29 复验修复（窗口缩成一小块）：最小化 / 隐藏托盘等场景 outerSize 可能报退化尺寸，
      // 写回后每轮启动都恢复小窗——最小化不写、小于最小可用尺寸（800×500）不写
      if (await appWindow.isMinimized()) return;
      const [pos, size, maximized] = await Promise.all([
        appWindow.outerPosition(),
        appWindow.outerSize(),
        appWindow.isMaximized(),
      ]);
      // 尺寸退化（237×39）或坐标飞去屏外（-32000）一律不写
      if (size.width < 800 || size.height < 500 || pos.x < -10000 || pos.y < -10000) return;
      await invoke("save_window_state", {
        state: { x: pos.x, y: pos.y, width: size.width, height: size.height, maximized },
      });
    } catch (e) {
      console.warn("窗口几何写回失败（静默降级）：", e);
    }
  }

  onMounted(() => {
    if (!opts.enabled) return; // 预览子窗口不记录
    void appWindow.onMoved(() => scheduleSaveWindowState()).then((fn) => (unlistenMoved = fn));
    void appWindow.onResized(() => scheduleSaveWindowState()).then((fn) => (unlistenResized = fn));
  });
  onBeforeUnmount(() => {
    clearTimeout(winStateTimer);
    unlistenMoved?.();
    unlistenResized?.();
  });
}
