// 操作通知条（6.0 地基，样式参照 Eagle 通知条截图）：中间面板工具栏下方浮层（不挡工具栏），
// 成功反馈 + 可选动作按钮（撤销 / 重做），到时自动消失；多条竖向堆叠。
import { defineStore } from "pinia";

export interface ToastItem {
  id: number;
  message: string;
  icon: "success" | "info"; // success = --success 绿圆底白勾；info 无图标（纯文字反馈）
  actionLabel?: string; // 动作按钮文字（如 "撤销" / "重做"）；无则不显示按钮
  onAction?: () => void; // 点动作按钮执行（执行后通知关闭）
  duration: number; // 自动消失时长（ms）
}

const DEFAULT_DURATION = 5000;

let nextId = 1;
// 自动消失计时器：close / act 时清理，防关闭后计时器空转
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const useToastStore = defineStore("toast", {
  state: () => ({
    items: [] as ToastItem[],
  }),
  actions: {
    notify(opts: {
      message: string;
      icon?: "success" | "info";
      actionLabel?: string;
      onAction?: () => void;
      duration?: number;
    }): number {
      const id = nextId++;
      this.items.push({
        id,
        message: opts.message,
        icon: opts.icon ?? "success",
        actionLabel: opts.actionLabel,
        onAction: opts.onAction,
        duration: opts.duration ?? DEFAULT_DURATION,
      });
      timers.set(
        id,
        setTimeout(() => this.close(id), opts.duration ?? DEFAULT_DURATION),
      );
      return id;
    },
    close(id: number): void {
      const timer = timers.get(id);
      if (timer) {
        clearTimeout(timer);
        timers.delete(id);
      }
      this.items = this.items.filter((t) => t.id !== id);
    },
    // 点动作按钮：执行回调并关闭本条
    act(id: number): void {
      const item = this.items.find((t) => t.id === id);
      this.close(id);
      item?.onAction?.();
    },
  },
});
