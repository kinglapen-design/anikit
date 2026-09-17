// 工作空间右键菜单 spec（PRD 02 步骤 1 只读底座 + 步骤 2 基础整理 + 2026-09-17 手动刷新兜底）：
// 空白处 = 新建文件夹 / 粘贴（剪贴板非空）/ 刷新；文件夹 = 打开 / 新建文件夹 / 重命名 /
// 复制 / 剪切 / 删除（系统回收站，两段确认）/ 在资源管理器中显示 / 刷新；
// 条目 = 打开 / 重命名 / 复制 / 剪切 / 删除 / 在资源管理器中显示 / 刷新。
// 一律按路径走 ws 命令（assetId = ws: 合成假 id 严禁发 assetId 类命令）；
// 写操作失败 toast 报错，成功由 ws store 局部重扫（watcher 回声后端消重）。
// 重命名 = 行内命名态入口：卡片经 selection.renamingKey（AssetCard 接管提交），
// 文件夹经 selection.namingFolderCardId（FolderStrip 接管提交），与双击名称同径。
import type { PlacementView } from "../../stores/content";
import type { CtxEntry } from "../contextMenus";
import { usePreviewStore } from "../../stores/preview";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import { useWsStore } from "../../stores/ws";
import { revealPath } from "../wsOps";
import { useToastStore } from "../../stores/toast";

// 刷新当前目录（手动兜底）：全量重扫 + 补请求缺失缩略图（fs-changed 热刷新同径，见 stores/ws）
function refreshDir(rel: string): void {
  void useWsStore().refreshDir(rel);
}

// 删除两段确认（与库删除同款 confirmPrompt 警告黄钮；系统回收站可找回，文案明说）
function confirmDelete(rels: string[], name: string): void {
  useSelectionStore().confirmPrompt = {
    title: "删除",
    body: `「${name}」将被移到系统回收站，可从回收站找回。`,
    confirmLabel: "删除",
    kind: "warning",
    onConfirm: () =>
      void useWsStore()
        .removeEntries(rels)
        .catch((e: unknown) => useToastStore().notify({ message: String(e), icon: "fail" })),
  };
}

// 视口空白处菜单（gridDispatch ws 分支调用）
export function wsBlankSpec(rel: string): CtxEntry[] {
  const ws = useWsStore();
  const fail = (e: unknown) => useToastStore().notify({ message: String(e), icon: "fail" });
  const entries: CtxEntry[] = [
    { label: "新建文件夹", action: () => void ws.createFolder(rel).catch(fail) },
  ];
  if (ws.clip) {
    entries.push({
      label: ws.clip.cut ? "粘贴（移动）" : "粘贴",
      action: () => void ws.paste(rel).catch(fail),
    });
  }
  entries.push({ label: "刷新", action: () => refreshDir(rel) });
  return entries;
}

// ws 文件夹菜单（FolderStrip ws 分流调用；打开 = 进该目录视口，与双击同径；
// 重命名 = 置 FolderStrip 行内命名态，提交在 FolderStrip 的 ws 分支）
export function wsFolderSpec(rel: string, _name: string): CtxEntry[] {
  const toast = useToastStore();
  const ws = useWsStore();
  const fail = (e: unknown) => toast.notify({ message: String(e), icon: "fail" });
  return [
    { label: "打开", action: () => void useTabsStore().openView(`ws:${rel}`) },
    { label: "新建文件夹", action: () => void ws.createFolder(rel).catch(fail) },
    { label: "重命名", action: () => void (useSelectionStore().namingFolderCardId = rel) },
    { label: "复制", action: () => ws.setClip([rel], false) },
    { label: "剪切", action: () => ws.setClip([rel], true) },
    { label: "删除", action: () => confirmDelete([rel], _name) },
    {
      label: "在资源管理器中显示",
      action: () => {
        const abs = ws.absPath(rel);
        if (!abs) return;
        void revealPath(abs).catch(fail);
      },
    },
    { label: "刷新", action: () => refreshDir(rel) },
  ];
}

// ws 条目菜单（打开 = 进预览，与双击卡片同径；重命名 = 置卡片行内命名态，
// 提交在 AssetCard 的 ws 分支；所在目录 rel = folderId 去 "ws:" 前缀）
export function wsCardSpec(p: PlacementView): CtxEntry[] {
  const rel = p.folderId.slice(3);
  const entryRel = p.assetId.slice(3);
  const toast = useToastStore();
  const ws = useWsStore();
  const fail = (e: unknown) => toast.notify({ message: String(e), icon: "fail" });
  return [
    { label: "打开", action: () => void usePreviewStore().open(`${p.assetId}|${p.folderId}`) },
    { label: "重命名", action: () => void (useSelectionStore().renamingKey = `${p.assetId}|${p.folderId}`) },
    { label: "复制", action: () => ws.setClip([entryRel], false) },
    { label: "剪切", action: () => ws.setClip([entryRel], true) },
    { label: "删除", action: () => confirmDelete([entryRel], p.name) },
    {
      label: "在资源管理器中显示",
      action: () => {
        if (!p.wsPath) return;
        void revealPath(p.wsPath).catch(fail);
      },
    },
    { label: "刷新", action: () => refreshDir(rel) },
  ];
}
