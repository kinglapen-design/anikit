// 主窗口选择模式（6.2，PRD 03_数据链接 04：资产选择器 = 主窗口就地选择，不弹窗）：
// 四重标识 = 窗口整圈品牌色边框 + 顶部目的横幅 + 右侧详情栏临时替换备选区 + 已选卡片压暗大勾；
// 交互变化（2026-08-24 用户收窄白名单）= 只放行：树 / 快速访问导航、搜索、查看方式、挑选文件
//（点选 + 双击预览）、面包屑后退前进；其余全禁（编辑类 / 汉堡 / 切库 / 预览内编辑等）。
// 单击卡片 = 加入/移出备选（不再是选中）；Esc / 取消 = 全部放弃，确认 = 按用途执行后退出回触发处。
// 关系 = **多选累积**（2026-08-24 用户定：选一个 = 追加一条而非切换——一次为发起方加
// 父亲/儿子/母亲/专武多条，每条自带双框称谓，确认批量写入单撤销步）。
import { defineStore } from "pinia";
import { nextTick } from "vue";
import { useContentStore } from "./content";
import { useSelectionStore } from "./selection";
import { useToastStore } from "./toast";
import { useUiStore } from "./ui";
import { addRelations, type PickerTarget, type PickedRelation } from "../lib/relationOps";

export type PickerTemplate = "file" | "relation";

export const usePickerStore = defineStore("picker", {
  state: () => ({
    active: false,
    template: "relation" as PickerTemplate,
    origin: null as PickerTarget | null, // 发起文件/档案（from）
    picked: [] as PickedRelation[], // 备选组（多选累积，每组带双框称谓；file 模板只用 target）
    confirming: false, // 确认在途防重入（连点确认 = 重复建关系，2026-08-23 探针实证）
  }),
  getters: {
    // 确认可用：关系模板 = 有备选 + 每组左框称谓非空；文件模板 = 有备选
    canConfirm(state): boolean {
      if (state.picked.length === 0) return false;
      if (state.template === "relation") return state.picked.every((p) => p.label.trim() !== "");
      return true;
    },
    // 顶部横幅文案（目的 + 发起名）
    bannerText(state): string {
      return `选择文件/档案 - 为 ${state.origin?.name ?? ""} 添加关系`;
    },
  },
  actions: {
    /** 进入关系选择模式（详情关系区块添加钮） */
    enterRelation(origin: PickerTarget): void {
      this.active = true;
      this.template = "relation";
      this.origin = origin;
      this.picked = [];
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
      if (origin) {
        void nextTick(() => {
          const selection = useSelectionStore();
          if (origin.kind === "archive") selection.clickFolder(origin.id);
          else selection.selectOnly(`${origin.id}|${origin.folderId ?? "root"}`);
        });
      }
    },

    /** 单击卡片 = 加入/移出备选（多选累积：点新卡追加一组、点已选移除该组）；
     *  发起文件自身不可选（卡片半透明禁选态，后端同拦） */
    togglePick(target: PickerTarget): void {
      if (this.origin && this.origin.kind === target.kind && this.origin.id === target.id) return;
      const i = this.picked.findIndex((p) => p.target.kind === target.kind && p.target.id === target.id);
      if (i >= 0) {
        this.picked.splice(i, 1);
        return;
      }
      this.picked.push({ target, label: "", reverseLabel: "" });
    },

    isPicked(kind: "asset" | "archive", id: string): boolean {
      return this.picked.some((p) => p.target.kind === kind && p.target.id === id);
    },

    /** 确认 = 批量写入（单撤销步）后退模式：成功组就地插入清单 */
    async confirm(): Promise<void> {
      if (!this.canConfirm || !this.origin || this.confirming) return;
      const content = useContentStore();
      const toast = useToastStore();
      this.confirming = true;
      try {
        const rels = await addRelations(this.origin, this.picked);
        // 幂等插入：刷新交错 / 重入下按 id 查重（同一条不重复进清单）
        for (const rel of rels) {
          if (!content.relations.some((r) => r.id === rel.id)) content.relations.push(rel);
        }
        toast.notify({
          message: rels.length > 1 ? `已添加 ${rels.length} 条关系` : `已添加关系：${rels[0].to.name} 是 ${rels[0].from.name} 的 ${rels[0].label}`,
        });
        this.exit();
      } catch (e) {
        toast.notify({ message: String(e), icon: "fail" });
      } finally {
        this.confirming = false;
      }
    },
  },
});
