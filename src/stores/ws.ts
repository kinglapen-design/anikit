// 工作空间 store（PRD 02）：根信息 + 按 relPath 的目录条目缓存（Map，懒加载）+
// 合成 PlacementView 供给 content.placementsFor + 步骤 2 基础整理（新建文件夹 / 重命名 /
// 回收站删除 / 移动 / 工作空间内复制·剪切·粘贴，命令返回即局部重扫刷新）。磁盘实况语义：
// 无标签 / 评分 / 排序等附加数据，所见即所得。
// 合成项铁律：assetId = `ws:${relPath}` 为合成假 id，**任何场景不得发给后端 assetId 类命令**——
// 消费方按本文件导出的 isWsPlacement 判定走按路径分支或直接禁用。
import { defineStore } from "pinia";
import {
  addWsLink,
  createWsFolder,
  deleteWsEntries,
  duplicateWsEntries,
  getClipboardSeq,
  getWorkspaceRoot,
  listWorkspaceDir,
  listWsPinned,
  moveWsEntries,
  pasteWsEntries,
  pinWsEntry,
  removeWsLink,
  renameWsEntry,
  requestWorkspaceThumbs,
  retargetWsLink,
  unpinWsEntry,
  wsClipboardFiles,
  wsImportPaths,
  wsPasteSystem,
  wsWriteClipboard,
  type WsEntry,
  type WsRootInfo,
  type WsThumbReady,
} from "../lib/wsOps";
import { useToastStore } from "./toast";
import type { PlacementView } from "./content";

// 目录拉取竞态守卫：同一 rel 在途请求共享一趟（fs-changed 连发 / 树展开与视口加载并发常见）
const inFlight = new Map<string, Promise<void>>();
let rootPromise: Promise<void> | null = null;

// 视口优先请求合批（2026-09-26 ws 缩略图性能优化 C）：可见卡上报即收集，120ms 一拍整批
// priority=true 插队队首（固定节拍而非去抖——连续滚动时去抖永不到点，请求会被无限推迟）
const visiblePending = new Set<string>();
let visibleTimer: ReturnType<typeof setTimeout> | undefined;

// ws 合成 PlacementView 的 assetId / folderId 前缀（viewKey 同口径："ws:" = 根，"ws:a/b" = relPath）
export const WS_ID_PREFIX = "ws:";

/** 磁盘实况条目是否 ws 合成项（禁发 assetId 类命令的统一判定） */
export function isWsPlacement(p: Pick<PlacementView, "assetId"> | null | undefined): boolean {
  return !!p && p.assetId.startsWith(WS_ID_PREFIX);
}

export const useWsStore = defineStore("ws", {
  state: () => ({
    // 根信息（workspace_root；null = 未开空间或尚未拉取）
    root: null as WsRootInfo | null,
    // 已加载目录条目缓存：relPath（"" = 根）→ 条目（含文件与目录；树取 isDir 子集，网格取文件）
    dirs: new Map<string, WsEntry[]>(),
    // 加载完成标记（含加载结果为空；未加载目录不在其中——树据此判定展开钮形态）
    loaded: {} as Record<string, boolean>,
    // 工作空间内剪贴板（PRD 02 步骤 2：复制·剪切·粘贴 = 磁盘操作；2026-10-01 起同步写系统
    // 剪贴板，Explorer 可接力）：rel 清单 + cut 语义；剪切粘贴成功即清空
    clip: null as { rels: string[]; cut: boolean } | null,
    // 系统剪贴板写入完成时刻的序号（2026-10-01 粘贴仲裁基准，库 selection.clipboardSeq 同构）：
    // 「先写后记」防竞态——粘贴时序号未变 = 自己刚写的 → 内部粘贴；变了 = 外部覆盖 → 系统剪贴板粘贴
    clipSeq: null as number | null,
    // 系统剪贴板是否含文件（菜单「粘贴」出现条件 = clip 非空 || systemClipHasFiles；
    // 右键开 ws 菜单前 refreshSystemClip 刷新）
    systemClipHasFiles: false,
    // 步骤 3 常用固定：rel 清单（顺序 = 固定先后；持久化于 .anikit/workspace.json，
    // 读取时后端静默剔除失效条目——改名 / 移动 / 删除 / 非目录）
    pinned: [] as string[],
  }),
  getters: {
    entries: (state) => (rel: string): WsEntry[] => state.dirs.get(rel) ?? [],
    subDirs: (state) => (rel: string): WsEntry[] => (state.dirs.get(rel) ?? []).filter((e) => e.isDir),
    loadedOk: (state) => (rel: string): boolean => !!state.loaded[rel],
    // 目录直接文件数（树行数量；未加载目录不显示）
    fileCount: (state) => (rel: string): number =>
      (state.dirs.get(rel) ?? []).filter((e) => !e.isDir).length,
    // 视口网格数据：文件条目合成 PlacementView（目录由侧栏树导航，不进网格）
    placementsFor(state): (rel: string) => PlacementView[] {
      return (rel: string): PlacementView[] =>
        (state.dirs.get(rel) ?? [])
          .filter((e) => !e.isDir)
          .map((e) => wsEntryToPlacement(e, rel, state.root?.rootPath ?? null));
    },
    // 剪切集中项淡化（2026-10-01，--opacity-cut 库同口径）：key = ws 选择域键
    //（`ws:{rel}|ws:{所在目录rel}`）；clip.cut 且 rels 含该项 rel 即淡化
    isCutKey: (state) => (key: string): boolean => {
      const c = state.clip;
      if (!c?.cut || !key.startsWith(WS_ID_PREFIX)) return false;
      const rel = key.slice(WS_ID_PREFIX.length, key.indexOf("|"));
      return c.rels.includes(rel);
    },
    // 链接门查询（junction 方向 b，2026-10-02；菜单「重新指向…/移除链接」出现条件与 pin 行徽章用）：
    // 条目的联接态存在父目录条目列表里——父目录未加载 / 条目缺失 = null（调用方按非链接降级）；
    // 字段缺省（旧缓存）按 false
    linkState: (state) => (rel: string): { isLink: boolean; linkOk: boolean } | null => {
      const parent = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
      const name = rel.includes("/") ? rel.slice(rel.lastIndexOf("/") + 1) : rel;
      const e = state.dirs.get(parent)?.find((x) => x.name === name);
      return e ? { isLink: !!e.isLink, linkOk: !!e.linkOk } : null;
    },
  },
  actions: {
    // 根信息拉取（幂等；未开空间恒 null）。**null 结果不缓存**：启动竞态——initWsListeners 随
    // App onMounted 即发，彼时后端空间常未开（workspace_root 返 null），缓存住则整会话
    // absPath 恒 null：缩略图永不请求（卡片全空）+ wsPath 恒 null（reveal 无反应）——
    // 2026-09-17 实机两报同根。root 迟到时补发此前因 root 缺失未请求的缩略图
    async ensureRoot(): Promise<void> {
      if (this.root) return;
      if (!rootPromise) {
        rootPromise = getWorkspaceRoot()
          .then((r) => {
            this.root = r;
            if (r) {
              this.requestMissingThumbs();
              void this.loadPinned();
            }
          })
          .catch((e) => {
            console.warn("工作空间根信息拉取失败（静默降级）：", e);
          })
          .finally(() => {
            rootPromise = null;
          });
      }
      await rootPromise;
    },
    // rel 解析为绝对路径：rootPath + rel（后端 rel 为 / 分隔，Windows 下拼 \ ——convertFileSrc 与
    // list_workspace_dir 同源，盘符与分隔符随系统走；root 未就绪返回 null）
    absPath(rel: string): string | null {
      const root = this.root?.rootPath;
      if (!root) return null;
      return rel === "" ? root : `${root}\\${rel.replace(/\//g, "\\")}`;
    },
    // 加载目录（竞态守卫；未开空间 / 目录缺失 → 空列表入缓存，与后端降级口径一致）；
    // 完成后对本目录补请求缺失缩略图（后台生成，经 workspace-thumbs-ready 回填）——
    // 首开示图 / fs-changed 重扫 / 手动刷新全走本路径，缩略图自补全覆盖
    async loadDir(rel: string): Promise<void> {
      await this.ensureRoot();
      const pending = inFlight.get(rel);
      if (pending) return await pending;
      const p = (async () => {
        try {
          const entries = await listWorkspaceDir(rel);
          this.dirs.set(rel, entries);
        } catch (e) {
          console.warn(`工作空间目录刷新失败（${rel}，静默降级）：`, e);
          this.dirs.set(rel, []);
        } finally {
          this.loaded[rel] = true;
          this.requestMissingThumbs(rel);
        }
      })();
      inFlight.set(rel, p);
      try {
        await p;
      } finally {
        inFlight.delete(rel);
      }
    },
    // fs-changed 去抖后全量重扫（仅负载 null 的形态异常兜底用；loadDir 内含缩略图补请求）
    async rescanLoaded(): Promise<void> {
      await Promise.all(Object.keys(this.loaded).map((rel) => this.loadDir(rel)));
    },
    // fs-changed 精准重扫（2026-10-01 性能优化①）：只重扫事件涉及的已加载目录——
    // 未加载目录无可视呈现，下次展开 / 进视口时 loadDir 自然取新
    async rescanDirs(rels: string[]): Promise<void> {
      await Promise.all(rels.filter((rel) => this.loaded[rel]).map((rel) => this.loadDir(rel)));
    },
    // 视口挂载入口（2026-10-01 性能优化③）：已加载目录跳过全量重读——加载后由 fs-changed
    // 精准重扫维持新鲜（监听盲区以右键「刷新」refreshDir 兜底）；未加载才走 loadDir 首发
    async ensureDir(rel: string): Promise<void> {
      if (this.loaded[rel]) return;
      await this.loadDir(rel);
    },
    // 手动刷新兜底（PRD 02 定案：右键「刷新」）：全量重扫当前目录
    async refreshDir(rel: string): Promise<void> {
      await this.loadDir(rel);
    },
    // —— 步骤 2 写操作（PRD 02：命令返回即直接更新 UI，watcher 回声后端消重）——
    // 写操作后的局部重扫：源父目录 + 源子树内已加载目录 + 目标目录（rel 以移动 / 重命名 /
    // 删除路径为锚，全部已加载键中匹配；loaded 目录数 = 展开树 + 开过视口，量级小）
    async refreshAround(srcRels: string[], destRel?: string): Promise<void> {
      const affected = new Set<string>();
      for (const rel of srcRels) {
        const parent = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
        affected.add(parent);
        for (const k of Object.keys(this.loaded)) {
          if (k === rel || k.startsWith(`${rel}/`)) affected.add(k);
        }
      }
      if (destRel !== undefined) affected.add(destRel);
      await Promise.all([...affected].map((rel) => this.loadDir(rel)));
      // 写操作可能殃及固定条目（改名 / 移动 / 删除）——后端读取即静默剔除回写，重取对齐
      void this.loadPinned();
    },
    // 新建文件夹：默认名「新建文件夹」，本地按当前目录缓存避让重名（库 uniqueFolderName 同形，
    // 尾缀 -n；原括号式 " (n)" 随 2026-09-23 序号定案废除）；返回实际创建名（调用方接行内命名态用）
    async createFolder(rel: string): Promise<string> {
      const names = new Set((this.dirs.get(rel) ?? []).map((e) => e.name));
      let name = "新建文件夹";
      for (let n = 2; names.has(name); n += 1) name = `新建文件夹-${n}`;
      await createWsFolder(rel, name);
      await this.refreshAround([], rel);
      return name;
    },
    // 重命名（文件与文件夹同径）：空 / 不变不动；成功重扫源父目录与源子树（目录改名殃及子路径）
    async rename(rel: string, newName: string): Promise<void> {
      const oldName = rel.includes("/") ? rel.slice(rel.lastIndexOf("/") + 1) : rel;
      if (!newName || newName === oldName) return;
      await renameWsEntry(rel, newName);
      await this.refreshAround([rel]);
    },
    // 删除到系统回收站（多路径一次调用）
    async removeEntries(rels: string[]): Promise<void> {
      if (rels.length === 0) return;
      await deleteWsEntries(rels);
      await this.refreshAround(rels);
    },
    // 移动（拖拽 / 剪切粘贴）：目标 = 自身所在目录的条目剔除（拖拽落到当前目录行 / 已剪切所在
    // 目录粘贴 = 无操作，防 unique_target 造出 "名_2" 副本）
    async move(rels: string[], destRel: string): Promise<void> {
      const targets = rels.filter((r) => (r.includes("/") ? r.slice(0, r.lastIndexOf("/")) : "") !== destRel);
      if (targets.length === 0) return;
      await moveWsEntries(targets, destRel);
      await this.refreshAround(targets, destRel);
    },
    // 复制 / 剪切入剪贴板（rels = 相对根路径；粘贴到自身所在目录 = 无操作由调用方判）。
    // 2026-10-01 PRD 02-13：写内部剪贴板的同时同步写系统剪贴板（CF_HDROP，Explorer 可直接
    // 粘贴接力；复制附位图 / 剪切附 DropEffect=MOVE，库 2026-09-17 同口径）——fire-and-forget，
    // 「先写后记」（库 placementOps 2026-09-19 竞态根治同模式）：写完再记序号作粘贴仲裁基准
    setClip(rels: string[], cut: boolean): void {
      this.clip = rels.length > 0 ? { rels, cut } : null;
      if (rels.length === 0) return;
      void (async () => {
        try {
          await wsWriteClipboard(rels, cut);
          this.clipSeq = await getClipboardSeq();
        } catch (e) {
          console.debug("ws 系统剪贴板写入不可用：", e);
        }
      })();
    },
    // 粘贴进 destRel（2026-10-01 仲裁，库 2026-09-17 序号机制同构）：内部剪贴板非空且序号未变
    //（自己刚复制 / 剪切的；序号读取失败 / 未记录同库口径保持内部优先）→ 内部粘贴
    //（剪切成功清 clip）；序号已变（外部覆盖）或内部空 → 系统剪贴板文件粘进目标目录——
    // 复制来源恒复制 / 剪切来源 = 移动；无文件静默，源全失效 toast
    async paste(destRel: string): Promise<void> {
      const seq = await getClipboardSeq().catch(() => -1);
      const clip = this.clip;
      if (clip && (this.clipSeq === null || seq < 0 || seq === this.clipSeq)) {
        const targets = clip.rels.filter((r) => {
          const parent = r.includes("/") ? r.slice(0, r.lastIndexOf("/")) : "";
          return parent !== destRel;
        });
        if (targets.length === 0) return;
        await pasteWsEntries(targets, destRel, clip.cut);
        await this.refreshAround(targets, destRel);
        if (clip.cut) this.clip = null;
        return;
      }
      const r = await wsPasteSystem(destRel);
      if (!r.hadFiles) return; // 系统剪贴板无文件属常态，静默
      if (r.pasted.length === 0) {
        useToastStore().notify({ message: "剪贴板中的文件已不存在", icon: "fail" });
        return;
      }
      await this.refreshAround(r.pasted, destRel);
    },
    // 系统剪贴板含文件态刷新（右键开 ws 菜单前调，菜单「粘贴」出现条件用）；失败静默保持原值
    async refreshSystemClip(): Promise<void> {
      try {
        const r = await wsClipboardFiles();
        this.systemClipHasFiles = r.paths.length > 0;
      } catch {
        /* 静默：读取失败保持原值，菜单可见性降级 */
      }
    },
    // Explorer 拖入（2026-10-01 PRD 02-13：拖入一律复制，非破坏口径）：外部绝对路径复制进
    // destRel；成功后按既有方式局部重扫（目标目录 + 落点目录子树内已加载目录）
    async importPaths(absPaths: string[], destRel: string): Promise<void> {
      const created = await wsImportPaths(absPaths, destRel);
      if (created.length > 0) await this.refreshAround(created, destRel);
    },
    // 创建副本（库「创建副本」Ctrl+D 移植，2026-10-01）：同目录复制，后端重名 _n 规避；
    // 文件与文件夹混合整批受理（同其余 ws 写操作口径）
    async duplicate(rels: string[]): Promise<void> {
      if (rels.length === 0) return;
      await duplicateWsEntries(rels);
      await this.refreshAround(rels);
    },
    // —— junction 挂载（方向 b：外部文件夹挂进 WS 子目录，2026-10-02）——
    // 挂载外部文件夹：targetPath 以 junction 挂进 destRel（挂载名 = 目标目录名，重名后端 _n）；
    // 返回挂载点 rel——父级 = destRel 随 refreshAround 一并重扫；校验失败中文错向上抛（菜单层 toast）
    async addLink(destRel: string, targetPath: string): Promise<void> {
      const rel = await addWsLink(destRel, targetPath);
      await this.refreshAround([rel]);
    },
    // 解除挂载：链接本体进系统回收站（只删「门」不动目标）；断链同径可移除
    async removeLink(rel: string): Promise<void> {
      await removeWsLink(rel);
      await this.refreshAround([rel]);
    },
    // 改挂目标（断链送修同径）：链接重指向 newTarget——子树内容整体换，
    // refreshAround 重扫父级 + 子树内已加载目录
    async retargetLink(rel: string, newTarget: string): Promise<void> {
      await retargetWsLink(rel, newTarget);
      await this.refreshAround([rel]);
    },
    // —— 步骤 3 常用固定（清单以后端返回为准覆写：固定顺序 + 失效剔除都由后端权威）——
    // 未开空间容忍空（命令层降级 []）；失败静默（不打扰主流程，下次重试）
    async loadPinned(): Promise<void> {
      try {
        this.pinned = await listWsPinned();
      } catch {
        /* 静默：常用区块缺失不阻塞树 */
      }
    },
    async pin(rel: string): Promise<void> {
      this.pinned = await pinWsEntry(rel);
    },
    async unpin(rel: string): Promise<void> {
      this.pinned = await unpinWsEntry(rel);
    },
    // 对已加载目录里 thumbPath 为 null 的文件发后台生成请求（生成完成经 workspace-thumbs-ready 回填）。
    // 全量排队列尾部（后端 worker 池去重，重复发起零成本）；解码失败条目经失败记忆落回格式图标
    // （thumbPath 非 null）后天然滤出本清单，不再重复请求（2026-09-26 性能优化）
    requestMissingThumbs(onlyRel?: string): void {
      const rels = onlyRel !== undefined ? [onlyRel] : Object.keys(this.loaded);
      const paths: string[] = [];
      for (const rel of rels) {
        for (const e of this.dirs.get(rel) ?? []) {
          if (!e.isDir && e.thumbPath === null) {
            const abs = this.absPath(e.relPath);
            if (abs) paths.push(abs);
          }
        }
      }
      if (paths.length > 0) void requestWorkspaceThumbs(paths).catch(() => {});
    },
    // 视口内（含近 buffer 区）缺失项优先请求：卡片组件按可见性 / 挂载上报，合批一拍
    // priority=true 插队队首——恢复滚动位置落在目录中部时，可见卡不等前面全部生成完
    requestVisibleThumbs(absPaths: string[]): void {
      for (const p of absPaths) visiblePending.add(p);
      if (visibleTimer === undefined) {
        visibleTimer = setTimeout(() => {
          visibleTimer = undefined;
          const batch = [...visiblePending];
          visiblePending.clear();
          if (batch.length > 0) void requestWorkspaceThumbs(batch, true).catch(() => {});
        }, 120);
      }
    },
    // 缩略图批次回填：absPath 匹配已缓存条目 → 换新数组触发响应式（未加载目录的条目忽略，
    // 下次 loadDir 自然命中新鲜缓存）；负载带 thumbKind（2026-09-26）：asset = 内容缩略图——
    // w/h 为空（视频/音频等非图片）时补上，布局回退链 thumbW→w 据此由 300 方盒恢复真实比例
    //（2026-09-26 视频卡裁切修复；图片条目 w/h 已是源魔数探测值，不覆盖）；
    // formatIcon = 解码失败条目落回的系统格式图标——只换图标与来源，不动 w/h
    //（图标见方不代表源比例，落回条目维持方盒 / 已探测比例）
    patchThumbs(items: WsThumbReady[]): void {
      if (items.length === 0) return;
      const byAbs = new Map(items.map((it) => [it.absPath, it]));
      for (const [rel, entries] of this.dirs) {
        if (!entries.some((e) => !e.isDir && e.thumbPath === null)) continue;
        const next = entries.map((e) => {
          const abs = e.isDir ? null : this.absPath(e.relPath);
          const ready = abs !== null ? byAbs.get(abs) : undefined;
          if (ready === undefined) return e;
          const dims = e.w === null && ready.thumbKind === "asset" ? { w: ready.w, h: ready.h } : {};
          return { ...e, ...dims, thumbPath: ready.thumbPath, thumbKind: ready.thumbKind };
        });
        this.dirs.set(rel, next);
      }
    },
  },
});

// 磁盘实况条目 → 合成 PlacementView（content.ts:15-42 形状；缺的真实数据给安全缺省值）：
// id/assetId 合成 = `ws:${relPath}`（假 id，禁发 assetId 类命令）；folderId = `ws:${所在目录 rel}`
//（视口 key 同口径，根级文件 = "ws:"）；thumbW/H 恒 null（布局回退链 thumbW→w——图片条目带
// 后端魔数探测的真实宽高 w/h（2026-09-17 实机定：卡片按真实比例自适应，禁填充裁切），非图片
//（视频/音频/字体等）由后端读缓存缩略图文件头回填、或 thumbs-ready 事件随生成结果回填
//（2026-09-26：缩略图与源同比例，布局只用比例）；缩略图未就绪仍无尺寸时回退 300 方盒，就绪后
// 自动重排恢复）；modifiedAt/importedAt 由 mtimeMs
// 转 RFC3339（? v= URL 版本戳 + 排序键）；frameCount/animated 恒 null/false（GIF 预览走直渲源
// 原生循环）；wsPath = 源文件绝对路径（预览按路径分支与卡片悬停判定用，库 Placement 无此字段）
function wsEntryToPlacement(e: WsEntry, parentRel: string, rootPath: string | null): PlacementView {
  const iso = e.mtimeMs > 0 ? new Date(e.mtimeMs).toISOString() : "";
  return {
    assetId: `${WS_ID_PREFIX}${e.relPath}`,
    folderId: `${WS_ID_PREFIX}${parentRel}`,
    name: e.name,
    w: e.w,
    h: e.h,
    thumbW: null,
    thumbH: null,
    ext: e.ext,
    thumbPath: e.thumbPath,
    thumbKind: e.thumbKind,
    origin: true,
    frameCount: null,
    animated: false,
    duration: null,
    bpm: null,
    importedFrom: null,
    size: e.size,
    createdAt: null,
    modifiedAt: iso,
    importedAt: iso,
    noteCount: null,
    thumbFrame: null,
    fps: null,
    hidden: false,
    tags: [], // 工作空间无标签（PRD 02：磁盘实况无附加数据）
    wsPath: rootPath !== null ? (e.relPath === "" ? rootPath : `${rootPath}\\${e.relPath.replace(/\//g, "\\")}`) : null,
  };
}
