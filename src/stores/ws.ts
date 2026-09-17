// 工作空间 store（PRD 02 步骤 1 只读底座）：根信息 + 按 relPath 的目录条目缓存（Map，懒加载）+
// 合成 PlacementView 供给 content.placementsFor。磁盘实况语义：无标签 / 评分 / 排序等附加数据，
// 所见即所得；写操作（新建 / 重命名 / 删除 / 拖拽移动）归步骤 2，本模块一概不做。
// 合成项铁律：assetId = `ws:${relPath}` 为合成假 id，**任何场景不得发给后端 assetId 类命令**——
// 消费方按本文件导出的 isWsPlacement 判定走按路径分支或直接禁用。
import { defineStore } from "pinia";
import { getWorkspaceRoot, listWorkspaceDir, requestWorkspaceThumbs, type WsEntry, type WsRootInfo, type WsThumbReady } from "../lib/wsOps";
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
          return thumb !== undefined ? { ...e, thumbPath: thumb } : e;
        });
        this.dirs.set(rel, next);
      }
    },
  },
});

// 磁盘实况条目 → 合成 PlacementView（content.ts:15-42 形状；缺的真实数据给安全缺省值）：
// id/assetId 合成 = `ws:${relPath}`（假 id，禁发 assetId 类命令）；folderId = `ws:${所在目录 rel}`
//（视口 key 同口径，根级文件 = "ws:"）；thumbW/H 与 w/h 恒 null（布局回退 300 方盒，缩略图就绪
// 后比例跟随缺失——磁盘实况无 metadata 可挂，步骤 1 接受方盒）；modifiedAt/importedAt 由 mtimeMs
// 转 RFC3339（? v= URL 版本戳 + 排序键）；frameCount/animated 恒 null/false（GIF 预览走直渲源
// 原生循环）；wsPath = 源文件绝对路径（预览按路径分支与卡片悬停判定用，库 Placement 无此字段）
function wsEntryToPlacement(e: WsEntry, parentRel: string, rootPath: string | null): PlacementView {
  const iso = e.mtimeMs > 0 ? new Date(e.mtimeMs).toISOString() : "";
  return {
    assetId: `${WS_ID_PREFIX}${e.relPath}`,
    folderId: `${WS_ID_PREFIX}${parentRel}`,
    name: e.name,
    w: null,
    h: null,
    thumbW: null,
    thumbH: null,
    ext: e.ext,
    thumbPath: e.thumbPath,
    thumbKind: e.thumbPath ? "asset" : null,
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
