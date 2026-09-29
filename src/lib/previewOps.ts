// 预览子窗口打开封装（2026-09-29 兼容轮）：open_preview_window 后端命令（同资产单例聚焦；
// 透明窗口创建失败后端自动降级非透明重建）。失败 toast 透出——此前各菜单 void invoke
// 静默吞错，透明窗口创建失败的机型上表现为「点击没反应」零线索（仿 plugins.ts
// openPluginWindow 口径：结果 toast 在本封装内）。
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../stores/toast";

export async function openPreviewWindow(assetKey: string): Promise<void> {
  const toast = useToastStore();
  try {
    await invoke("open_preview_window", { assetKey });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}
