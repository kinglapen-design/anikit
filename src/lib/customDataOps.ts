// 自定义数据操作（6.1，PRD 03_数据链接 02/06）：文件与档案的属性（打词）/ 描述编辑、
// 属性池（32 稿「已有属性标签」）、全局删除属性（池内 ×，二次确认后）、档案封面设置（43 稿）。
// 与 placementOps 的分工：不动 placements，不做 content.refresh() 全量——档案卡片徽标 / 封面
// 刷 refreshArchives；详情面板（文件 / 档案两侧）统一 bumpDetailRev 信号重拉。
// 写命令全部已接撤销（后端 commands::custom_data / commands::archive），toast 带「撤销」动作。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type ArchiveBadge } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
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

// —— 复制 / 粘贴标签·描述（2026-09-05 走查「上下文菜单」：复制数据组新增，粘贴 = 合并追加——
// 标签按词去重、描述按标题去重，用户定；多选批量同一语义）——
// 复制 = 写 selection.copiedData 会话槽（不写系统剪贴板——系统侧文本格式未定，APP 内流转场景为主）；
// 粘贴只读槽不清空（剪贴板心智：复制一次可贴多处）。

interface AssetDetailBrief {
  tags: string[];
  descriptions: DescItemInput[];
}

async function readAssetData(assetId: string): Promise<AssetDetailBrief | null> {
  try {
    const d = await invoke<AssetDetailBrief>("get_asset_detail", { assetId });
    return { tags: d.tags ?? [], descriptions: d.descriptions ?? [] };
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
    return null;
  }
}

/** 复制标签（文件）：detail 读属性词写入会话槽；空集提示不覆盖旧槽 */
export async function copyAssetTags(assetId: string): Promise<void> {
  const d = await readAssetData(assetId);
  if (!d) return;
  if (d.tags.length === 0) {
    useToastStore().notify({ message: "该文件没有标签", icon: "fail" });
    return;
  }
  useSelectionStore().copiedData.tags = [...d.tags];
  useToastStore().notify({ message: `已复制 ${d.tags.length} 个标签`, duration: 3000 });
}

/** 复制描述（文件） */
export async function copyAssetDescriptions(assetId: string): Promise<void> {
  const d = await readAssetData(assetId);
  if (!d) return;
  if (d.descriptions.length === 0) {
    useToastStore().notify({ message: "该文件没有描述", icon: "fail" });
    return;
  }
  useSelectionStore().copiedData.descriptions = d.descriptions.map((x) => ({ ...x }));
  useToastStore().notify({ message: `已复制 ${d.descriptions.length} 条描述`, duration: 3000 });
}

/** 复制标签（档案）：content.archives 直读，无详情命令往返 */
export function copyArchiveTags(folderId: string): void {
  const props = useContentStore().archives[folderId]?.props ?? [];
  if (props.length === 0) {
    useToastStore().notify({ message: "该档案没有标签", icon: "fail" });
    return;
  }
  useSelectionStore().copiedData.tags = [...props];
  useToastStore().notify({ message: `已复制 ${props.length} 个标签`, duration: 3000 });
}

/** 复制描述（档案） */
export function copyArchiveDescriptions(folderId: string): void {
  const list = useContentStore().archives[folderId]?.descriptions ?? [];
  if (list.length === 0) {
    useToastStore().notify({ message: "该档案没有描述", icon: "fail" });
    return;
  }
  useSelectionStore().copiedData.descriptions = list.map((x) => ({ ...x }));
  useToastStore().notify({ message: `已复制 ${list.length} 条描述`, duration: 3000 });
}

// 粘贴目标（多选批量 = 逐目标合并写入；同一菜单语境只会有一种，混合域也兜得住）
export interface PasteDataTargets {
  assets: string[];
  archives: string[];
}

// 批量粘贴统一收尾（不走 run——逐目标合并须先读后写，且只出一次 toast）：
// 逐目标 try，失败计数不中断；成功项合并写盘后一次 refreshArchives + bumpDetailRev。
// zeroMessage（2026-09-05 清空标签复用）：零目标提示文案按操作给（粘贴 = 没有可粘贴的目标 / 清空 = 所选项都没有标签）
async function pasteRun(label: () => string, write: () => Promise<number>, zeroMessage = "没有可粘贴的目标"): Promise<void> {
  const content = useContentStore();
  const toast = useToastStore();
  try {
    const done = await write();
    if (done === 0) {
      toast.notify({ message: zeroMessage, icon: "fail" });
      return;
    }
    await content.refreshArchives();
    content.bumpDetailRev();
    void useUndoStore().refreshState();
    toast.notify({ message: `已${label()}`, duration: 3000 });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

/** 粘贴标签（合并追加，按词去重）：返回写入目标数 */
export async function pasteTags(targets: PasteDataTargets): Promise<void> {
  const copied = useSelectionStore().copiedData.tags;
  if (!copied?.length) return;
  await pasteRun(
    () => `粘贴标签（${targets.assets.length + targets.archives.length} 个目标）`,
    async () => {
      let done = 0;
      for (const id of targets.assets) {
        const d = await readAssetData(id);
        if (!d) continue;
        const merged = [...d.tags, ...copied.filter((w) => !d.tags.includes(w))];
        if (merged.length === d.tags.length) {
          done++; // 全部已存在 = 幂等成功（合并语义下无变化也是"贴上了"）
          continue;
        }
        await invoke("set_asset_props", { assetId: id, props: merged });
        done++;
      }
      for (const id of targets.archives) {
        const cur = useContentStore().archives[id]?.props ?? [];
        const merged = [...cur, ...copied.filter((w) => !cur.includes(w))];
        if (merged.length !== cur.length) await invoke("set_archive_props", { folderId: id, props: merged });
        done++;
      }
      return done;
    },
  );
}

/** 粘贴描述（合并追加，按标题去重） */
export async function pasteDescriptions(targets: PasteDataTargets): Promise<void> {
  const copied = useSelectionStore().copiedData.descriptions;
  if (!copied?.length) return;
  await pasteRun(
    () => `粘贴描述（${targets.assets.length + targets.archives.length} 个目标）`,
    async () => {
      let done = 0;
      for (const id of targets.assets) {
        const d = await readAssetData(id);
        if (!d) continue;
        const add = copied.filter((x) => !d.descriptions.some((e) => e.title === x.title));
        if (add.length > 0) {
          await invoke("set_asset_descriptions", { assetId: id, descriptions: [...d.descriptions, ...add] });
        }
        done++;
      }
      for (const id of targets.archives) {
        const cur = useContentStore().archives[id]?.descriptions ?? [];
        const add = copied.filter((x) => !cur.some((e) => e.title === x.title));
        if (add.length > 0) {
          await invoke("set_archive_descriptions", { folderId: id, descriptions: [...cur, ...add] });
        }
        done++;
      }
      return done;
    },
  );
}

/** 清空标签（菜单栏主菜单「整理」，2026-09-05 用户定：所选项的标签全部删掉）——
 *  文件 + 档案多选批量：逐目标写空集（已空的幂等跳过不计数）；一次 toast 收尾（pasteRun 同口径；
 *  撤销 = 后端逐目标步，Ctrl+Z 逐步回滚） */
export async function clearTags(targets: PasteDataTargets): Promise<void> {
  await pasteRun(
    () => `清空标签（${targets.assets.length + targets.archives.length} 个目标）`,
    async () => {
      let done = 0;
      for (const id of targets.assets) {
        const d = await readAssetData(id);
        if (!d || d.tags.length === 0) continue; // 空集幂等跳过
        await invoke("set_asset_props", { assetId: id, props: [] });
        done++;
      }
      for (const id of targets.archives) {
        const cur = useContentStore().archives[id]?.props ?? [];
        if (cur.length === 0) continue;
        await invoke("set_archive_props", { folderId: id, props: [] });
        done++;
      }
      return done;
    },
    "所选项都没有标签",
  );
}
