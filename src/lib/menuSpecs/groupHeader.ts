// 分组标题菜单（12 稿，3.5 二轮）：分组标题 group-menu 按钮触发。
// 一级（150 宽）：跳转▸（二级 = 全部分组列表，点击滚动跳转，滚动条联动）/ 新建分组（34 稿弹窗）/
// 编辑分组（33 稿管理弹窗）/ 展开折叠▸（二级：当前分组 / 全部分组 / 排除当前分组展开全部）/
// 上移 / 下移 / 删除。
// 未分组（组 id null）简化：仅 跳转▸ / 展开折叠▸（未分组不是真组——无规则可编辑、不参与组序）。
// 上移 / 下移 / 删除 = 即时生效（set_folder_groups 单撤销步；删除组不删文件，内容归未分组——
// 分组只是显示层归组，无确认）。
import { useContentStore } from "../../stores/content";
import { useUiStore } from "../../stores/ui";
import type { CtxEntry } from "../contextMenus";
import type { GroupDef } from "../grouping";
import { setFolderGroups } from "../groupOps";

export interface GroupMenuCtx {
  folderId: string;
  groupId: string | null; // 当前组；null = 未分组（简化菜单）
  /** 跳转：滚动到目标组标题（AssetGrid 注入 scrollTo） */
  scrollToGroup: (groupId: string | null) => void;
}

export function groupHeaderSpec(ctx: GroupMenuCtx): CtxEntry[] {
  const content = useContentStore();
  const ui = useUiStore();
  const defs: GroupDef[] = content.folderGroupsFor(`folder:${ctx.folderId}`) ?? [];
  const collapsedSet = (): (string | null)[] => {
    // 全部组 id（定义序）+ 未分组（内容区未分组恒在头）
    return [null, ...defs.map((g) => g.id)];
  };

  const jumpItem: CtxEntry = {
    label: "跳转",
    // 稿无独立图标（二级箭头由 submenu 自动出）——"稿无图标不配"口径
    submenuWidth: 180,
    submenu: collapsedSet().map((id) => ({
      label: id === null ? "未分组" : (defs.find((g) => g.id === id)?.name ?? "未命名分组"),
      action: () => ctx.scrollToGroup(id),
    })),
  };
  const foldItem: CtxEntry = {
    label: "展开/折叠",
    submenuWidth: 180,
    submenu: [
      {
        label: "当前分组",
        action: () => content.toggleGroupCollapsed(ctx.folderId, ctx.groupId),
      },
      {
        label: "全部分组",
        action: () => {
          // 有折叠则全展开，否则全折叠（toggle 语义）
          const cur = content.groupCollapsed[ctx.folderId] ?? [];
          if (cur.length > 0) content.expandAllGroups(ctx.folderId);
          else content.expandGroupsExcept(ctx.folderId, "__none__", collapsedSet());
        },
      },
      {
        label: "排除当前分组展开全部",
        action: () => content.expandGroupsExcept(ctx.folderId, ctx.groupId, collapsedSet()),
      },
    ],
  };
  // 未分组简化菜单：跳转 / 展开折叠
  if (ctx.groupId === null) return [jumpItem, foldItem];

  const gid = ctx.groupId;
  const index = defs.findIndex((g) => g.id === gid);
  const commit = (next: GroupDef[]): void => {
    void setFolderGroups(ctx.folderId, next).catch(() => {});
  };
  return [
    jumpItem,
    {
      label: "新建分组",
      action: () => {
        ui.groupDialogFolder = ctx.folderId;
        ui.groupEditOpen = true;
      },
    },
    {
      label: "编辑分组",
      action: () => {
        ui.groupDialogFolder = ctx.folderId;
        ui.groupManageOpen = true;
      },
    },
    foldItem,
    {
      label: "上移",
      trailingIcon: "chevron-small",
      disabled: index <= 0,
      action: () => {
        if (index <= 0) return;
        const next = [...defs];
        [next[index - 1], next[index]] = [next[index], next[index - 1]];
        commit(next);
      },
    },
    {
      label: "下移",
      trailingIcon: "chevron-small",
      trailingIconClass: "rot-180",
      disabled: index >= defs.length - 1,
      action: () => {
        if (index >= defs.length - 1) return;
        const next = [...defs];
        [next[index], next[index + 1]] = [next[index + 1], next[index]];
        commit(next);
      },
    },
    {
      label: "删除",
      action: () => commit(defs.filter((g) => g.id !== gid)),
    },
  ];
}
