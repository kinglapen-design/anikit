// 主窗口选择模式（6.2 一轮，PRD 03_数据链接 04：资产选择器 = 主窗口就地选择，不弹窗）：
// 四重标识 = 窗口整圈品牌色边框 + 顶部目的横幅 + 右侧详情栏临时替换备选区 + 已选卡片压暗大勾；
// 交互变化 = 单击卡片加入/移出备选（不再是选中）、双击仍预览（返回后模式保持）、右键仅查看类、
// 拖拽禁用、树仅导航；Esc / 取消 = 全部放弃，确认 = 退出模式回到触发处按用途执行。
// 本轮用途 = 关系（单选：备选恒一条选新换旧）；备选区两模板 = relation（关系容器）/ file（文件清单，
// 多选累积——文件选择用途入口随后续轮接入，模板与多选模型框架就位）。
import { defineStore } from "pinia";
import { nextTick } from "vue";
import { useContentStore } from "./content";
import { useSelectionStore } from "./selection";
import { useToastStore } from "./toast";
import { useUiStore } from "./ui";
import { addRelation, updateRelation, type PickerTarget } from "../lib/relationOps";

export type PickerPurpose = "relation-add" | "relation-retarget";
export type PickerTemplate = "file" | "relation";

export const usePickerStore = defineStore("picker", {
  state: () => ({
    active: false,
    purpose: "relation-add" as PickerPurpose,
    template: "relation" as PickerTemplate,
    multiple: false, // 关系 = 单选（PRD：备选区恒一条，选新换旧）
    origin: null as PickerTarget | null, // 发起文件/档案（from）
    picked: [] as PickerTarget[], // 备选清单（单选 = 至多一条）
    label: "", // 关系左框：选中文件 对 发起文件 的称谓
    reverseLabel: "", // 关系右框：发起文件 对 选中文件 的称谓（手动填写，可空）
    editingRelationId: null as string | null, // 换绑用途的关系 id
    retargetSide: "to" as "from" | "to", // 换绑被换端（详情目标在 from 侧 → 换 to；在 to 侧 → 换 from）
    confirming: false, // 确认在途防重入（连点确认 = 重复建关系，2026-08-23 探针实证）
  }),
  getters: {
    // 确认可用：关系模板 = 已选 + 左框称谓非空；文件模板 = 已选（多选带数量文案）
    canConfirm(state): boolean {
      if (state.picked.length === 0) return false;
      if (state.template === "relation") return state.label.trim() !== "";
      return true;
    },
    // 顶部横幅文案（目的 + 发起名；PRD 例："选择文件：为 雪牧 添加关系"，稿格式带"选择文件/档案"）
    bannerText(state): string {
      const name = state.origin?.name ?? "";
      return state.purpose === "relation-retarget"
        ? `选择文件/档案 - 为 ${name} 更换关系对象`
        : `选择文件/档案 - 为 ${name} 添加关系`;
    },
  },
  actions: {
    /** 进入关系选择模式：relationId 给了 = 换绑用途（详情关系 item 点文件二次修改，预填现状；
     *  side = 被换端——当前详情目标在 from 侧则对方是 to（换 to），反之换 from） */
    enterRelation(
      origin: PickerTarget,
      opts?: { relationId?: string; side?: "from" | "to"; picked?: PickerTarget; label?: string; reverseLabel?: string },
    ): void {
      this.active = true;
      this.purpose = opts?.relationId ? "relation-retarget" : "relation-add";
      this.template = "relation";
      this.multiple = false;
      this.origin = origin;
      this.picked = opts?.picked ? [opts.picked] : [];
      this.label = opts?.label ?? "";
      this.reverseLabel = opts?.reverseLabel ?? "";
      this.editingRelationId = opts?.relationId ?? null;
      this.retargetSide = opts?.side ?? "to";
      // 进入即清全局选择：单击语义已变为加入/移出备选，残留选中只会误导
      useSelectionStore().clear();
      // 备选区在右侧栏：收起态进入选择模式时强制展开（否则四重标识缺一，用户无处确认）
      const ui = useUiStore();
      if (ui.rightCollapsed) ui.rightCollapsed = false;
    },

    /** 退出模式（取消 / Esc / 确认成功后）：全部状态清场 + 回到触发处（PRD 03_数据链接 04——
     *  恢复发起目标选中：asset 按 key 直选（placements 全量，视口不含也出详情）、archive 入文件夹
     *  选择域；nextTick 让过 App.vue sidebarSelected watch 的清选择，同 FolderTree 入域口径） */
    exit(): void {
      const origin = this.origin;
      this.active = false;
      this.origin = null;
      this.picked = [];
      this.label = "";
      this.reverseLabel = "";
      this.editingRelationId = null;
      this.retargetSide = "to";
      if (origin) {
        void nextTick(() => {
          const selection = useSelectionStore();
          if (origin.kind === "archive") selection.clickFolder(origin.id);
          else selection.selectOnly(`${origin.id}|${origin.folderId ?? "root"}`);
        });
      }
    },

    /** 单击卡片 = 加入/移出备选：单选选新换旧；发起文件自身不可选（后端同拦） */
    togglePick(target: PickerTarget): void {
      if (this.origin && this.origin.kind === target.kind && this.origin.id === target.id) return;
      const i = this.picked.findIndex((p) => p.kind === target.kind && p.id === target.id);
      if (i >= 0) {
        this.picked.splice(i, 1);
        return;
      }
      if (this.multiple) this.picked.push(target);
      else this.picked = [target];
    },

    isPicked(kind: "asset" | "archive", id: string): boolean {
      return this.picked.some((p) => p.kind === kind && p.id === id);
    },

    /** 确认 = 按用途执行后退出：添加关系（就地插入清单）/ 换绑更新（重拉对齐快照）。
     *  confirming 防重入：连点确认钮只执行一次（连点 = 重复建关系，探针实证风险） */
    async confirm(): Promise<void> {
      if (!this.canConfirm || !this.origin || this.confirming) return;
      const content = useContentStore();
      const toast = useToastStore();
      const picked = this.picked[0];
      this.confirming = true;
      try {
        if (this.purpose === "relation-add") {
          const rel = await addRelation(this.origin, picked, this.label.trim(), this.reverseLabel.trim());
          // 幂等插入：刷新交错 / 重入下按 id 查重（同一条不重复进清单）
          if (!content.relations.some((r) => r.id === rel.id)) content.relations.push(rel);
          toast.notify({ message: `已添加关系：${rel.to.name} 是 ${rel.from.name} 的 ${rel.label}` });
        } else if (this.editingRelationId) {
          await updateRelation(
            this.editingRelationId,
            this.label.trim(),
            this.reverseLabel.trim(),
            this.retargetSide === "to" ? picked : undefined,
            this.retargetSide === "from" ? picked : undefined,
          );
          await content.refreshRelations();
          toast.notify({ message: "已更新关系" });
        }
        this.exit();
      } catch (e) {
        toast.notify({ message: String(e), icon: "fail" });
      } finally {
        this.confirming = false;
      }
    },
  },
});
