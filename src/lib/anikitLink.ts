// anikit 链接工具（2026-10-08 双形态分工，PRD 02-09）：anikit://item|folder/{id} = 内部标识符，
// http://127.0.0.1:{port}/(open/)item|folder/{id} = 对外可点击形态——渲染 / 粘贴 / 跳转 / 打包共用
// 同一归一化解析器；media-open 事件 = 媒体服务 /open 端点点击跳转（S1，server/media.rs）

import { nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";

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

/** anikit 图片缩略图 URL（2026-10-08 S3；2026-10-10 自 lib/markdown 提为共享——md 渲染态与 CM6 Live Preview 图片 widget 共用）：
 *  缩略图 convertFileSrc + ?v=modifiedAt 版本戳（AssetCard 同口径）；资产不在库 / 无缩略图 = null 落占位文本 */
export function anikitThumbUrl(assetId: string): string | null {
  const p = useContentStore().placements.find((x) => x.assetId === assetId);
  if (!p?.thumbPath) return null;
  return `${convertFileSrc(p.thumbPath)}?v=${encodeURIComponent(p.modifiedAt ?? "")}`;
}

/** 跳转到 anikit 引用目标（media-open 事件 / links 输入框点击 / 内部 MD 渲染态点击共用）：
 *  **新起标签页定位**（2026-10-08 用户定：任何地方的跳转点击都不打断当前视口、不拉浏览器）——
 *  已有同 viewKey 标签页激活复用（防频繁点击堆积），没有才新开；
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
    const viewKey = p.folderId === "root" ? "quick:uncategorized" : `folder:${p.folderId}`;
    tabs.openInFreshTab(viewKey);
    content.requestReveal(viewKey, `${p.assetId}|${p.folderId}`);
    void nextTick(() => selection.selectOnly(`${p.assetId}|${p.folderId}`));
    return;
  }
  tabs.openInFreshTab(`folder:${ref.id}`);
  // openInNewTab 同口径：新建页触发 sidebarSelected watch 清选择——nextTick 让过再入域
  void nextTick(() => selection.clickFolder(ref.id));
}

/** 媒体服务 /open 端点与 deep-link 点击跳转（2026-10-08 S1 + deep-link 复活）：
 *  后端 emit media-open → 新开标签页定位目标（主窗口前置由后端做）；
 *  冷启动 = 协议链接拉起进程：后端暂存 pending，此处 take 消费——库未就绪（启动开库在途）
 *  等 placements 首批到达再跳（防「目标不在本库」误报） */
let mediaOpenInited = false;
export function initMediaOpenListener(): void {
  if (mediaOpenInited) return;
  mediaOpenInited = true;
  void listen<AnikitRef>("media-open", (e) => jumpToAnikitRef(e.payload));
  void invoke<[string, string] | null>("take_pending_deep_link").then((p) => {
    if (!p) return;
    const ref: AnikitRef = { kind: p[0] as AnikitRef["kind"], id: p[1] };
    if (useContentStore().placements.length > 0) {
      jumpToAnikitRef(ref);
      return;
    }
    const stop = watch(
      () => useContentStore().placements.length,
      (n) => {
        if (n > 0) {
          stop();
          jumpToAnikitRef(ref);
        }
      },
    );
  });
}
