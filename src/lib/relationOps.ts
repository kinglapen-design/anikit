// 关系（6.2 选择模式一轮，PRD 03_数据链接 02/06）：文件与档案之间的关系。
// 数据 = 空间级 relations.json（可跨库，ref 携库名 + 显示名快照）；命令封装 + 显示侧纯函数。
// 方向语义：label = to 对 from 的称谓（雪牧.from 发起选雪天.to 输入"父亲" → 雪牧详情显示"雪天是父亲"）；
// reverseLabel = from 对 to 的称谓（选择模式备选区右框手动填写，2026-08-23 用户定：不做词典推导），
// 空 = 未填写，反向显示回退 label。本轮 kind 恒 free；voice 音色随链接音频轮接入。
import { invoke } from "@tauri-apps/api/core";

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

/** 建立关系（选择模式备选区确认）；返回完整记录（含 id 与快照，前端就地插入） */
export async function addRelation(
  from: PickerTarget,
  to: PickerTarget,
  label: string,
  reverseLabel: string,
): Promise<Relation> {
  return invoke<Relation>("add_relation", {
    fromKind: from.kind,
    fromId: from.id,
    toKind: to.kind,
    toId: to.id,
    label,
    reverseLabel: reverseLabel || null,
  });
}

/** 编辑关系：改称谓；newTo / newFrom 二选一 = 换绑对方（详情关系 item 点文件进选择模式二次修改） */
export async function updateRelation(
  id: string,
  label: string,
  reverseLabel: string,
  newTo?: PickerTarget,
  newFrom?: PickerTarget,
): Promise<void> {
  await invoke("update_relation", {
    id,
    label,
    reverseLabel: reverseLabel || null,
    newToKind: newTo?.kind ?? null,
    newToId: newTo?.id ?? null,
    newFromKind: newFrom?.kind ?? null,
    newFromId: newFrom?.id ?? null,
  });
}

/** 删除关系（详情关系 item ×） */
export async function removeRelation(id: string): Promise<void> {
  await invoke("remove_relation", { id });
}
