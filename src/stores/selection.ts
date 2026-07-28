// 素材选择态（6.1，2026-07-28 用户定案）：多选 Set + Shift 锚点，会话级状态不进 ui 持久化。
// key = `${assetId}|${folderId}`（Placement 无自身 ID，沿用原 selectedAssetKey 同构格式）。
// 消费口径：详情面板 / 预览等单资产消费方读 content.selectedAssetKey（= 本 store first() 转发）；
// 卡片选中视觉读 isSelected；范围选按 content.visiblePlacements 顺序取锚点区间。
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

// 删除两段式询问（C1）：第一段返回 needConfirm 时填入，询问窗选择后带 lastAction 再调
export interface DeletePrompt {
  items: PlacementRef[];
  lastCount: number; // 涉及"最后一条 Placement"的资源数（询问窗文案）
}

// 新建文件夹"命名中"临时节点（C3）：确认名称后才调 create_folder，取消则不入库
// （后端无 rename_folder，故先以前端临时态插入，2026-07-28 定案）
export interface NewFolderDraft {
  parentId: string | null; // null = 根级
  afterFolderId: string | null; // 插到该兄弟之后；null = 该级末尾
}

function keyOf(p: { assetId: string; folderId: string }): string {
  return `${p.assetId}|${p.folderId}`;
}

export const useSelectionStore = defineStore("selection", {
  state: () => ({
    // 选中集：Set 直接放 state（reactive 集合，has/add/delete 均被追踪，同 content.placements 的既有响应式模式）
    selected: new Set<string>() as Set<string>,
    // Shift 范围选锚点（最后单击项；范围选 / 清空不移动）
    anchor: null as string | null,
    // —— 6.1 阶段 C 操作态（会话级，均不进持久化）——
    clipboard: null as InternalClipboard | null,
    linkDraft: null as LinkDraft | null, // 非空 = 关联弹窗打开（宿主 App.vue）
    deletePrompt: null as DeletePrompt | null, // 非空 = 删除询问窗打开（宿主 App.vue）
    // 卡片就地重命名目标 key（C2；F2 / 右键菜单设置，单选才可；滚出视口 / Esc = 取消置 null）
    renamingKey: null as string | null,
    // 新建文件夹"命名中"临时节点（C3）与文件树右键菜单（AppMenu，宿主 LibraryView）
    newFolderDraft: null as NewFolderDraft | null,
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

    // 卡片点击三态（2026-07-28 用户定案）：
    // 无修饰 = 单选（清空后选它，anchor = 它）；ctrl = 切换该项（anchor 跟到最后点击）；
    // shift = 锚点（无则当前首项）到该项在当前视口顺序中的区间全选（清空后选区间，anchor 不动）
    clickItem(key: string, mods: ClickMods = {}): void {
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

    // 单选并互斥清文件夹卡片 / 收件箱条目选中（卡片点击、右键未选中项先单选同口径，6.1 阶段 C）
    selectOnly(key: string): void {
      this.clickItem(key);
      const content = useContentStore();
      content.selectedFolderId = "";
      content.selectedInboxName = "";
    },

    // 全选（Ctrl+A，2026-07-28 用户定：仅素材卡片口径 = 当前视口全部 Placement）
    selectAll(): void {
      const keys = useContentStore().visiblePlacements.map(keyOf);
      this.selected.clear();
      for (const k of keys) this.selected.add(k);
      this.anchor = keys[0] ?? null;
    },

    // 框选命中集整体写入（B3）：追加语义由调用方把 drag 起点的 base 并入 hits 后传入
    setSelection(keys: Iterable<string>): void {
      this.selected.clear();
      for (const k of keys) this.selected.add(k);
    },

    clear(): void {
      this.selected.clear();
      this.anchor = null;
      this.renamingKey = null; // 选择清空 = 重命名输入态一并取消
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

    // content.refresh() 全量替换 placements 后调用：剔除已不存在的 key（链接被删 / 移动后 folderId 变等）
    prune(): void {
      const valid = new Set(useContentStore().placements.map(keyOf));
      for (const k of [...this.selected]) {
        if (!valid.has(k)) this.selected.delete(k);
      }
      if (this.anchor && !valid.has(this.anchor)) this.anchor = null;
    },
  },
});
