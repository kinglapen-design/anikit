// 文件大小格式化（B / KB / MB 1 位小数 / GB 2 位小数）：
// 自 DuplicateRow.vue 局部函数提取（2.7 扫描重复文件共用，2026-08-13）。
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}
