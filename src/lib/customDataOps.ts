// 自定义数据操作（6.1，PRD 03_数据链接 02/06）：文件与档案的属性（打词）/ 描述编辑、
// 属性池（32 稿「已有属性标签」）、全局删除属性（池内 ×，二次确认后）、档案封面设置（43 稿）。
// 与 placementOps 的分工：不动 placements，不做 content.refresh() 全量——档案卡片徽标 / 封面
// 刷 refreshArchives；详情面板（文件 / 档案两侧）统一 bumpDetailRev 信号重拉。
// 写命令全部已接撤销（后端 commands::custom_data / commands::archive），toast 带「撤销」动作。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type ArchiveBadge } from "../stores/content";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";

export interface DescItemInput {
  title: string;
  content: string;
}

/** 取景框参数（43 稿：滑块缩放 + 拖动位移；与后端 bake_cover 同一约定） */
export interface CropInput {
  x: number; // 图片中心相对框中心的偏移（框宽分数，右正）
  y: number; // 框高分数，下正
  scale: number; // ≥1，1 = 图片 cover 铺满框
}

// 统一收尾：refreshArchives（档案卡片）+ bumpDetailRev（详情面板）+ undo 态 + toast
async function run(label: () => string, action: () => Promise<unknown>): Promise<boolean> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    await action();
    await content.refreshArchives();
    content.bumpDetailRev();
    void undo.refreshState();
    toast.notify({
      message: `已${label()}`,
      actionLabel: "撤销",
      onAction: () => void undo.undo(),
    });
    return true;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return false;
  }
}

/** 编辑文件属性（tags 语义 = 属性词） */
export async function setAssetProps(assetId: string, props: string[]): Promise<boolean> {
  return await run(() => "编辑属性", () => invoke("set_asset_props", { assetId, props }));
}

/** 编辑文件描述 */
export async function setAssetDescriptions(assetId: string, descriptions: DescItemInput[]): Promise<boolean> {
  return await run(() => "编辑描述", () => invoke("set_asset_descriptions", { assetId, descriptions }));
}

/** 编辑档案属性 */
export async function setArchiveProps(folderId: string, props: string[]): Promise<boolean> {
  return await run(() => "编辑属性", () => invoke("set_archive_props", { folderId, props }));
}

/** 编辑档案描述 */
export async function setArchiveDescriptions(folderId: string, descriptions: DescItemInput[]): Promise<boolean> {
  return await run(() => "编辑描述", () => invoke("set_archive_descriptions", { folderId, descriptions }));
}

/** 属性池（32 稿弹窗打开时拉取；失败静默降级空池，不阻断弹窗） */
export async function listPropPool(): Promise<string[]> {
  try {
    return await invoke<string[]>("list_prop_pool");
  } catch (e) {
    console.warn("属性池拉取失败（静默降级）：", e);
    return [];
  }
}

/** 全局删除属性（池内 ×，警告二次确认后）：全库命中处移除，toast 报影响处数 */
export async function deletePropsGlobally(words: string[]): Promise<boolean> {
  let affected = 0;
  return await run(
    () => `删除属性（${affected} 处失效）`,
    async () => {
      affected = await invoke<number>("delete_props_globally", { words });
    },
  );
}

/** 档案封面设置（43 稿确认）：badge 三角——undefined 不动 / null 清除 / 对象设置；
 *  cover 给了 = 本地选图按取景参数烘培新封面（undefined 不动） */
export async function setArchiveCoverSettings(
  folderId: string,
  badge: ArchiveBadge | null | undefined,
  cover: { sourcePath: string; crop: CropInput } | undefined,
): Promise<boolean> {
  return await run(
    () => "设置档案封面",
    () => invoke("set_archive_cover_settings", { folderId, badge, cover }),
  );
}
