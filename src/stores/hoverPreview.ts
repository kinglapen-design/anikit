// 卡片悬停播放（5.3）：活跃实例注册表——播放层组件挂载时注册 pause 回调、卸载时反注册；
// 主窗口隐藏到托盘时暂停全部活跃悬停播放（对齐 5.2 预览语义：最小化保持播放，隐藏停止）。
// media 元素归各播放层组件自管（与 previewMedia 预览单例隔离，中栏预览与卡片悬停可并存）。
import { listen } from "@tauri-apps/api/event";

const active = new Set<() => void>();
let listening = false;

/** 注册悬停播放器；返回反注册函数（组件卸载时调用） */
export function registerHoverPlayer(pause: () => void): () => void {
  active.add(pause);
  if (!listening) {
    listening = true;
    void listen("main-window-hidden", () => {
      for (const p of active) p();
    });
  }
  return () => {
    active.delete(pause);
  };
}
