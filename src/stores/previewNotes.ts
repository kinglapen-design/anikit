// 逐帧笔记（5.2）：sequence/anim/video 播放条的笔记浮层状态。
// key 约定：序列帧/动图 = 帧名（frame-00001），视频 = t:{currentTime.toFixed(3)}；
// set_note 的 text 去空白后为空 = 删除；两条命令统一返回按 key 排序全量，前端直接替换。
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "./content";
import { usePreviewStore } from "./preview";
import { usePreviewMediaStore } from "./previewMedia";

export interface NoteEntry {
  key: string;
  text: string;
  updatedAt: number;
}

// get_note_thumb 返回：path = 帧画面绝对路径（webview 直渲）；取不到为 null（前端 C4 占位）
export interface NoteThumb {
  path: string | null;
}

export const usePreviewNotesStore = defineStore("previewNotes", {
  state: () => ({
    open: false, // 浮层开关（笔记按钮激活态）
    notes: [] as NoteEntry[],
    // 数据变更版本号（set_note 后 +1）：详情面板笔记区块 watch 它做热刷新（2026-07-26 用户）
    version: 0,
  }),
  getters: {
    // 帧条笔记圆点判定（sequence/anim）
    keySet(): Set<string> {
      return new Set(this.notes.map((n) => n.key));
    },
    // 当前帧/当前时间点 key（video 播放中随时间变化；浮层打开即暂停，编辑期稳定）
    currentKey(): string | null {
      const preview = usePreviewStore();
      if (preview.kind === "sequence" || preview.kind === "anim") {
        return preview.seq?.frames[preview.frame]?.name ?? null;
      }
      if (preview.kind === "video") {
        return `t:${usePreviewMediaStore().currentTime.toFixed(3)}`;
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
      const p = usePreviewStore().current;
      this.notes = p ? await invoke<NoteEntry[]>("get_notes", { assetId: p.assetId }) : [];
    },
    // 打开浮层即暂停（编辑期 currentKey 稳定，防播放推进冲掉草稿）
    toggle(): void {
      this.open = !this.open;
      if (this.open) {
        usePreviewStore().pause();
        usePreviewMediaStore().pause();
        if (this.notes.length === 0) void this.load();
      }
    },
    async save(key: string, text: string): Promise<void> {
      const p = usePreviewStore().current;
      if (!p || !key) return;
      this.notes = await this.saveNote(p.assetId, key, text);
    },
    // 笔记命令统一封装（DetailNotes 同走，规范第 1 条）——
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
    // 详情面板悬停浮帧画面：序列帧 / 动图直给帧原图，视频 ffmpeg 截帧（未就绪 path=null）
    async getNoteThumb(assetId: string, key: string): Promise<NoteThumb> {
      return await invoke<NoteThumb>("get_note_thumb", { assetId, key });
    },
    async remove(key: string): Promise<void> {
      await this.save(key, ""); // 空文本 = 删除（后端约定）
    },
    // 列表点击跳转：sequence/anim 跳帧，video 跳到该时间点
    jump(key: string): void {
      const preview = usePreviewStore();
      if (preview.kind === "sequence" || preview.kind === "anim") {
        const i = preview.seq?.frames.findIndex((f) => f.name === key) ?? -1;
        if (i >= 0) preview.setFrame(i);
      } else if (preview.kind === "video" && key.startsWith("t:")) {
        usePreviewMediaStore().seek(Number(key.slice(2)));
      }
    },
    reset(): void {
      this.open = false;
      this.notes = [];
    },
  },
});
