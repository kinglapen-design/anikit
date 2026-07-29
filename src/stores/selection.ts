// 素材选择态（6.1，2026-07-28 用户定案）：多选 Set + Shift 锚点，会话级状态不进 ui 持久化。
// key = `${assetId}|${folderId}`（Placement 无自身 ID，沿用原 selectedAssetKey 同构格式）。
// 消费口径：详情面板 / 预览等单资产消费方读 content.selectedAssetKey（= 本 store first() 转发）；
// 卡片选中视觉读 isSelected；范围选按 content.visiblePlacements 顺序取锚点区间。
// 文件夹选择域（2026-07-28 复验反馈 5）：selectedFolders（folderId）+ folderAnchor，
// 与素材选择互斥（选文件夹清素材、选素材清文件夹）；全选 / 框选仍只涉及素材；
// 范围选顺序由调用方给（文件夹卡片 = 条带顺序 / 树 item = 可见展开态扁平化顺序），
// 锚点不在同域可见列表时退化单选（与素材范围选同规则）。
// content store 与本 store 互相 lazy 取用（函数内 useXxxStore），避免模块循环引用初始化问题。
import { defineStore } from "pinia";
import { useContentStore, type FolderNode } from "./content";

// 点击修饰键（ctrl 含 macOS metaKey，调用方归一化后传入）
export interface ClickMods {
  ctrl?: boolean;
  shift?: boolean;
}

// Placement 定位三元组（与后端 services/operations PlacementRef 对应，camelCase）
export interface PlacementRef {
  assetId: string;
  folderId: string;
  origin: boolean;
}

// 内部剪贴板（6.1，2026-07-28 用户定案）：仅素材 Placement，文件夹不进；
// copy = 粘贴时弹关联弹窗 / cut = 粘贴时直接移动到当前文件夹（粘贴一次后清空）
export interface InternalClipboard {
  items: PlacementRef[];
  action: "copy" | "cut";
}

// 关联弹窗草稿（C5）：触发源 Ctrl+V(copy) / Ctrl+拖拽落文件夹 / 右键菜单"粘贴"
export interface LinkDraft {
  items: PlacementRef[];
  targetFolder: string; // 落点文件夹 ID（"root" 或 F 开头）
}

// 回收站彻底删除两段式询问（6.1 二轮，2026-07-29）：delete_recycle / clear_recycle 干跑
// 返回 needConfirm 时填入（携带虚拟片段引用数警示），确认后带 confirmed=true 再调——不可恢复，无撤销
export interface RecyclePrompt {
  kind: "delete" | "clear"; // delete = 彻底删除选中项 / clear = 清空回收站
  assetIds: string[];
  segmentRefs: number; // 被虚拟片段引用数（确认窗警示文案）
}

// （2026-07-29 复验三轮：新建文件夹改"先建后命名"，原 NewFolderDraft 命名中临时节点已移除）

function keyOf(p: { assetId: string; folderId: string }): string {
  return `${p.assetId}|${p.folderId}`;
}

export const useSelectionStore = defineStore("selection", {
  state: () => ({
    // 选中集：Set 直接放 state（reactive 集合，has/add/delete 均被追踪，同 content.placements 的既有响应式模式）
    selected: new Set<string>() as Set<string>,
    // Shift 范围选锚点（最后单击项；范围选 / 清空不移动）
    anchor: null as string | null,
    // 文件夹选择域（2026-07-28 复验反馈 5）：folderId 集合 + 范围选锚点；与素材选择互斥
    selectedFolders: new Set<string>() as Set<string>,
    folderAnchor: null as string | null,
    // —— 6.1 阶段 C 操作态（会话级，均不进持久化）——
    clipboard: null as InternalClipboard | null,
    linkDraft: null as LinkDraft | null, // 非空 = 关联弹窗打开（宿主 App.vue）
    recyclePrompt: null as RecyclePrompt | null, // 非空 = 回收站彻底删除确认窗打开（宿主 App.vue，6.1 二轮）
    // 卡片就地重命名目标 key（C2；F2 / 右键菜单设置，单选才可；滚出视口 / Esc = 取消置 null）
    renamingKey: null as string | null,
    // 新建文件夹命名态目标 id（2026-07-29 复验三轮"先建后命名"：createFolder 成功返回 id 后置位，
    // 树对应节点进 naming 输入态，保存 / 退出后清 null）与文件树右键菜单（AppMenu，宿主 LibraryView）
    namingFolderId: null as string | null,
    treeMenu: null as { x: number; y: number; node: FolderNode } | null,
  }),
  getters: {
    // 首项（Set 插入序）：详情面板 / 预览等单资产消费口径；空 = 空串（与原 selectedAssetKey 空串语义一致）
    first(state): string {
      for (const k of state.selected) return k;
      return "";
    },
    count(state): number {
      return state.selected.size;
    },
  },
  actions: {
    isSelected(key: string): boolean {
      return this.selected.has(key);
    },
    isFolderSelected(id: string): boolean {
      return this.selectedFolders.has(id);
    },

    // 素材域选中 = 清文件夹域（互斥，2026-07-28 复验反馈 5）
    clearFolders(): void {
      this.selectedFolders.clear();
      this.folderAnchor = null;
    },

    // 卡片点击三态（2026-07-28 用户定案）：
    // 无修饰 = 单选（清空后选它，anchor = 它）；ctrl = 切换该项（anchor 跟到最后点击）；
    // shift = 锚点（无则当前首项）到该项在当前视口顺序中的区间全选（清空后选区间，anchor 不动）
    clickItem(key: string, mods: ClickMods = {}): void {
      this.clearFolders(); // 选素材清文件夹（互斥）
      if (mods.shift) {
        const keys = useContentStore().visiblePlacements.map(keyOf);
        const to = keys.indexOf(key);
        if (to < 0) return;
        const fromKey = this.anchor ?? this.first;
        const from = fromKey ? keys.indexOf(fromKey) : -1;
        if (from < 0) {
          // 锚点不在当前视口（换过目录等）：退化为单选该项
          this.selected.clear();
          this.selected.add(key);
          this.anchor = key;
          return;
        }
        this.selected.clear();
        const [a, b] = from < to ? [from, to] : [to, from];
        for (let i = a; i <= b; i++) this.selected.add(keys[i]);
        return; // anchor 不动
      }
      if (mods.ctrl) {
        if (this.selected.has(key)) this.selected.delete(key);
        else this.selected.add(key);
        this.anchor = key;
        return;
      }
      this.selected.clear();
      this.selected.add(key);
      this.anchor = key;
    },

    // 文件夹点击三态（2026-07-28 复验反馈 5，与 clickItem 同规则）：
    // order = 同域可见顺序（条带顺序 / 树可见展开态扁平化顺序），范围选取锚点区间；
    // 锚点不在 order 内（跨域 / 折叠未见）退化单选；order 为空时 shift 等同单选
    clickFolder(id: string, mods: ClickMods = {}, order: string[] = []): void {
      // 选文件夹清素材（互斥）；收件箱条目选中一并清（原 FolderStrip 口径）
      this.selected.clear();
      this.anchor = null;
      this.renamingKey = null;
      useContentStore().selectedInboxName = "";
      if (mods.shift) {
        const to = order.indexOf(id);
        const from = this.folderAnchor ? order.indexOf(this.folderAnchor) : -1;
        if (to < 0 || from < 0) {
          this.selectedFolders.clear();
          this.selectedFolders.add(id);
          this.folderAnchor = id;
          return;
        }
        this.selectedFolders.clear();
        const [a, b] = from < to ? [from, to] : [to, from];
        for (let i = a; i <= b; i++) this.selectedFolders.add(order[i]);
        return; // folderAnchor 不动
      }
      if (mods.ctrl) {
        if (this.selectedFolders.has(id)) this.selectedFolders.delete(id);
        else this.selectedFolders.add(id);
        this.folderAnchor = id;
        return;
      }
      this.selectedFolders.clear();
      this.selectedFolders.add(id);
      this.folderAnchor = id;
    },

    // 单选并互斥清收件箱条目选中（卡片点击、右键未选中项先单选同口径，6.1 阶段 C）
    selectOnly(key: string): void {
      this.clickItem(key);
      useContentStore().selectedInboxName = "";
    },

    // 全选（Ctrl+A，2026-07-28 用户定：仅素材卡片口径 = 当前视口全部 Placement）
    selectAll(): void {
      this.clearFolders();
      const keys = useContentStore().visiblePlacements.map(keyOf);
      this.selected.clear();
      for (const k of keys) this.selected.add(k);
      this.anchor = keys[0] ?? null;
    },

    // 框选命中集整体写入（B3）：追加语义由调用方把 drag 起点的 base 并入 hits 后传入
    setSelection(keys: Iterable<string>): void {
      this.clearFolders();
      this.selected.clear();
      for (const k of keys) this.selected.add(k);
    },

    clear(): void {
      this.selected.clear();
      this.anchor = null;
      this.renamingKey = null; // 选择清空 = 重命名输入态一并取消
      this.clearFolders();
    },

    // 选中项三元组清单（6.1 命令入参）：按当前视口顺序输出（稳定、与可见顺序一致），
    // 视口外的选中项（切过目录未 prune 的极少窗口期）追加在后；origin 从 PlacementView 取
    selectedRefs(): PlacementRef[] {
      const content = useContentStore();
      const byKey = new Map(content.placements.map((p) => [keyOf(p), p]));
      const out: PlacementRef[] = [];
      const seen = new Set<string>();
      for (const p of content.visiblePlacements) {
        const k = keyOf(p);
        if (this.selected.has(k) && !seen.has(k)) {
          seen.add(k);
          out.push({ assetId: p.assetId, folderId: p.folderId, origin: p.origin });
        }
      }
      for (const k of this.selected) {
        if (seen.has(k)) continue;
        const p = byKey.get(k);
        if (p) out.push({ assetId: p.assetId, folderId: p.folderId, origin: p.origin });
      }
      return out;
    },

    // 选中文件夹 id 清单（2026-07-28 复验反馈 5，命令入参）：按给定可见顺序输出，
    // 不在顺序内的选中项（条带 / 折叠未见的）按 Set 序追加在后
    orderedSelectedFolderIds(order: string[] = []): string[] {
      const out = order.filter((id) => this.selectedFolders.has(id));
      for (const id of this.selectedFolders) {
        if (!out.includes(id)) out.push(id);
      }
      return out;
    },

    // 被剪切项（2026-07-28 复验反馈 8）：cut 期间卡片半透明；粘贴成功 / 被新 copy 覆盖 /
    // 清空后 clipboard 变化即自动恢复
    isCut(key: string): boolean {
      const c = this.clipboard;
      return !!c && c.action === "cut" && c.items.some((it) => keyOf(it) === key);
    },

    // content.refresh() 全量替换 placements 后调用：剔除已不存在的 key（链接被删 / 移动后 folderId 变等）；
    // 文件夹域同口径剔除（2026-07-28 复验反馈 5）。
    // 2026-07-29 二轮：有效集并上回收站 / 最近使用条目——回收站 key（folderId="recycle"）不在
    // placements 里，彻底删除后条目消失、key 随 prune 清理；最近使用 key 多为真实 Placement 冗余保护
    prune(): void {
      const content = useContentStore();
      const valid = new Set(content.placements.map(keyOf));
      for (const e of content.recycleEntries) valid.add(keyOf(e));
      for (const e of content.recentAssets) valid.add(keyOf(e));
      for (const k of [...this.selected]) {
        if (!valid.has(k)) this.selected.delete(k);
      }
      if (this.anchor && !valid.has(this.anchor)) this.anchor = null;
      const folderIds = new Set<string>();
      const walk = (nodes: FolderNode[]): void => {
        for (const n of nodes) {
          folderIds.add(n.id);
          walk(n.children);
        }
      };
      walk(content.folders);
      for (const id of [...this.selectedFolders]) {
        if (!folderIds.has(id)) this.selectedFolders.delete(id);
      }
      if (this.folderAnchor && !folderIds.has(this.folderAnchor)) this.folderAnchor = null;
    },
  },
});
