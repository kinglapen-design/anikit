// 分组操作（3.5 二轮）：set_folder_groups 命令封装（33 稿管理弹窗 / 34 稿新建 / 12 稿菜单统一入口）；
// 单撤销步「设置分组」；错误直抛（23 稿"完全相同"等校验文案由调用方弹窗展示）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useUndoStore } from "../stores/undo";
import type { GroupDef } from "./grouping";

/** 提交文件夹分组配置（空数组 = 不分组）；成功后全量刷新（分组改变内容区组织） */
export async function setFolderGroups(folderId: string, groups: GroupDef[]): Promise<void> {
  await invoke("set_folder_groups", { folderId, groups });
  await useContentStore().refresh();
  void useUndoStore().refreshState();
}

/** 自由组成员归组（2026-09-17 无规则空组）：移动至分组（groupId）/ 移出分组（null）——
 * 移动语义唯一归属（后端先摘除再加入）；成功后全量刷新（归组改变内容区组织） */
export async function assignGroup(folderId: string, groupId: string | null, assetIds: string[]): Promise<void> {
  if (assetIds.length === 0) return;
  await invoke("assign_group", { folderId, groupId, assetIds });
  await useContentStore().refresh();
  void useUndoStore().refreshState();
}
