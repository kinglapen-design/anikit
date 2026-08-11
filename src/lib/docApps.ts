// 新建文档软件检测（1.3，2026-08-11 用户定）：每次打开 UI 由 App.vue 调 ensureDocApps，
// 后端按 Windows 文件关联判定软件是否安装并提取主程序图标（PNG data URL）。
// 结果驱动 07 新增菜单"新建文档"组：未安装 → 删除对应新建项；已安装 → 带系统图标显示
//（新建文档业务阶段三 3.1 接，本轮项仍 disabled 占位）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface DocAppInfo {
  id: string; // animate / photoshop / afterEffects / illustrator / word / excel / text
  installed: boolean;
  icon: string | null; // PNG data URL
}

export const docApps = reactive({
  loaded: false,
  apps: [] as DocAppInfo[],
});

export function docApp(id: string): DocAppInfo | undefined {
  return docApps.apps.find((a) => a.id === id);
}

// 幂等加载（每次打开 UI 调用一次 = 打开时监测）
export async function ensureDocApps(): Promise<void> {
  if (docApps.loaded) return;
  docApps.loaded = true;
  try {
    docApps.apps = await invoke<DocAppInfo[]>("detect_doc_apps");
  } catch (e) {
    docApps.loaded = false; // 失败下次重试
    console.warn("新建文档软件检测失败", e);
  }
}
