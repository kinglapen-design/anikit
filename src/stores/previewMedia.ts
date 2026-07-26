// 音视频播放（5.2）：<video> 元素模块级持有（非响应式，音频也复用 video 元素免双实现），
// 播放状态（playing/currentTime/duration/volume/rate/loop）经元素事件同步进 store。
// get_media_source 解析播放源（WebView2 不可直播格式后端已转码 cache/tmp，转码期 transcoding 加载态）；
// 解析失败降级：preview.kind 落 other + 中文报错文案（preview.error 画面提示）。
import { defineStore } from "pinia";
import { nextTick } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useContentStore } from "./content";
import { usePreviewStore } from "./preview";

interface MediaSource {
  path: string;
  transcoded: boolean;
}

// 媒体元素单例（模块级；store 状态响应式、元素本身不进响应式系统）
let el: HTMLVideoElement | null = null;

function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "该格式暂时无法播放";
}

export const usePreviewMediaStore = defineStore("previewMedia", {
  state: () => ({
    kind: null as "audio" | "video" | null,
    srcPath: null as string | null, // 播放源绝对路径（源文件或转码产物）
    transcoding: false, // get_media_source 解析/转码中（"转码中…"加载态）
    playing: false,
    currentTime: 0,
    duration: 0,
    volume: 1,
    muted: false,
    rate: 1,
    loop: false,
  }),
  actions: {
    // 取媒体元素（不存在则创建并接事件同步；PreviewPanel 把它挂进画面区）
    getEl(): HTMLVideoElement {
      if (el) return el;
      el = document.createElement("video");
      el.preload = "auto";
      el.controls = false;
      el.style.cssText = "display:block;width:100%;height:100%;object-fit:contain;";
      el.addEventListener("play", () => (this.playing = true));
      el.addEventListener("pause", () => (this.playing = false));
      el.addEventListener("ended", () => (this.playing = false));
      el.addEventListener("timeupdate", () => (this.currentTime = el?.currentTime ?? 0));
      el.addEventListener("durationchange", () => (this.duration = el?.duration || 0));
      el.addEventListener("loadedmetadata", () => {
        this.duration = el?.duration || 0;
        // 视频尺寸喂给查看器（fit/缩放数学与图片同一套）
        if (this.kind === "video" && el) usePreviewStore().onImageLoad(el.videoWidth, el.videoHeight);
      });
      return el;
    },
    // 解析播放源并装载（转码可能数十秒，期间 transcoding=true）；
    // 装载完成即自动播放（2026-07-26 用户定：打开预览的目的就是播放，同动图/序列帧口径）
    async load(assetId: string, kind: "audio" | "video"): Promise<void> {
      const preview = usePreviewStore();
      this.kind = kind;
      this.transcoding = true;
      try {
        const r = await invoke<MediaSource>("get_media_source", { assetId });
        if (preview.current?.assetId !== assetId) return; // await 期间已切走
        this.srcPath = r.path;
        const v = this.getEl();
        v.src = convertFileSrc(r.path);
        v.loop = this.loop;
        this.play();
      } catch (e) {
        if (preview.current?.assetId !== assetId) return;
        console.error("音视频源解析失败：", e);
        preview.kind = "other"; // 降级：格式图标/缩略图静态 + 报错文案
        preview.error = errText(e);
      } finally {
        this.transcoding = false;
      }
    },
    // 释放（切素材/退出预览）：暂停并卸载源，状态归零（音量/静音保留用户设置）
    release(): void {
      if (el) {
        el.pause();
        el.removeAttribute("src");
        el.load();
      }
      this.kind = null;
      this.srcPath = null;
      this.transcoding = false;
      this.playing = false;
      this.currentTime = 0;
      this.duration = 0;
      this.rate = 1;
      this.loop = false;
    },
    play(): void {
      const v = this.getEl();
      if (v.src) void v.play().catch((e) => console.warn("播放失败：", e));
    },
    pause(): void {
      el?.pause();
    },
    togglePlay(): void {
      if (this.playing) this.pause();
      else this.play();
    },
    seek(t: number): void {
      const v = this.getEl();
      const max = this.duration > 0 ? this.duration : Number.MAX_SAFE_INTEGER;
      v.currentTime = Math.min(max, Math.max(0, t));
      this.currentTime = v.currentTime;
    },
    seekBy(delta: number): void {
      this.seek(this.currentTime + delta);
    },
    setVolume(v: number): void {
      const vv = this.getEl();
      vv.volume = Math.min(1, Math.max(0, v));
      this.volume = vv.volume;
      if (vv.volume > 0) {
        vv.muted = false; // 拖音量自动解除静音
        this.muted = false;
      }
    },
    toggleMute(): void {
      const v = this.getEl();
      v.muted = !v.muted;
      this.muted = v.muted;
    },
    setRate(r: number): void {
      this.rate = r;
      this.getEl().playbackRate = r;
    },
    toggleLoop(): void {
      this.loop = !this.loop;
      this.getEl().loop = this.loop;
    },
    // 当前画面设为预览图（5.2 视频）：ffmpeg 截帧走缩略图管线，前端即时换卡片图（同 setAsThumb 技巧）
    async setVideoThumb(): Promise<void> {
      const p = usePreviewStore().current;
      if (!p) return;
      const r = await invoke<{ thumbPath: string }>("set_video_thumb", {
        assetId: p.assetId,
        atSecs: this.currentTime,
      });
      const content = useContentStore();
      const target = content.placements.find(
        (x) => x.assetId === p.assetId && x.folderId === p.folderId,
      );
      if (target) {
        target.thumbPath = null;
        await nextTick();
        target.thumbPath = r.thumbPath;
      }
    },
  },
});
