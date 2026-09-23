// 关系（6.2 选择模式一轮，PRD 03_数据链接 02/06）：文件与档案之间的关系。
// 数据 = 空间级 relations.json（可跨库，ref 携库名 + 显示名快照）；命令封装 + 显示侧纯函数。
// 方向语义：label = to 对 from 的称谓（雪牧.from 发起选雪天.to 输入"父亲" → 雪牧详情显示"雪天是父亲"）；
// reverseLabel = from 对 to 的称谓（选择模式备选区右框手动填写，2026-08-23 用户定：不做词典推导），
// 空 = 未填写，反向显示回退 label。本轮 kind 恒 free；voice 音色随链接音频轮接入。
import { invoke } from "@tauri-apps/api/core";
import { nextTick } from "vue";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { currentLibrary } from "./libraryOps";

/** 关系端点（后端 RelationRef 同形，camelCase）：lib = 库名；kind = asset/archive；name = 显示名快照 */
export interface RelationRef {
  lib: string;
  kind: "asset" | "archive";
  id: string;
  name: string;
}

/** 一条关系（后端 Relation 同形） */
export interface Relation {
  id: string;
  from: RelationRef;
  to: RelationRef;
  kind: string; // free（本轮）；voice 预留
  label: string;
  reverseLabel: string | null;
}

/** 选择模式目标（发起文件 / 选中文件）：kind + id 定位，name 供备选区显示；
 *  ext / thumb = 备选区 item 左标（文件 = 15px 缩略图、无图回退扩展名文字；档案 = 封面小图 / archive 图标）；
 *  folderId = 发起文件的 Placement 所在文件夹（前端退出模式恢复选中用，不进命令入参） */
export interface PickerTarget {
  kind: "asset" | "archive";
  id: string;
  name: string;
  ext?: string;
  thumb?: string | null;
  folderId?: string;
  /** 文件模板链接状态徽章：false = 链接 Placement（origin 缺省视同原始文件不显示徽章） */
  origin?: boolean;
}

/** 目标视角的关系视图一条：other = 对方端点，label = 对方对当前目标的称谓（"对方是我的 label"）；
 *  role = 当前目标在记录里的端位（编辑称谓 / 换绑对方时定位写字段与被换端用） */
export interface RelationView {
  relation: Relation;
  other: RelationRef;
  label: string;
  role: "from" | "to";
}

/** 目标的关系清单（显示侧）：T == from → 对方 = to、称谓 = label；
 *  T == to → 对方 = from、称谓 = reverseLabel ?? label（反向未填回退同词）。
 *  匹配携库名（关系可跨库，同库 id 唯一——跨库端点不会等于当前目标） */
export function relationsForTarget(
  relations: Relation[],
  currentLib: string,
  target: { kind: "asset" | "archive"; id: string },
): RelationView[] {
  const out: RelationView[] = [];
  for (const r of relations) {
    if (r.from.lib === currentLib && r.from.kind === target.kind && r.from.id === target.id) {
      out.push({ relation: r, other: r.to, label: r.label, role: "from" });
    } else if (r.to.lib === currentLib && r.to.kind === target.kind && r.to.id === target.id) {
      out.push({ relation: r, other: r.from, label: r.reverseLabel ?? r.label, role: "to" });
    }
  }
  return out;
}

/** 备选区一组（多选累积，2026-08-24 用户定）：选中文件 + 双框称谓（左 = to 对 from，右 = from 对 to 可空）；
 *  name = 文件模板行名称草稿（link 用途确认按行写入落点文件夹，2026-08-24 用户定"稿的名称框本身就是输入框"） */
export interface PickedRelation {
  target: PickerTarget;
  label: string;
  reverseLabel: string;
  name?: string;
}

/** 批量建立关系（选择模式确认；任一失败全不写，单撤销步）；返回完整记录组（前端就地插入）。
 *  kind 缺省 free；voice = 链接音频（无称谓——label 恒传空，后端校验 to 必须音频 asset） */
export async function addRelations(from: PickerTarget, items: PickedRelation[], kind?: "voice"): Promise<Relation[]> {
  return invoke<Relation[]>("add_relations", {
    fromKind: from.kind,
    fromId: from.id,
    kind: kind ?? null,
    items: items.map((i) => ({
      toKind: i.target.kind,
      toId: i.target.id,
      label: kind === "voice" ? "" : i.label.trim(),
      reverseLabel: kind === "voice" ? null : i.reverseLabel.trim() || null,
    })),
  });
}

/** 编辑关系称谓（详情关系 item 点称谓框二次修改；换绑 2026-08-24 用户定废除——要换就重新添加） */
export async function updateRelation(id: string, label: string, reverseLabel: string): Promise<void> {
  await invoke("update_relation", {
    id,
    label,
    reverseLabel: reverseLabel || null,
  });
}

/** 删除关系（详情关系 item ×） */
export async function removeRelation(id: string): Promise<void> {
  await invoke("remove_relation", { id });
}

/** 跳转到指定位置（6.2 四轮走查②④⑤：链接区块 item / 打开文件链接所在位置 / 徽章悬停链接列表
 *  共用）：目标文件夹视口 + 选中该卡片 + 滚进视野（2026-09-23 用户定：跳转不再停顶端——
 *  pendingReveal 由目标网格重排时消费，同视口跳转由 pendingReveal watch 驱动重排）；
 *  openView 入面包屑历史（后退即返回） */
export function jumpToAssetPlacement(assetId: string, folderId: string): void {
  const tabs = useTabsStore();
  const selection = useSelectionStore();
  const content = useContentStore();
  const viewKey = folderId === "root" ? "quick:uncategorized" : `folder:${folderId}`;
  tabs.openView(viewKey);
  content.requestReveal(viewKey, `${assetId}|${folderId}`);
  void nextTick(() => selection.selectOnly(`${assetId}|${folderId}`));
}

/** 跳转定位（2026-08-24 用户定：详情数据 item 点文件卡 = 跳过去——"光显示在那是没用的"）：
 *  asset → origin Placement 所在文件夹视口 + 选中卡片；archive → 档案视口 + 选中档案卡；
 *  均经 tabs.openView 入面包屑历史（后退即返回）；跨库 / 目标失效给提示。
 *  关系 / 音频两区块共用（6.2 三轮由 SectionRelations 提公共） */
export function jumpToRelationTarget(other: RelationRef): void {
  const content = useContentStore();
  const toast = useToastStore();
  if (other.lib !== (currentLibrary.name ?? "")) {
    toast.notify({ message: `「${other.name}」在资源库 ${other.lib} 中（跨库跳转随后续轮）`, icon: "fail" });
    return;
  }
  const tabs = useTabsStore();
  const selection = useSelectionStore();
  if (other.kind === "archive") {
    tabs.openView(`folder:${other.id}`);
    // openView 触发 sidebarSelected watch 清选择——nextTick 让过再入域（FolderTree 同口径）
    void nextTick(() => selection.clickFolder(other.id));
    return;
  }
  const p =
    content.placements.find((pl) => pl.assetId === other.id && pl.origin) ??
    content.placements.find((pl) => pl.assetId === other.id);
  if (!p) {
    toast.notify({ message: "目标文件不在本库任何文件夹中（可能在回收站）", icon: "fail" });
    return;
  }
  const viewKey = p.folderId === "root" ? "quick:uncategorized" : `folder:${p.folderId}`;
  tabs.openView(viewKey);
  content.requestReveal(viewKey, `${p.assetId}|${p.folderId}`);
  void nextTick(() => selection.selectOnly(`${p.assetId}|${p.folderId}`));
}
