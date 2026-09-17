// 工作空间右键菜单 spec（PRD 02 步骤 1 只读底座 + 2026-09-17 手动刷新兜底定案）：
// 空白处 = 「刷新」（重扫当前目录 + 补请求缺失缩略图——热刷新失手的兜底入口）；
// 条目 = 最小集（打开 / 在资源管理器中显示 / 刷新）——**不得出现任何库写操作项**
// （删除 / 移动 / 标签 / 副本等一律不挂，assetId = ws: 合成假 id 严禁发 assetId 类命令）。
import type { PlacementView } from "../../stores/content";
import type { CtxEntry } from "../contextMenus";
import { usePreviewStore } from "../../stores/preview";
import { useTabsStore } from "../../stores/tabs";
import { useWsStore } from "../../stores/ws";
import { revealPath } from "../wsOps";
import { useToastStore } from "../../stores/toast";

// 刷新当前目录（手动兜底）：全量重扫 + 补请求缺失缩略图（fs-changed 热刷新同径，见 stores/ws）
function refreshDir(rel: string): void {
  void useWsStore().refreshDir(rel);
}

// 视口空白处菜单（gridDispatch ws 分支调用）
export function wsBlankSpec(rel: string): CtxEntry[] {
  return [{ label: "刷新", action: () => refreshDir(rel) }];
}

// ws 文件夹菜单（FolderStrip ws 分流调用；打开 = 进该目录视口，与双击同径）
export function wsFolderSpec(rel: string, _name: string): CtxEntry[] {
  const toast = useToastStore();
  return [
    {
      label: "打开",
      action: () => void useTabsStore().openView(`ws:${rel}`),
    },
    {
      label: "在资源管理器中显示",
      action: () => {
        const abs = useWsStore().absPath(rel);
        if (!abs) return;
        void revealPath(abs).catch((e: unknown) =>
          toast.notify({ message: String(e), icon: "fail" }),
        );
      },
    },
    { label: "刷新", action: () => refreshDir(rel) },
  ];
}

// ws 条目菜单（打开 = 进预览，与双击卡片同径；所在目录 rel = folderId 去 "ws:" 前缀）
export function wsCardSpec(p: PlacementView): CtxEntry[] {
  const rel = p.folderId.slice(3);
  const toast = useToastStore();
  return [
    {
      label: "打开",
      action: () => void usePreviewStore().open(`${p.assetId}|${p.folderId}`),
    },
    {
      label: "在资源管理器中显示",
      action: () => {
        if (!p.wsPath) return;
        void revealPath(p.wsPath).catch((e: unknown) =>
          toast.notify({ message: String(e), icon: "fail" }),
        );
      },
    },
    { label: "刷新", action: () => refreshDir(rel) },
  ];
}
