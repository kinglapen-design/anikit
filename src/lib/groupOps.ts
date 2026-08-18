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
