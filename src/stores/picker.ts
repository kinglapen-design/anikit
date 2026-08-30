// 主窗口选择模式（6.2，PRD 03_数据链接 04：资产选择器 = 主窗口就地选择，不弹窗）：
// 四重标识 = 窗口整圈品牌色边框 + 顶部目的横幅 + 右侧详情栏临时替换备选区 + 已选卡片压暗大勾；
// 交互变化（2026-08-24 用户收窄白名单）= 只放行：树 / 快速访问导航、搜索、查看方式、挑选文件
//（点选 + 双击预览）、面包屑后退前进；其余全禁（编辑类 / 汉堡 / 切库 / 预览内编辑等）。
// 单击卡片 = 加入/移出备选（不再是选中）；Esc / 取消 = 全部放弃，确认 = 按用途执行后退出回触发处。
// 三用途（2026-08-24 用户定案）：
// - relation 关系（模板 relation）：多选累积整组复制，双框称谓，确认 add_relations(kind=free)；
// - link 添加文件链接（模板 file）：文件夹/档案右键「添加文件/档案链接」(Ctrl+L) / 工具栏钮进入，
//   落点 = 目标文件夹（targetFolder）；备选区行 = 名称输入框（默认源名可改）+ 链接徽章 + 红删，
//   确认 link_placements 按行写入（行 = 输入框可改显示名，PRD 03-04 多选口径——稿的"名称框本身就是输入框"）；
// - voice 链接音频（模板 file）：详情音频区块添加钮进入，origin = 发起文件/档案；
//   **过滤非音频**（02 稿"过滤掉非音频文件去掉噪音"：网格只渲染音频卡片、档案区不渲染），
//   确认 add_relations(kind=voice)（无称谓，后端校验 to 必须音频 asset）。
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { nextTick } from "vue";
import { useContentStore } from "./content";
import { useSelectionStore } from "./selection";
import { useToastStore } from "./toast";
import { useUiStore } from "./ui";
import { linkPlacements } from "../lib/placementOps";
import { addRelations, type PickerTarget, type PickedRelation } from "../lib/relationOps";

export type PickerTemplate = "file" | "relation";
export type PickerPurpose = "relation" | "link" | "voice";

export const usePickerStore = defineStore("picker", {
  state: () => ({
    active: false,
    template: "relation" as PickerTemplate,
    purpose: "relation" as PickerPurpose,
    origin: null as PickerTarget | null, // 发起文件/档案（relation / voice 的 from）
    targetFolder: null as { id: string; name: string } | null, // link 落点文件夹/档案
    filterAudio: false, // voice：网格只渲染音频卡片（AssetGrid / ArchiveStrip 分流）
    picked: [] as PickedRelation[], // 备选组（多选累积；relation 用双框称谓，file 模板用 name 草稿）
    confirming: false, // 确认在途防重入（连点确认 = 重复建关系，2026-08-23 探针实证）
  }),
  getters: {
    // 确认可用：关系模板 = 有备选 + 每组左框称谓非空；文件模板 = 有备选
    canConfirm(state): boolean {
      if (state.picked.length === 0) return false;
      if (state.template === "relation") return state.picked.every((p) => p.label.trim() !== "");
      return true;
    },
    // 顶部横幅文案（目的 + 发起名；格式贴阶段七通知栏-选择模式稿：逗号分隔 + 名称方括号，2026-08-29）
    bannerText(state): string {
      if (state.purpose === "voice") return `选择音频，为 [${state.origin?.name ?? ""}] 链接音频`;
      if (state.purpose === "link") return `选择文件，为 [${state.targetFolder?.name ?? ""}] 添加链接文件`;
      return `选择文件/档案，为 [${state.origin?.name ?? ""}] 添加关系`;
    },
  },
  actions: {
    /** 进入模式公共段：清场 + 强制展开右栏（四重标识缺一，用户无处确认） */
    enter(): void {
      this.active = true;
      this.picked = [];
      // 云母模式 DWM 窗口边框切染品牌色（四重标识之一；与 ::after 品牌框同色重叠单层，
      // 2026-08-30 实机定；实色模式 DWM 忽略、前端自绘，调用无害；失败静默降级）
      void invoke("set_main_border_picking", { picking: true }).catch(() => {});
      // 进入即清全局选择：单击语义已变为加入/移出备选，残留选中只会误导
      useSelectionStore().clear();
      const ui = useUiStore();
      if (ui.rightCollapsed) ui.rightCollapsed = false;
    },

    /** 进入关系选择模式（详情关系区块添加钮） */
    enterRelation(origin: PickerTarget): void {
      this.enter();
      this.template = "relation";
      this.purpose = "relation";
      this.origin = origin;
      this.targetFolder = null;
      this.filterAudio = false;
    },

    /** 进入链接音频模式（详情音频区块添加钮）：文件模板 + 过滤非音频（02 稿） */
    enterVoice(origin: PickerTarget): void {
      this.enter();
      this.template = "file";
      this.purpose = "voice";
      this.origin = origin;
      this.targetFolder = null;
      this.filterAudio = true;
    },

    /** 进入添加文件链接模式（文件夹/档案右键 Ctrl+L / 工具栏钮）：落点 = 目标文件夹 */
    enterLink(folder: { id: string; name: string }): void {
      this.enter();
      this.template = "file";
      this.purpose = "link";
      this.origin = null;
      this.targetFolder = folder;
      this.filterAudio = false;
    },

    /** 退出模式（取消 / Esc / 确认成功后）：全部状态清场 + 回到触发处（PRD 03_数据链接 04——
     *  恢复发起目标选中：asset 按 key 直选（placements 全量，视口不含也出详情）、archive 入文件夹
     *  选择域；nextTick 让过 App.vue sidebarSelected watch 的清选择，同 FolderTree 入域口径）；
     *  link 无 origin（触发 = 文件夹/工具栏），视口本就在落点文件夹，不恢复选中 */
    exit(): void {
      const origin = this.origin;
      this.active = false;
      // DWM 窗口边框染回常态色（通用/六级 #333333，同 enter 口径反向）
      void invoke("set_main_border_picking", { picking: false }).catch(() => {});
      this.origin = null;
      this.targetFolder = null;
      this.filterAudio = false;
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
     *  发起文件自身不可选（卡片半透明禁选态，后端同拦）；
     *  file 模板行 name 草稿默认取源显示名（备选区输入框可改，确认按行写入） */
    togglePick(target: PickerTarget): void {
      if (this.origin && this.origin.kind === target.kind && this.origin.id === target.id) return;
      const i = this.picked.findIndex((p) => p.target.kind === target.kind && p.target.id === target.id);
      if (i >= 0) {
        this.picked.splice(i, 1);
        return;
      }
      this.picked.push({ target, label: "", reverseLabel: "", name: target.name });
    },

    isPicked(kind: "asset" | "archive", id: string): boolean {
      return this.picked.some((p) => p.target.kind === kind && p.target.id === id);
    },

    /** 确认 = 按用途执行后退出（relation/voice 批量写入单撤销步；link 按行写入落点文件夹） */
    async confirm(): Promise<void> {
      if (!this.canConfirm || this.confirming) return;
      const content = useContentStore();
      const toast = useToastStore();
      this.confirming = true;
      try {
        if (this.purpose === "link") {
          const folder = this.targetFolder;
          if (!folder) return;
          const ok = await linkPlacements(
            this.picked.map((p) => ({ assetId: p.target.id, folderId: p.target.folderId ?? "root", origin: p.target.origin ?? false })),
            folder.id,
            this.picked.map((p) => p.name?.trim() || null),
          );
          if (ok) this.exit();
          return;
        }
        if (!this.origin) return;
        const kind = this.purpose === "voice" ? "voice" : undefined;
        const rels = await addRelations(this.origin, this.picked, kind);
        // 幂等插入：刷新交错 / 重入下按 id 查重（同一条不重复进清单）
        for (const rel of rels) {
          if (!content.relations.some((r) => r.id === rel.id)) content.relations.push(rel);
        }
        if (this.purpose === "voice") {
          toast.notify({
            message: rels.length > 1 ? `已链接 ${rels.length} 条音频` : `已链接音频：${rels[0].to.name}`,
          });
        } else {
          toast.notify({
            message: rels.length > 1 ? `已添加 ${rels.length} 条关系` : `已添加关系：${rels[0].to.name} 是 ${rels[0].from.name} 的 ${rels[0].label}`,
          });
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
