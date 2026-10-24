// MD 打包导出（2026-10-08 S6，PRD 02-09——AE 收集打包的 MD 版）：解析文档内 anikit 双形态引用
// → 收集图片导出 {文档名}-打包/assets/ + 引用改写相对路径。库内 md = 目录选择器选落点；
// ws md = 默认文档旁，可由偏好设置的全局默认导出目录覆盖。

import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../stores/toast";
import { useUiStore } from "../stores/ui";

export interface BundleReport {
  outDir: string; // 产物目录（{文档名}-打包/）
  images: number; // 导出图片张数（去重后）
  docFile: string; // 改写后的 md 副本文件名
  skipped: string[]; // 跳过清单（人类可读原因）
}

/** 库内 md 资产打包导出：目录选择器选落点（取消 = 无操作） */
export async function exportMdBundleAsset(assetId: string): Promise<void> {
  const dest = await open({ directory: true, title: "选择打包导出位置" });
  if (!dest) return;
  await run(() => invoke<BundleReport>("export_md_bundle", { assetId, destDir: dest }));
}

/** 工作空间 md 打包导出：有默认目录用默认目录，空配置回退文档旁 */
export async function exportMdBundleWs(absPath: string): Promise<void> {
  const destDir = useUiStore().mdBundleExportDir.trim();
  await run(() => invoke<BundleReport>("ws_export_md_bundle", { absPath, destDir: destDir || null }));
}

async function run(action: () => Promise<BundleReport>): Promise<void> {
  const toast = useToastStore();
  try {
    const r = await action();
    const skip = r.skipped.length > 0 ? `，跳过 ${r.skipped.length} 个引用` : "";
    const message = r.images === 0 && r.skipped.length === 0 ? "文档中没有 anikit 引用（已生成文档副本）" : `已打包 ${r.images} 张图片${skip}`;
    toast.notify({ message, icon: "success", duration: 4000 });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}
