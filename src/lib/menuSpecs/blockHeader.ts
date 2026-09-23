// 区块标题菜单（2026-09-23 接业务：LibraryView 区块头 more 钮，§20「点亮不接业务」收口）：
// 大纲管理 / 上移 / 下移 三项平铺（同 3.5 分组菜单无分隔口径）。
// 大纲管理 = 侧栏区块管理弹窗（30 稿 BlockManageDialog，ui.blockManageOpen——与视图菜单同项同径）；
// 上移 / 下移 = 即时生效（reorder_blocks，封装 reorderBlocks：乐观重排 + toast 带撤销，
// 与区块头拖拽排序同源）；trailingIcon 与边界禁用随 3.5 分组菜单「上移/下移」口径。
// 选择模式闸在调用方（LibraryView onBlockMenu 整体不弹——排序与管理弹窗均非 §12.1 白名单）。
import { useContentStore } from "../../stores/content";
import { useUiStore } from "../../stores/ui";
import { reorderBlocks } from "../blockOps";
import type { CtxEntry } from "../contextMenus";

export function blockHeaderSpec(blockId: string): CtxEntry[] {
  const ui = useUiStore();
  const blocks = useContentStore().blocks;
  const index = blocks.findIndex((b) => b.id === blockId);
  // reorder_blocks 契约 = ids 恰好覆盖全部区块（前端按当前 blocks 组装全量序，拖拽排序同径）
  const move = (dir: -1 | 1): void => {
    const j = index + dir;
    if (index < 0 || j < 0 || j >= blocks.length) return;
    const ids = blocks.map((b) => b.id);
    [ids[index], ids[j]] = [ids[j], ids[index]];
    void reorderBlocks(ids);
  };
  return [
    { label: "大纲管理", action: () => (ui.blockManageOpen = true) },
    {
      label: "上移",
      trailingIcon: "chevron-small",
      disabled: index <= 0,
      action: () => move(-1),
    },
    {
      label: "下移",
      trailingIcon: "chevron-small",
      trailingIconClass: "rot-180",
      disabled: index < 0 || index >= blocks.length - 1,
      action: () => move(1),
    },
  ];
}
