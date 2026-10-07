// anikit 链接工具（2026-10-08 双形态分工，PRD 02-09）：anikit://item|folder/{id} = 内部标识符，
// http://127.0.0.1:{port}/(open/)item|folder/{id} = 对外可点击形态——渲染 / 粘贴 / 跳转 / 打包共用
// 同一归一化解析器；media-open 事件 = 媒体服务 /open 端点点击跳转（S1，server/media.rs）

import { nextTick } from "vue";
import { listen } from "@tauri-apps/api/event";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { jumpToAssetPlacement } from "./relationOps";

export interface AnikitRef {
  kind: "item" | "folder";
  id: string;
}

/** anikit 引用归一化解析（trim 后全串匹配）：anikit://item|folder/{id} 与
 *  http(s)://127.0.0.1:{port}/(open/)item|folder/{id} 同认 */
export function parseAnikitRef(text: string): AnikitRef | null {
  const t = text.trim();
  const internal = /^anikit:\/\/(item|folder)\/([^\s/?#]+)$/.exec(t);
  if (internal) return { kind: internal[1] as AnikitRef["kind"], id: internal[2] };
  const external = /^https?:\/\/(?:127\.0\.0\.1|localhost):\d+\/(?:open\/)?(item|folder)\/([^\s/?#]+)$/.exec(t);
  if (external) return { kind: external[1] as AnikitRef["kind"], id: external[2] };
  return null;
}

/** 跳转到 anikit 引用目标（media-open 事件 / links 输入框点击 / 内部 MD 渲染态点击共用）：
 *  item → origin Placement 所在文件夹视口 + 选中卡片 + 滚进视野；folder → 文件夹视口 + 选中档案域 */
export function jumpToAnikitRef(ref: AnikitRef): void {
  const content = useContentStore();
  const toast = useToastStore();
  const tabs = useTabsStore();
  const selection = useSelectionStore();
  if (ref.kind === "item") {
    const p =
      content.placements.find((pl) => pl.assetId === ref.id && pl.origin) ??
      content.placements.find((pl) => pl.assetId === ref.id);
    if (!p) {
      toast.notify({ message: "目标文件不在本库任何文件夹中（可能在回收站）", icon: "fail" });
      return;
    }
    jumpToAssetPlacement(p.assetId, p.folderId);
    return;
  }
  tabs.openView(`folder:${ref.id}`);
  // openView 触发 sidebarSelected watch 清选择——nextTick 让过再入域（FolderTree 同口径）
  void nextTick(() => selection.clickFolder(ref.id));
}

/** 媒体服务 /open 端点点击跳转（2026-10-08 S1）：后端 emit media-open → 定位目标（主窗口前置由后端做） */
let mediaOpenInited = false;
export function initMediaOpenListener(): void {
  if (mediaOpenInited) return;
  mediaOpenInited = true;
  void listen<AnikitRef>("media-open", (e) => jumpToAnikitRef(e.payload));
}
