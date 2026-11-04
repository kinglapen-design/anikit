// 素材选择态（6.1，2026-07-28 用户定案）：多选 Set + Shift 锚点，会话级状态不进 ui 持久化。
// key = placementKey(p)（content.ts 导出）：基底 `${assetId}|${folderId}`，同资产同文件夹多条
// 位置（origin + 链接 / 重复链接）撞键时加 `|origin|name` 后缀精确到行（2026-10-11 选择域独立化——
// 实机报"同文件原始与链接同时被选中"；此前 2026-09-19 只修过选择模式 picker 域，主选择域未修）。
// 消费口径：详情面板 / 预览等单资产消费方读 content.selectedAssetKey（= 本 store first() 转发）；
// 卡片选中视觉读 isSelected；范围选 / 全选 / 选中集输出按 content.displayOrderFor 视口显示序
//（分组视口 = 组拼接扁平序，与网格所见一致——2026-10-08 修复分组下范围选跨组带项）。
// 文件夹选择域（2026-07-28 复验反馈 5）：selectedFolders（folderId）+ folderAnchor，
// 与素材选择互斥（选文件夹清素材、选素材清文件夹）；全选 / 框选仍只涉及素材；
// 范围选顺序由调用方给（文件夹卡片 = 条带顺序 / 树 item = 可见展开态扁平化顺序），
// 锚点不在同域可见列表时退化单选（与素材范围选同规则）。
// content store 与本 store 互相 lazy 取用（函数内 useXxxStore），避免模块循环引用初始化问题。
import { defineStore } from "pinia";
import { placementKey, placementRef, useContentStore, type FolderNode } from "./content";
import { useWsStore } from "./ws";

// 点击修饰键（ctrl 含 macOS metaKey，调用方归一化后传入）
export interface ClickMods {
  ctrl?: boolean;
  shift?: boolean;
}

// Placement 定位三元组（与后端 services/operations PlacementRef 对应，camelCase）；
// name（2026-10-11）：撞键行（同资产同文件夹多条位置）携带显示名精确到行——后端 find_placement
// 见名即按行匹配，陈旧名 = 匹配落空跳过（不错伤他行）；非撞键项不携带（无字段）
export interface PlacementRef {
  assetId: string;
  folderId: string;
  origin: boolean;
  name?: string;
}

// 内部剪贴板（6.1，2026-07-28 用户定案）：仅素材 Placement，文件夹不进；
// copy = 粘贴时静默建链接（2026-08-18 定案废除关联弹窗）/ cut = 粘贴时直接移动到当前文件夹（粘贴一次后清空）。
// folders（3.4 文件夹剪切粘贴，2026-08-18）：与 items 互斥（选择域互斥，一次只剪切一类）；
// 文件夹只有剪切（= 移动，move_folders 护栏），复制语义 = 克隆（Ctrl+D）
export interface InternalClipboard {
  items: PlacementRef[];
  action: "copy" | "cut";
  folders?: string[];
}

// 链接管理弹窗草稿（17 稿，6.2 三轮）：文件右键「文件链接管理」Ctrl+J / 详情链接区块添加钮，
// 非空 = 弹窗打开（宿主 App.vue LinkManageDialog）；档案链接管理随档案多位置立项（2026-08-24 用户定）
export interface LinkManageDraft {
  assetId: string;
  name: string; // 显示名快照（弹窗标题栏副标 / 兜底显示）
}

// 回收站彻底删除两段式询问（6.1 二轮，2026-07-29）：delete_recycle / clear_recycle 干跑
// 返回 needConfirm 时填入（携带虚拟片段引用数 + 快照内链接数警示——2026-08-18 用户定：
// 彻底清除前提示"所有关联的链接文件都将被清除"），确认后带 confirmed=true 再调——不可恢复，无撤销
export interface RecyclePrompt {
  kind: "delete" | "clear"; // delete = 彻底删除选中项 / clear = 清空回收站
  assetIds: string[];
  segmentRefs: number; // 被虚拟片段引用数（确认窗警示文案）
  linkRefs: number; // 快照内链接 Placement 数（确认窗警示文案）
}

// 通用确认弹窗（6.3）：轻量一次性确认（警示 / 可撤销操作的前置说明 / 不可逆操作二次确认）
export interface ConfirmPrompt {
  title: string;
  body: string;
  // 富文本正文（3.4 移动确认：发起名与目标名高亮——2026-09-08 用户定改 Animate 插件面板
  // 方括号强调口径：加粗 + 亮色/二级，「」括号并入强调段）——给了它就不读 body
  bodySegments?: { text: string; accent?: boolean }[];
  confirmLabel?: string; // 缺省"确认"
  // 确认钮语义档（2026-09-08 用户定：旧 alert 48 徽章随 ConfirmModal 换代没了，纯靠按钮颜色
  // 区分轻重——按语义显式给；不给则派生：alert warning→warning / alert error·danger→danger / 否则 brand）
  kind?: "warning" | "danger" | "brand";
  danger?: boolean; // 主按钮用错误色（不可逆操作）
  alert?: "warning" | "error"; // 侵入式警示（figma 09：提示音；旧 48 徽章随 2026-09-08 换代废除）
  onConfirm: () => void;
}

// 移动面板草稿（3.4，29 稿）：菜单 / Ctrl+Y 触发时自选择域快照——
// assets = 移动文件（move_placements）；folders = 移动文件夹/档案（move_folders，含护栏）
export type MoveDraft =
  | { kind: "assets"; refs: PlacementRef[] }
  | { kind: "folders"; ids: string[] };

// 目的树选择器草稿（2026-10-03 库 ↔ 工作空间互通，PRD 02 互通节）：「导出到工作空间 /
// 导入到库」共用的移动面板样式树选择弹窗（宿主 App.vue DestTreeDialog）——
// tree = 数据源（"ws" = 工作空间目录树，懒加载逐级展开 / "library" = 库文件夹树含未分类行）；
// onPick = 单击落点确认后的执行闭包：ws 模式 target = rel（"" = 根），library 模式 = folderId
//（null = 未分类根）；返回 true 关面板（pinia state 存闭包 = confirmPrompt.onConfirm 先例）
export interface DestTreeDraft {
  tree: "ws" | "library";
  title: string; // 弹窗标题 + 确认钮文案（移动面板口径）
  sourceName: string; // 发起名（确认文案品牌色高亮段）
  onPick: (target: string | null) => Promise<boolean>;
}

// （2026-07-29 复验三轮：新建文件夹改"先建后命名"，原 NewFolderDraft 命名中临时节点已移除）

// 选择键单写点 = content.placementKey（撞键后缀判定在它那里；本函数仅留窄签名兼容旧调用点）
function keyOf(p: { assetId: string; folderId: string; origin?: boolean; name?: string }): string {
  return placementKey(p);
}

// 命令入参行 = content.placementRef（撞键行附 name 精确到行，2026-10-11）
const refOf = placementRef;

export const useSelectionStore = defineStore("selection", {
  state: () => ({
    // 选中集：Set 直接放 state（reactive 集合，has/add/delete 均被追踪，同 content.placements 的既有响应式模式）
    selected: new Set<string>() as Set<string>,
    // Shift 范围选锚点（最后单击项；范围选 / 清空不移动）
    anchor: null as string | null,
    // 文件夹选择域（2026-07-28 复验反馈 5）：folderId 集合 + 范围选锚点；与素材选择互斥
    selectedFolders: new Set<string>() as Set<string>,
    folderAnchor: null as string | null,
    // 收件箱磁盘条目选择域（2026-11-04 批量手动导入）：与素材 / 文件夹选择互斥
    selectedInbox: new Set<string>() as Set<string>,
    inboxAnchor: null as string | null,
    // —— 6.1 阶段 C 操作态（会话级，均不进持久化）——
    clipboard: null as InternalClipboard | null,
    // 内剪贴板写入时刻的系统剪贴板序号（2026-09-17 粘贴仲裁）：Ctrl+V 时序号变了 =
    // 用户在外部复制了新内容（覆盖 APP 内剪贴板）→ 内部粘贴让位系统剪贴板导入；
    // null = 旧会话未记录（按内部优先原口径）
    clipboardSeq: null as number | null,
    // 复制的属性词 / 描述（2026-09-05 走查「上下文菜单」：复制数据组加「复制标签/复制描述」，
    // 复制后右键多出对应粘贴项）：会话级双槽——复制标签不覆盖已复制的描述；粘贴只读不写槽，可连续多处粘贴
    copiedData: { tags: null, descriptions: null } as {
      tags: string[] | null;
      descriptions: { title: string; content: string }[] | null;
    },
    // 缩略图链接（批次 4，2026-09-02）：「转换为缩略图链接并复制」写入、「将 xx 设置为缩略图」消费——
    // 纯引用复制（不动库数据，hidden 在实际绑定 set_thumb_asset 时打）；会话级不落盘
    thumbLink: null as { assetId: string; name: string } | null,
    linkManageDraft: null as LinkManageDraft | null, // 非空 = 链接管理弹窗打开（宿主 App.vue，6.2 三轮）
    recyclePrompt: null as RecyclePrompt | null, // 非空 = 回收站彻底删除确认窗打开（宿主 App.vue，6.1 二轮）
    // 卡片就地重命名目标 key（C2；F2 / 右键菜单设置，单选才可；滚出视口 / Esc = 取消置 null）
    renamingKey: null as string | null,
    // 新建文件夹命名态目标 id（2026-07-29 复验三轮"先建后命名"：createFolder 成功返回 id 后置位，
    // 树对应节点进 naming 输入态，保存 / 退出后清 null）
    namingFolderId: null as string | null,
    // 通用确认弹窗（6.3：文件管理器打开警示 / 删除文件夹 / 彻底删除素材·文件夹，宿主 App.vue）
    confirmPrompt: null as ConfirmPrompt | null,
    // 断链修复弹窗（junction 方向 b 二轮，2026-10-02 用户定：选中断链链接目录即弹——
    // 主钮 重新定位… / 次钮 删除链接 / 取消 = 先不管；宿主 App.vue）：
    // 三触发源 = 选中单断链（App.vue watch）/ 导航拦截（tabs.openView ws 守卫）/ 幽灵回退（缓存过期实际已断）
    brokenLinkPrompt: null as { rel: string; name: string } | null,
    // 移动面板草稿（3.4，29 稿移动面板宿主 App.vue）：非空 = 移动面板打开
    moveDraft: null as MoveDraft | null,
    // 目的树选择器草稿（2026-10-03 库 ↔ 工作空间互通，宿主 App.vue DestTreeDialog）：
    // 非空 = 「导出到工作空间 / 导入到库」树选择弹窗打开
    destTreeDraft: null as DestTreeDraft | null,
    // 批量重命名草稿（3.4，37 稿批量重命名弹窗宿主 App.vue）：多选「重命名」/ Ctrl+R 触发，
    // 非空 = 弹窗打开；内容为打开时刻的选中 Placement 快照
    renameDraft: null as PlacementRef[] | null,
    // 文件夹批量重命名草稿（2026-10-11，PRD 02-14）：folderId 列表（树 DFS 视觉序，
    // selectedFolderIdsInTreeOrder 产出），与 renameDraft 互斥（两选择域天然互斥）；
    // 同一弹窗 BatchRenameDialog 文件夹模式消费（宿主 App.vue）
    renameFolderDraft: null as string[] | null,
    // 文件夹卡片就地重命名目标 id（6.3 右键"重命名"与名称双击同入口，FolderCard 读）
    namingFolderCardId: null as string | null,
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
    inboxCount(state): number {
      return state.selectedInbox.size;
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

    clearInbox(): void {
      this.selectedInbox.clear();
      this.inboxAnchor = null;
    },

    // 收件箱与素材卡同套三态：单选 / Ctrl 切换 / Shift 按当前磁盘显示序范围选
    clickInbox(name: string, mods: ClickMods = {}, order: string[] = []): void {
      this.selected.clear();
      this.anchor = null;
      this.renamingKey = null;
      this.clearFolders();
      const names = order.length > 0 ? order : useContentStore().inboxDisk.map((e) => e.name);
      if (mods.shift) {
        const to = names.indexOf(name);
        const from = this.inboxAnchor ? names.indexOf(this.inboxAnchor) : -1;
        if (to < 0 || from < 0) {
          this.selectedInbox.clear();
          this.selectedInbox.add(name);
          this.inboxAnchor = name;
          return;
        }
        this.selectedInbox.clear();
        const [a, b] = from < to ? [from, to] : [to, from];
        for (let i = a; i <= b; i++) this.selectedInbox.add(names[i]);
        return;
      }
      if (mods.ctrl) {
        if (this.selectedInbox.has(name)) this.selectedInbox.delete(name);
        else this.selectedInbox.add(name);
      } else {
        this.selectedInbox.clear();
        this.selectedInbox.add(name);
      }
      this.inboxAnchor = name;
    },

    pruneInbox(names: Iterable<string>): void {
      const alive = new Set(names);
      for (const name of [...this.selectedInbox]) {
        if (!alive.has(name)) this.selectedInbox.delete(name);
      }
      if (this.inboxAnchor && !alive.has(this.inboxAnchor)) this.inboxAnchor = null;
    },

    // 卡片点击三态（2026-07-28 用户定案）：
    // 无修饰 = 单选（清空后选它，anchor = 它）；ctrl = 切换该项（anchor 跟到最后点击）；
    // shift = 锚点（无则当前首项）到该项在当前视口顺序中的区间全选（清空后选区间，anchor 不动）。
    // order：范围选顺序调用方显式给——AssetGrid 恒传本网格 displayItems（组拼接显示序 / 搜索态
    // 平铺过滤集，2026-10-08 起）；ws 文件夹卡片 = 条带顺序（文件夹不是 Placement 不在视口列表）。
    // 缺省 = 当前激活视口显示序（content.displayOrderFor：分组视口 = 组拼接序）
    clickItem(key: string, mods: ClickMods = {}, order?: string[]): void {
      this.clearFolders(); // 选素材清文件夹（互斥）
      if (mods.shift) {
        const content = useContentStore();
        const keys = order ?? content.displayOrderFor(content.sidebarSelected).map(keyOf);
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
      this.clearInbox();
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
      this.clearInbox();
    },

    // 全选（Ctrl+A，2026-07-28 用户定：仅素材卡片口径 = 当前视口全部 Placement）；
    // 键序 = 视口显示序（displayOrderFor，分组视口 = 组拼接序——锚点 = 视觉首项，2026-10-08）
    selectAll(): void {
      this.clearFolders();
      const content = useContentStore();
      const keys = content.displayOrderFor(content.sidebarSelected).map(keyOf);
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
      // 空白反选、切换选择域与 Esc 都清空收件箱选择及 Shift 锚点
      this.clearInbox();
    },

    // 选中项三元组清单（6.1 命令入参）：按当前视口显示序输出（displayOrderFor——分组视口 = 组拼接
    // 扁平序，与用户所见一致，2026-10-08；批量重命名编号等顺序敏感消费方随视觉序），
    // 视口外的选中项（切过目录未 prune 的极少窗口期）追加在后；origin 从 PlacementView 取，
    // 撞键行附 name 精确到行（refOf，2026-10-11 选择域独立化）。
    // ws 文件夹键兜底合成（2026-09-26 ws 文件夹卡片 / 树行选中入素材域）：文件夹不是
    // Placement 查不到，按键形直合成——ws 写操作全按路径，只取 assetId 里的 rel；
    // ws 键只在 ws 视口产生（切视图即清空），库命令路径收不到
    selectedRefs(): PlacementRef[] {
      const content = useContentStore();
      const byKey = new Map(content.placements.map((p) => [keyOf(p), p]));
      const out: PlacementRef[] = [];
      const seen = new Set<string>();
      for (const p of content.displayOrderFor(content.sidebarSelected)) {
        const k = keyOf(p);
        if (this.selected.has(k) && !seen.has(k)) {
          seen.add(k);
          out.push(refOf(p));
        }
      }
      for (const k of this.selected) {
        if (seen.has(k)) continue;
        const p = byKey.get(k);
        if (p) {
          out.push(refOf(p));
        } else if (k.startsWith("ws:")) {
          const [assetId, folderId] = k.split("|");
          out.push({ assetId, folderId: folderId ?? "", origin: true });
        }
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

    // 选中文件夹按树 DFS 视觉序输出（2026-10-11 文件夹批量重命名：编号顺序口径——
    // 与素材批量重命名「编号按视觉序」同款；条带选中 = 树里同父级 children 序，天然是 DFS 序子序列；
    // 折叠未见的选中项落在 DFS 序里同样覆盖）
    selectedFolderIdsInTreeOrder(): string[] {
      const content = useContentStore();
      const order: string[] = [];
      const walk = (nodes: FolderNode[]): void => {
        for (const n of nodes) {
          order.push(n.id);
          walk(n.children);
        }
      };
      walk(content.folders);
      return this.orderedSelectedFolderIds(order);
    },

    // 被剪切项（2026-07-28 复验反馈 8）：cut 期间卡片半透明；粘贴成功 / 被新 copy 覆盖 /
    // 清空后 clipboard 变化即自动恢复
    isCut(key: string): boolean {
      const c = this.clipboard;
      return !!c && c.action === "cut" && c.items.some((it) => keyOf(it) === key);
    },

    // 被剪切文件夹（3.4，2026-08-18）：cut 期间树 item / 文件夹·档案卡片半透明（--opacity-cut），同素材口径
    isFolderCut(id: string): boolean {
      const c = this.clipboard;
      return !!c && c.action === "cut" && (c.folders?.includes(id) ?? false);
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
      // 工作空间磁盘实况选中键（PRD 02 步骤 1）存活以 ws store 已缓存条目为准——
      // 合成项不在 content.placements，不补判会被库 refresh 连带清掉；
      // 2026-09-26 起目录键同样存活（ws 文件夹卡片 / 树行选中入素材域）
      const ws = useWsStore();
      for (const entries of ws.dirs.values()) {
        for (const e of entries) {
          valid.add(`ws:${e.relPath}|ws:${e.relPath.split("/").slice(0, -1).join("/")}`);
        }
      }
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
