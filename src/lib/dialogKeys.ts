import { onBeforeUnmount, onMounted } from "vue";

const closeStack: (() => void)[] = [];

export function useDialogEscape(close: () => void): void {
  function onKey(event: KeyboardEvent): void {
    if (event.key !== "Escape" || closeStack[closeStack.length - 1] !== close) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest("input, textarea, [contenteditable='true']")) return;
    event.stopImmediatePropagation();
    close();
  }
  onMounted(() => {
    closeStack.push(close);
    window.addEventListener("keydown", onKey, true);
  });
  onBeforeUnmount(() => {
    const index = closeStack.lastIndexOf(close);
    if (index >= 0) closeStack.splice(index, 1);
    window.removeEventListener("keydown", onKey, true);
  });
}
