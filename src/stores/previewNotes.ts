// 逐帧笔记（5.2）：sequence/anim/video 播放条的笔记浮层状态。
// key 约定：序列帧 = 帧名（frame-00001）；**动图 = `f:{帧索引0起}`**（2026-08-20 解包废除，
// 不再依赖帧文件名；存量帧名 key 由后端 get_notes 一次性迁移）；视频 = t:{currentTime.toFixed(3)}；
// set_note 的 text 去空白后为空 = 删除；两条命令统一返回按 key 排序全量，前端直接替换。
// 拆分（2026-09-25 per-panel 会话化）：全局 usePreviewNotesStore 只留共享命令层
//（version / loadNotes / saveNote / getNoteThumb——详情面板在无预览态也消费）；
// 浮层会话态（open/notes、帧 key 解析与跳转）收 buildPreviewNotesStore 工厂——
// 每 dockview panel 一会话，previewNotesSession(panelId) 懒建。
import { defineStore, getActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "./content";
import { previewSession } from "./preview";
import { previewMediaSession } from "./previewMedia";

export interface NoteEntry {
  key: string;
  text: string;
  updatedAt: number;
}

// get_note_thumb 返回：path = 帧画面绝对路径（webview 直渲）；取不到为 null（前端 C4 占位）
export interface NoteThumb {
  path: string | null;
}

// 共享命令层（全局单例）：笔记命令统一封装（DetailNotes 同走，规范第 1 条）
export const usePreviewNotesStore = defineStore("previewNotes", {
  state: () => ({
    // 数据变更版本号（set_note 后 +1）：详情面板笔记区块 watch 它做热刷新（2026-07-26 用户）
    version: 0,
  }),
  actions: {
    // 详情面板拉取（无预览态也查）：失败静默降级 console.warn 返回 null
    async loadNotes(assetId: string): Promise<NoteEntry[] | null> {
      try {
        return await invoke<NoteEntry[]>("get_notes", { assetId });
      } catch (e) {
        console.warn("笔记拉取失败（静默降级）：", e);
        return null;
      }
    },
    // 写笔记统一收尾（set_note 空文本 = 删除）：返回全量替换 + noteCount 同步卡片角标
    //（后端已落 metadata，前端按返回全量算条数）+ 版本号热刷新详情面板笔记区块
    async saveNote(assetId: string, key: string, text: string): Promise<NoteEntry[]> {
      const list = await invoke<NoteEntry[]>("set_note", { assetId, key, text });
      useContentStore().setNoteCount(assetId, list.length);
      this.version += 1;
      return list;
    },
    // 详情面板悬停浮帧画面：序列帧直给帧原图，动图 `f:{索引}` 抽帧，视频 ffmpeg 截帧（未就绪 path=null）
    async getNoteThumb(assetId: string, key: string): Promise<NoteThumb> {
      return await invoke<NoteThumb>("get_note_thumb", { assetId, key });
    },
  },
});

function buildPreviewNotesStore(panelId: string) {
  return defineStore(`previewNotes#${panelId}`, {
    state: () => ({
      open: false, // 浮层开关（笔记按钮激活态）
      notes: [] as NoteEntry[],
    }),
    getters: {
      // 帧条笔记圆点判定（sequence/anim）
      keySet(): Set<string> {
        return new Set(this.notes.map((n) => n.key));
      },
      // 当前帧/当前时间点 key（video 播放中随时间变化；浮层打开即暂停，编辑期稳定）
      currentKey(): string | null {
        const preview = previewSession(panelId);
        if (preview.kind === "anim") {
          return `f:${preview.frame}`;
        }
        if (preview.kind === "sequence") {
          return preview.seq?.frames[preview.frame]?.name ?? null;
        }
        if (preview.kind === "video") {
          return `t:${previewMediaSession(panelId).currentTime.toFixed(3)}`;
        }
        return null;
      },
      currentText(): string {
        const k = this.currentKey;
        return this.notes.find((n) => n.key === k)?.text ?? "";
      },
    },
    actions: {
      async load(): Promise<void> {
        const p = previewSession(panelId).current;
        this.notes = p ? await invoke<NoteEntry[]>("get_notes", { assetId: p.assetId }) : [];
      },
      // 打开浮层即暂停（编辑期 currentKey 稳定，防播放推进冲掉草稿）
      toggle(): void {
        this.open = !this.open;
        if (this.open) {
          previewSession(panelId).pause();
          previewMediaSession(panelId).pause();
          if (this.notes.length === 0) void this.load();
        }
      },
      // 保存走全局命令层 saveNote（全量替换 + noteCount 角标 + version 热刷新链路不变）
      async save(key: string, text: string): Promise<void> {
        const p = previewSession(panelId).current;
        if (!p || !key) return;
        this.notes = await usePreviewNotesStore().saveNote(p.assetId, key, text);
      },
      async remove(key: string): Promise<void> {
        await this.save(key, ""); // 空文本 = 删除（后端约定）
      },
      // 列表点击跳转：sequence/anim 跳帧，video 跳到该时间点
      jump(key: string): void {
        const preview = previewSession(panelId);
        if (preview.kind === "anim") {
          if (key.startsWith("f:")) preview.setFrame(Number(key.slice(2)));
        } else if (preview.kind === "sequence") {
          const i = preview.seq?.frames.findIndex((f) => f.name === key) ?? -1;
          if (i >= 0) preview.setFrame(i);
        } else if (preview.kind === "video" && key.startsWith("t:")) {
          previewMediaSession(panelId).seek(Number(key.slice(2)));
        }
      },
      reset(): void {
        this.open = false;
        this.notes = [];
      },
    },
  });
}

// —— 会话注册表（per-panel 懒建；随 preview 会话联动销毁）——
export type PreviewNotesSession = ReturnType<ReturnType<typeof buildPreviewNotesStore>>;
const sessionCache = new Map<string, PreviewNotesSession>();

// 属主面板的逐帧笔记会话（无则懒建）
export function previewNotesSession(panelId: string): PreviewNotesSession {
  let s = sessionCache.get(panelId);
  if (!s) {
    s = buildPreviewNotesStore(panelId)();
    sessionCache.set(panelId, s);
  }
  return s;
}

// 销毁会话（panel 关闭联动）：dispose 并删缓存
export function dropPreviewNotesSession(panelId: string): void {
  const s = sessionCache.get(panelId);
  if (!s) return;
  s.$dispose();
  sessionCache.delete(panelId);
  // $dispose 不清 pinia state 键（动态 id 只增不减）——顺手删 state 键
  const pinia = getActivePinia();
  if (pinia) delete (pinia.state.value as Record<string, unknown>)[s.$id];
}
