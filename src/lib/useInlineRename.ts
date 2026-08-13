// 就地重命名 composable（6.1 C2/C3，自 AssetCard / ListRowCard / FolderCard / TreeItem 四份
// 逐字重复收编）：active 转真时回填初值 + 挂载聚焦选区（selectStem = 选中词干，缺省全选）；
// Enter / 失焦恒提交，Esc 由 escSaves 决定保存或取消（素材卡片 Esc 取消 / 文件夹与树 Esc 保存）。
// 提交前的"空或不变不动"与实际改名调用在 onCommit 内（各调用方业务不同）；commit/cancel 自带
// active 护栏——Enter 提交后 blur 晚到 / 滚出虚拟滚动先清状态再 blur 晚到的双触发在此挡掉。
// InboxDiskSection 2026-08-13 接入（原 blur 取消离群语义废除，统一 blur 提交）。
import { nextTick, ref, watch, type Ref } from "vue";

export function useInlineRename(opts: {
  active: () => boolean; // 命名态开关（转真时回填初值 + 聚焦选区；提交/取消前作护栏复判）
  initial: () => string; // 初值（当前名）
  selectStem?: boolean; // true = 选中词干（素材名，扩展名不入选区）；缺省 = 全选
  escSaves?: boolean; // true = Esc 同 Enter 保存；缺省 = Esc 取消（调 onCancel）
  onCommit: (name: string) => void | Promise<void>; // 入参已 trim；空或不变的判定归调用方
  onCancel?: () => void;
}): {
  draft: Ref<string>;
  bindInput: (el: unknown) => void; // 模板 :ref 绑定（函数 ref：挂载回填 / 卸载清空 inputRef）
  commit: () => void;
  onEsc: () => void;
} {
  const draft = ref("");
  const inputRef = ref<HTMLInputElement>();

  function bindInput(el: unknown): void {
    inputRef.value = (el as HTMLInputElement | null) ?? undefined;
  }

  watch(
    () => opts.active(),
    (a) => {
      if (!a) return;
      draft.value = opts.initial();
      void nextTick(() => {
        const el = inputRef.value;
        if (!el) return;
        el.focus();
        if (opts.selectStem) {
          const name = opts.initial();
          const dot = name.lastIndexOf(".");
          el.setSelectionRange(0, dot > 0 ? dot : name.length); // 默认选中词干
        } else {
          el.select();
        }
      });
    },
    { immediate: true },
  );

  function commit(): void {
    if (!opts.active()) return; // 已退出命名态（Enter 提交后 blur 晚到 / 滚出视口先清状态）
    void opts.onCommit(draft.value.trim());
  }
  function cancel(): void {
    if (!opts.active()) return;
    opts.onCancel?.();
  }

  return { draft, bindInput, commit, onEsc: opts.escSaves ? commit : cancel };
}
