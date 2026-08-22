// 卡片悬停播放（5.3）：活跃实例注册表——播放层组件挂载时注册 pause/resume 回调、卸载时反注册；
// 主窗口隐藏到托盘时暂停全部活跃悬停播放（对齐 5.2 预览语义：最小化保持播放，隐藏停止），
// 重新显示时恢复（main-window-shown，2026-08-23 用户实机报修：隐藏再开后自动播放/悬停播放
// 全灭，需滚出视口或切文件夹才活——此前只有 pause 无恢复路径）。
// 预览单例（previewMedia）不走此表——预览 = 重新显示后手动再播（2026-07-27 用户定，不动）。
// media 元素归各播放层组件自管（与 previewMedia 预览单例隔离，中栏预览与卡片悬停可并存）。
import { listen } from "@tauri-apps/api/event";

interface HoverPlayer {
  pause: () => void;
  resume: () => void;
}

const active = new Set<HoverPlayer>();
let listening = false;

/** 注册悬停播放器；返回反注册函数（组件卸载时调用） */
export function registerHoverPlayer(pause: () => void, resume: () => void): () => void {
  const entry: HoverPlayer = { pause, resume };
  active.add(entry);
  if (!listening) {
    listening = true;
    void listen("main-window-hidden", () => {
      for (const p of active) p.pause();
    });
    void listen("main-window-shown", () => {
      for (const p of active) p.resume();
    });
  }
  return () => {
    active.delete(entry);
  };
}
