// 工作空间 store（PRD 02）：根信息 + 按 relPath 的目录条目缓存（Map，懒加载）+
// 合成 PlacementView 供给 content.placementsFor + 步骤 2 基础整理（新建文件夹 / 重命名 /
// 回收站删除 / 移动 / 工作空间内复制·剪切·粘贴，命令返回即局部重扫刷新）。磁盘实况语义：
// 无标签 / 评分 / 排序等附加数据，所见即所得。
// 合成项铁律：assetId = `ws:${relPath}` 为合成假 id，**任何场景不得发给后端 assetId 类命令**——
// 消费方按本文件导出的 isWsPlacement 判定走按路径分支或直接禁用。
import { defineStore } from "pinia";
import {
  createWsFolder,
  deleteWsEntries,
  getWorkspaceRoot,
  listWorkspaceDir,
  moveWsEntries,
  pasteWsEntries,
  renameWsEntry,
  requestWorkspaceThumbs,
  type WsEntry,
  type WsRootInfo,
  type WsThumbReady,
} from "../lib/wsOps";
import type { PlacementView } from "./content";

// 目录拉取竞态守卫：同一 rel 在途请求共享一趟（fs-changed 连发 / 树展开与视口加载并发常见）
const inFlight = new Map<string, Promise<void>>();
let rootPromise: Promise<void> | null = null;

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
    // 工作空间内剪贴板（PRD 02 步骤 2：复制·剪切·粘贴 = 纯磁盘操作，与系统剪贴板无关）：
    // rel 清单 + cut 语义；剪切粘贴成功即清空
    clip: null as { rels: string[]; cut: boolean } | null,
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
            if (r) this.requestMissingThumbs();
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
    // fs-changed 去抖后调用：重扫全部已加载目录（loadDir 内含缩略图补请求）
    async rescanLoaded(): Promise<void> {
      await Promise.all(Object.keys(this.loaded).map((rel) => this.loadDir(rel)));
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
    },
    // 新建文件夹：默认名「新建文件夹」，本地按当前目录缓存避让重名（与后端 unique_target 同形）
    async createFolder(rel: string): Promise<void> {
      const names = new Set((this.dirs.get(rel) ?? []).map((e) => e.name));
      let name = "新建文件夹";
      for (let n = 2; names.has(name); n += 1) name = `新建文件夹 (${n})`;
      await createWsFolder(rel, name);
      await this.refreshAround([], rel);
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
    // 目录粘贴 = 无操作，防 unique_target 造出 "名 (2)" 副本）
    async move(rels: string[], destRel: string): Promise<void> {
      const targets = rels.filter((r) => (r.includes("/") ? r.slice(0, r.lastIndexOf("/")) : "") !== destRel);
      if (targets.length === 0) return;
      await moveWsEntries(targets, destRel);
      await this.refreshAround(targets, destRel);
    },
    // 复制 / 剪切入剪贴板（rels = 相对根路径；粘贴到自身所在目录 = 无操作由调用方判）
    setClip(rels: string[], cut: boolean): void {
      this.clip = rels.length > 0 ? { rels, cut } : null;
    },
    // 粘贴进 destRel；剪切成功后清空剪贴板（一次性语义）
    async paste(destRel: string): Promise<void> {
      const clip = this.clip;
      if (!clip) return;
      const targets = clip.rels.filter((r) => {
        const parent = r.includes("/") ? r.slice(0, r.lastIndexOf("/")) : "";
        return parent !== destRel;
      });
      if (targets.length === 0) return;
      await pasteWsEntries(targets, destRel, clip.cut);
      await this.refreshAround(targets, destRel);
      if (clip.cut) this.clip = null;
    },
    // 对已加载目录里 thumbPath 为 null 的文件发后台生成请求（生成完成经 workspace-thumbs-ready 回填）
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
    // 缩略图批次回填：absPath 匹配已缓存条目 → 换新数组触发响应式（未加载目录的条目忽略，
    // 下次 loadDir 自然命中新鲜缓存）
    patchThumbs(items: WsThumbReady[]): void {
      if (items.length === 0) return;
      const byAbs = new Map(items.map((it) => [it.absPath, it.thumbPath]));
      for (const [rel, entries] of this.dirs) {
        if (!entries.some((e) => !e.isDir && e.thumbPath === null)) continue;
        const next = entries.map((e) => {
          const abs = e.isDir ? null : this.absPath(e.relPath);
          const thumb = abs !== null ? byAbs.get(abs) : undefined;
          // 本事件只载内容缩略图（格式图标在 list 时已就位）——回填即 asset 来源
          return thumb !== undefined ? { ...e, thumbPath: thumb, thumbKind: "asset" as const } : e;
        });
        this.dirs.set(rel, next);
      }
    },
  },
});

// 磁盘实况条目 → 合成 PlacementView（content.ts:15-42 形状；缺的真实数据给安全缺省值）：
// id/assetId 合成 = `ws:${relPath}`（假 id，禁发 assetId 类命令）；folderId = `ws:${所在目录 rel}`
//（视口 key 同口径，根级文件 = "ws:"）；thumbW/H 恒 null（布局回退链 thumbW→w，图片条目带
// 后端探测的真实宽高 w/h（2026-09-17 实机定：卡片按真实比例自适应，禁填充裁切；非图片
// 无尺寸回退 300 方盒）；modifiedAt/importedAt 由 mtimeMs
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
    wsPath: rootPath !== null ? (e.relPath === "" ? rootPath : `${rootPath}\\${e.relPath.replace(/\//g, "\\")}`) : null,
  };
}
