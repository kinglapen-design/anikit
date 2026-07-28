// 撤销 / 重做（6.0 地基）：快捷键 Ctrl+Z / Ctrl+Shift+Z 与通知条动作按钮共用同一入口。
// 后端回滚数据文件快照 + 磁盘逆操作并重载内存索引；前端成功后全量刷新（盘上是事实来源），
// 结果经通知条反馈（已撤销 / 已重做，带反向动作按钮）。
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "./content";
import { useToastStore } from "./toast";

// 与后端 core/undo.rs UndoState 对应（camelCase）
export interface UndoState {
  canUndo: boolean;
  canRedo: boolean;
  undoLabel: string | null;
  redoLabel: string | null;
}

export const useUndoStore = defineStore("undo", {
  state: () => ({
    canUndo: false,
    canRedo: false,
    undoLabel: null as string | null,
    redoLabel: null as string | null,
  }),
  actions: {
    // 栈状态同步（菜单置灰用；6.1 右键菜单 / 编辑菜单接入）
    async refreshState(): Promise<void> {
      try {
        const s = await invoke<UndoState>("get_undo_state");
        this.canUndo = s.canUndo;
        this.canRedo = s.canRedo;
        this.undoLabel = s.undoLabel;
        this.redoLabel = s.redoLabel;
      } catch {
        // 未打开库等场景保持默认（双空）
      }
    },
    async undo(): Promise<void> {
      const toast = useToastStore();
      try {
        const label = await invoke<string>("undo");
        await useContentStore().refresh();
        void this.refreshState();
        toast.notify({
          message: `已撤销：${label}`,
          actionLabel: "重做",
          onAction: () => void this.redo(),
        });
      } catch (e) {
        toast.notify({ message: String(e), icon: "info" });
      }
    },
    async redo(): Promise<void> {
      const toast = useToastStore();
      try {
        const label = await invoke<string>("redo");
        await useContentStore().refresh();
        void this.refreshState();
        toast.notify({
          message: `已重做：${label}`,
          actionLabel: "撤销",
          onAction: () => void this.undo(),
        });
      } catch (e) {
        toast.notify({ message: String(e), icon: "info" });
      }
    },
  },
});
