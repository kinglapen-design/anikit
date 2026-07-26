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

export const usePreviewNotesStore = defineStore("previewNotes", {
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
      this.notes = await invoke<NoteEntry[]>("set_note", { assetId: p.assetId, key, text });
      // noteCount 冗余同步卡片角标（后端已落 metadata，前端按返回全量算条数）
      useContentStore().setNoteCount(p.assetId, this.notes.length);
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
