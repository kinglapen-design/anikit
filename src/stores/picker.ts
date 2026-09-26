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
// 四用途起（2026-09-08 用户定）：
// - cover 档案封面（模板 file 占位——**单选直回不进备选区**）：封面设置组「更换封面 - 从资源库中选择」
//   进入，origin = 档案；**过滤非封面图**（静态图 png/jpg/jpeg/bmp/webp + gif 动图，排除序列帧包——
//   同烘培可解码格式口径，isThumbCapableExt 镜像后端 is_thumb_capable）；单击图片卡片即
//   pickCoverAsset 解析源图路径 → coverDraft → 退模式恢复档案选中，SectionPinned 消费草稿
//   自动展开封面设置组取景态；确认按钮恒禁用（picked 恒空），Esc / 备选区取消 = 放弃。
// 五用途起（2026-09-23 用户定）：
// - thumb 伴生缩略图（模板 file——**单选直回**同 cover 范式）：详情缩略图区块添加钮进入，
//   origin = 发起工程文件（fla 等）；过滤同 cover（filterImage = 可作伴生缩略图格式口径）；
//   单击图片卡片即 pickThumbAsset → set_thumb_asset 绑定（重复 = 换绑，后端既有语义）→ 退模式
//   恢复发起文件选中；确认按钮恒禁用，Esc / 取消 = 放弃。
// 六用途起（提示词轮，2026-09-05 走查三态定案）：
// - prompt 指定提示词（模板 file——**单选直回**同 cover/thumb 范式）：菜单「指定文本文件为提示词」/
//   详情提示词 tab「指定」钮进入，origin = 右键卡 / 详情目标，promptBatch = 全部选中目标（含 origin，
//   多选批量指定；档案目标 kind "archive" folderId 恒 ""）；**过滤非文本**（filterText：网格只渲染
//   txt/md 卡片、档案区不渲染）；单击文本卡即 pickPromptAsset → 对 batch 逐个 assign_*_prompt
//   （按 kind 分流 asset/archive 两命令）→ 退模式恢复选中；toast 汇总，确认按钮恒禁用。
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { nextTick } from "vue";
import { useContentStore } from "./content";
import { useSelectionStore } from "./selection";
import { useToastStore } from "./toast";
import { useUiStore } from "./ui";
import { useUndoStore } from "./undo";
import { linkPlacements } from "../lib/placementOps";
import { getAssetFilePath } from "../lib/mediaOps";
import { isTextExt } from "../lib/formatBadge";
import { addRelations, type PickerTarget, type PickedRelation } from "../lib/relationOps";

export type PickerTemplate = "file" | "relation";
export type PickerPurpose = "relation" | "link" | "voice" | "cover" | "thumb" | "prompt";

/** 备选匹配键（2026-09-19 BUG 3）：kind+id+folderId 三元组——同资产多条链接位置各自独立；
 *  archive 无位置概念（folderId 恒 ""） */
function pickKey(kind: "asset" | "archive", id: string, folderId?: string): string {
  return `${kind}|${id}|${folderId ?? ""}`;
}

export const usePickerStore = defineStore("picker", {
  state: () => ({
    active: false,
    template: "relation" as PickerTemplate,
    purpose: "relation" as PickerPurpose,
    origin: null as PickerTarget | null, // 发起文件/档案（relation / voice 的 from）
    targetFolder: null as { id: string; name: string } | null, // link 落点文件夹/档案
    filterAudio: false, // voice：网格只渲染音频卡片（AssetGrid / ArchiveStrip 分流）
    filterImage: false, // cover：网格只渲染可作封面的图片卡片（静态图 + gif，排除序列帧包），档案区不渲染
    filterText: false, // prompt：网格只渲染文本文件卡片（txt/md，isTextExt），档案区不渲染
    promptBatch: [] as PickerTarget[], // prompt：指定目标集（origin 含在内；多选批量逐个 assign）
    coverDraft: null as { archiveId: string; sourcePath: string } | null, // cover 单选直回草稿（退模式后由 SectionPinned 一次性消费）
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
      if (state.purpose === "cover") return `选择图片，为 [${state.origin?.name ?? ""}] 设置封面`;
      if (state.purpose === "thumb") return `选择图片，为 [${state.origin?.name ?? ""}] 设置缩略图`;
      if (state.purpose === "prompt") return `选择文本文件，为 [${state.origin?.name ?? ""}] 指定提示词`;
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
      this.coverDraft = null; // 陈旧草稿清场（上轮未消费的封面图源不带入新会话）
      this.filterText = false; // prompt 过滤清场（enterPrompt 随后自置；其余用途恒 false）
      this.promptBatch = []; // prompt 目标集清场（同上）
      // 云母模式 DWM 窗口边框切染品牌色（四重标识之一；与 ::after 品牌框同色重叠单层，
      // 2026-08-30 实机定；实色模式 DWM 忽略、前端自绘，调用无害；失败告警不阻断——
      // 掉回常态色前端品牌框仍在，可降级）
      void invoke("set_main_border_picking", { picking: true }).catch((e) => console.warn("[picker] DWM 边框切色失败：", e));
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

    /** 进入档案封面选择模式（封面设置组「更换封面 - 从资源库中选择」，2026-09-08 点亮）：
     *  过滤非封面图；单击图片卡片即定（pickCoverAsset 单选直回，不进备选区） */
    enterCover(origin: PickerTarget): void {
      this.enter();
      this.template = "file";
      this.purpose = "cover";
      this.origin = origin;
      this.targetFolder = null;
      this.filterAudio = false;
      this.filterImage = true;
    },

    /** 进入伴生缩略图选择模式（详情缩略图区块添加钮，2026-09-23 点亮）：过滤同 cover
     *  （可作伴生缩略图格式）；单击图片卡片即定（pickThumbAsset 单选直回，不进备选区） */
    enterThumb(origin: PickerTarget): void {
      this.enter();
      this.template = "file";
      this.purpose = "thumb";
      this.origin = origin;
      this.targetFolder = null;
      this.filterAudio = false;
      this.filterImage = true;
    },

    /** 进入指定提示词选择模式（提示词轮：菜单「指定文本文件为提示词」/ 详情提示词 tab「指定」钮）：
     *  过滤非文本（filterText，网格只渲 txt/md、档案区不渲染）；单击文本卡即定
     *  （pickPromptAsset 单选直回对 promptBatch 逐个指定，不进备选区）；
     *  origin = 右键卡 / 详情目标，batch = 全部选中目标（含 origin；档案 kind "archive" folderId 恒 ""） */
    enterPrompt(origin: PickerTarget, batch: PickerTarget[]): void {
      this.enter();
      this.template = "file";
      this.purpose = "prompt";
      this.origin = origin;
      this.targetFolder = null;
      this.filterAudio = false;
      this.filterImage = false;
      this.filterText = true;
      this.promptBatch = batch.length > 0 ? batch : [origin];
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
      void invoke("set_main_border_picking", { picking: false }).catch((e) => console.warn("[picker] DWM 边框回色失败：", e));
      this.origin = null;
      this.targetFolder = null;
      this.filterAudio = false;
      this.filterImage = false;
      this.filterText = false;
      this.promptBatch = [];
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
     *  file 模板行 name 草稿默认取源显示名（备选区输入框可改，确认按行写入）；
     *  匹配键 = 三元组 kind+id+folderId（2026-09-19 BUG 3：曾按 kind+id 匹配——同资产多条
     *  链接位置共享 assetId，任一位置入备选、同资产全部位置卡片都亮大勾，点另一条还误判
     *  "取消"→ 改为 Placement 级独立选择；archive 无位置概念 folderId 恒 ""） */
    togglePick(target: PickerTarget): void {
      if (this.origin && this.origin.kind === target.kind && this.origin.id === target.id) return;
      const key = pickKey(target.kind, target.id, target.folderId);
      const i = this.picked.findIndex((p) => pickKey(p.target.kind, p.target.id, p.target.folderId) === key);
      if (i >= 0) {
        this.picked.splice(i, 1);
        return;
      }
      this.picked.push({ target, label: "", reverseLabel: "", name: target.name });
    },

    isPicked(kind: "asset" | "archive", id: string, folderId?: string): boolean {
      const key = pickKey(kind, id, folderId);
      return this.picked.some((p) => pickKey(p.target.kind, p.target.id, p.target.folderId) === key);
    },

    /** cover 用途单选直回（2026-09-08 用户定：选择图片后不进备选区，直接显示在封面里）：
     *  解析资产源图路径 → coverDraft → 退模式（exit 恢复档案选中 → SectionPinned 消费草稿
     *  自动展开封面设置组取景态）；解析失败留在模式内可改选别的 */
    async pickCoverAsset(target: PickerTarget): Promise<void> {
      const origin = this.origin;
      if (!origin || this.purpose !== "cover") return;
      const toast = useToastStore();
      try {
        const r = await getAssetFilePath(target.id);
        if (!r.path) {
          toast.notify({ message: "该文件没有可用图片，无法用作封面", icon: "fail" });
          return;
        }
        this.coverDraft = { archiveId: origin.id, sourcePath: r.path };
        this.exit();
      } catch (e) {
        toast.notify({ message: String(e), icon: "fail" });
      }
    },

    /** prompt 用途单选直回（提示词轮）：点文本卡即对 promptBatch 逐个指定提示词
     *  （按 kind 分流 assign_asset_prompt / assign_archive_prompt；逐目标 try 失败计数不中断）；
     *  守卫 isTextExt（网格已过滤，此处双保险防列表态漏网）；全失败留模式内可改选，
     *  有成功即退模式（exit 恢复发起目标选中）+ 全量刷新 + toast 汇总 */
    async pickPromptAsset(target: PickerTarget): Promise<void> {
      const origin = this.origin;
      if (!origin || this.purpose !== "prompt") return;
      const toast = useToastStore();
      if (!isTextExt(target.ext ?? "")) {
        toast.notify({ message: "只能指定文本文件（txt / md）为提示词", icon: "fail" });
        return;
      }
      const batch = this.promptBatch.length > 0 ? [...this.promptBatch] : [origin];
      let done = 0;
      let failed = 0;
      let firstError: string | null = null;
      for (const t of batch) {
        try {
          if (t.kind === "archive") await invoke("assign_archive_prompt", { folderId: t.id, promptAssetId: target.id });
          else await invoke("assign_asset_prompt", { assetId: t.id, promptAssetId: target.id });
          done++;
        } catch (e) {
          failed++;
          firstError = firstError ?? String(e);
        }
      }
      if (done === 0) {
        toast.notify({ message: firstError ?? "指定提示词失败", icon: "fail" });
        return;
      }
      this.exit();
      const content = useContentStore();
      await content.refresh(); // 全量对齐（promptState 在 PlacementView / 档案 prompt 在 archives，refresh 含 refreshArchives）
      content.bumpDetailRev();
      void useUndoStore().refreshState();
      toast.notify({
        message:
          done > 1
            ? `已为 ${done} 个目标指定提示词${failed > 0 ? `，${failed} 个失败` : ""}`
            : `已为「${origin.name}」指定提示词`,
      });
    },

    /** thumb 用途单选直回（2026-09-23，详情缩略图区块添加钮）：点图即 set_thumb_asset 绑定
     * （重复 = 换绑，后端既有语义；发起工程自身不可选——后端同拦「不能把资产设为自身的缩略图」），
     * 退模式恢复发起文件选中；刷新 = 全量 refresh（卡片代理图）+ bumpDetailRev（详情缩略图区块） */
    async pickThumbAsset(target: PickerTarget): Promise<void> {
      const origin = this.origin;
      if (!origin || this.purpose !== "thumb") return;
      if (target.kind === "asset" && target.id === origin.id) return;
      const toast = useToastStore();
      try {
        await invoke("set_thumb_asset", { assetId: origin.id, thumbId: target.id });
        this.exit();
        const content = useContentStore();
        await content.refresh();
        content.bumpDetailRev();
        toast.notify({ message: `已将「${target.name}」设置为缩略图`, icon: "success" });
      } catch (e) {
        toast.notify({ message: String(e), icon: "fail" });
      }
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
        // relation / voice = 资产级关系：同资产多条链接位置（BUG 3 起按 Placement 独立选择）
        // 折叠为首条——同资产不重复建关系
        const seen = new Set<string>();
        const items = this.picked.filter((p) => {
          const k = `${p.target.kind}|${p.target.id}`;
          if (seen.has(k)) return false;
          seen.add(k);
          return true;
        });
        const rels = await addRelations(this.origin, items, kind);
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
