// 音视频播放（5.2）：<video> 元素模块级持有（非响应式，音频也复用 video 元素免双实现），
// 播放状态（playing/currentTime/duration/volume/rate/loop）经元素事件同步进 store。
// get_media_source 解析播放源（2026-09-13 直读优先：先直给源路径，播放 error 回退转码
// cache/media 持久产物，转码期 transcoding 加载态）；
// 解析失败降级：preview.kind 落 other + 中文报错文案（preview.error 画面提示）。
import { defineStore } from "pinia";
import { nextTick } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useContentStore } from "./content";
import { usePreviewStore } from "./preview";
import { useToastStore } from "./toast";
import { getMediaSource, getMediaSourceByPath } from "../lib/mediaOps";

// 媒体元素单例（模块级；store 状态响应式、元素本身不进响应式系统）
let el: HTMLVideoElement | null = null;
// transcodingSlow 延迟定时器（300ms 防闪）
let slowTimer: ReturnType<typeof setTimeout> | undefined;
// 直读优先回退用（2026-09-13）：当前装载资产 id + 是否已回退过转码（每素材只回退一次）
let curAssetId: string | null = null;
let transcodeRetried = false;
// 按路径模式（PRD 02 步骤 1 工作空间预览）：当前装载源文件绝对路径 + 转码回退标记
// （curPath 非空 = onSourceError 走按路径回退；与按 assetId 模式互斥，release 两者皆清）
let curPath: string | null = null;
let pathTranscodeRetried = false;
// 直读重试标记（2026-09-24 瞬时错误修复，两模式各一）：error 非"格式不支持"
// （seek / 硬解管线偶发报错）时先同源重载直读重试一次，仍 error 才落转码兜底——
// 一刀切转码会把瞬时错误误诊成格式不可播，白等几十秒软编码（用户实机：拖进度到开头偶发"转码中…"）
let directRetried = false;
let pathDirectRetried = false;

function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "该格式暂时无法播放";
}

export const usePreviewMediaStore = defineStore("previewMedia", {
  state: () => ({
    kind: null as "audio" | "video" | null,
    srcPath: null as string | null, // 播放源绝对路径（源文件或转码产物）
    transcoding: false, // get_media_source 解析/转码中（内部真实状态）
    // UI 显示用（2026-07-26 用户：切换素材时直渲解析一瞬会闪"转码中…"）——
    // transcoding 持续 300ms 以上才置真：真转码才出提示，瞬间往返不闪
    transcodingSlow: false,
    playing: false,
    currentTime: 0,
    // 播放条拖动 scrub 中（2026-08-22 跟手修复）：拖动期间 timeupdate 不回写——
    // UI 以 seek() 乐观目标值为准（跟手），避免被元素实际位置回拽（弹回）；松手恢复同步
    scrubbing: false,
    duration: 0,
    // 视频帧率（5.3 帧化改造，2026-07-28 用户定：帧数优先）：load 时由 PlacementView.fps 传入，
    // 音频 / 未知为 null（帧显示与逐帧步进回退时间口径）；切素材 release 复位
    fps: null as number | null,
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
      el.addEventListener("timeupdate", () => {
        if (!this.scrubbing) this.currentTime = el?.currentTime ?? 0;
      });
      el.addEventListener("durationchange", () => (this.duration = el?.duration || 0));
      // 直读优先的播放错误回退（2026-09-13）：源编码直解不了 → error 后按装载模式路由回退
      // （按 assetId 链 / 工作空间按路径链，PRD 02 步骤 1）
      el.addEventListener("error", () => {
        if (curPath) void this.onPathSourceError();
        else void this.onSourceError();
      });
      el.addEventListener("loadedmetadata", () => {
        this.duration = el?.duration || 0;
        // 视频尺寸喂给查看器（fit/缩放数学与图片同一套）
        if (this.kind === "video" && el) usePreviewStore().onImageLoad(el.videoWidth, el.videoHeight);
      });
      return el;
    },
    // 解析播放源并装载（转码可能数十秒，期间 transcoding=true）；
    // 装载完成即自动播放（2026-07-26 用户定：打开预览的目的就是播放，同动图/序列帧口径）
    async load(assetId: string, kind: "audio" | "video", fps: number | null = null): Promise<void> {
      const preview = usePreviewStore();
      this.kind = kind;
      curAssetId = assetId;
      transcodeRetried = false;
      directRetried = false;
      this.fps = kind === "video" ? fps : null;
      this.transcoding = true;
      this.transcodingSlow = false;
      clearTimeout(slowTimer);
      slowTimer = setTimeout(() => {
        if (this.transcoding) this.transcodingSlow = true;
      }, 300);
      try {
        const r = await getMediaSource(assetId);
        if (preview.current?.assetId !== assetId) return; // await 期间已切走
        // 响应 fps 优先（自愈后的准确值；PlacementView 参数只是开库时的旧快照，
        // 2026-07-28 复验反馈修复：存量视频播放条/笔记立即正确）
        if (kind === "video" && r.fps != null) this.fps = r.fps;
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
        clearTimeout(slowTimer);
        this.transcoding = false;
        this.transcodingSlow = false;
      }
    },
    // 按路径装载（PRD 02 步骤 1 工作空间预览）：get_media_source_by_path 直读优先，
    // error 后 forceTranscode 转码兜底（同 load 口径）；null（非音视频 / 越界）= 降级 other + 报错文案。
    // "已切走"守卫按 current.wsPath 判定（合成项 assetId 为假 id 不可用）
    async loadPath(absPath: string, kind: "audio" | "video", fps: number | null = null): Promise<void> {
      const preview = usePreviewStore();
      this.kind = kind;
      curPath = absPath;
      pathTranscodeRetried = false;
      pathDirectRetried = false;
      this.fps = kind === "video" ? fps : null;
      this.transcoding = true;
      this.transcodingSlow = false;
      clearTimeout(slowTimer);
      slowTimer = setTimeout(() => {
        if (this.transcoding) this.transcodingSlow = true;
      }, 300);
      try {
        const r = await getMediaSourceByPath(absPath);
        if (preview.current?.wsPath !== absPath) return; // await 期间已切走
        if (!r) {
          preview.kind = "other";
          preview.error = "该格式暂时无法播放";
          return;
        }
        if (kind === "video" && r.fps != null) this.fps = r.fps;
        this.srcPath = r.path;
        const v = this.getEl();
        v.src = convertFileSrc(r.path);
        v.loop = this.loop;
        this.play();
      } catch (e) {
        if (preview.current?.wsPath !== absPath) return;
        console.error("工作空间播放源解析失败：", e);
        preview.kind = "other"; // 降级：格式图标/缩略图静态 + 报错文案
        preview.error = errText(e);
      } finally {
        clearTimeout(slowTimer);
        this.transcoding = false;
        this.transcodingSlow = false;
      }
    },
    // 按路径播放错误回退转码（直读优先口径，同 onSourceError）：仅直读源 error 触发一次
    async onPathSourceError(): Promise<void> {
      const preview = usePreviewStore();
      const absPath = curPath;
      if (!absPath || pathTranscodeRetried || !el?.currentSrc) return;
      if (preview.current?.wsPath !== absPath) return;
      // 瞬时错误先直读重试一次（口径见 onSourceError）
      if (el.error?.code !== MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED && !pathDirectRetried) {
        pathDirectRetried = true;
        console.warn(`播放源瞬时错误（code=${el.error?.code ?? "?"}），直读重试：`, absPath);
        this.retryDirect();
        return;
      }
      pathTranscodeRetried = true;
      clearTimeout(slowTimer);
      this.transcoding = true;
      this.transcodingSlow = false;
      slowTimer = setTimeout(() => {
        if (this.transcoding) this.transcodingSlow = true;
      }, 300);
      try {
        const r = await getMediaSourceByPath(absPath, true);
        if (preview.current?.wsPath !== absPath || !el) return;
        if (!r) {
          preview.kind = "other";
          preview.error = "该格式暂时无法播放";
          return;
        }
        if (this.kind === "video" && r.fps != null) this.fps = r.fps;
        this.srcPath = r.path;
        el.src = convertFileSrc(r.path);
        el.loop = this.loop;
        this.play();
      } catch (e) {
        if (preview.current?.wsPath !== absPath) return;
        console.error("工作空间转码兜底失败：", e);
        preview.kind = "other";
        preview.error = errText(e);
      } finally {
        clearTimeout(slowTimer);
        this.transcoding = false;
        this.transcodingSlow = false;
      }
    },
    // 播放错误回退转码（直读优先口径，2026-09-13）：仅直读源 error 触发一次；
    // 转码期复用 transcoding + 300ms 防闪（"转码中…"提示）
    async onSourceError(): Promise<void> {
      const preview = usePreviewStore();
      const id = curAssetId;
      if (!id || transcodeRetried || !el?.currentSrc) return;
      if (preview.current?.assetId !== id) return;
      // 瞬时错误（seek / 硬解管线偶发报错，2026-09-24 用户实机"拖进度到开头偶发转码中"）——
      // error 不分错误码一刀切转码会把瞬时错误误诊成格式不可播，白等整段软编码：
      // 非"格式不支持"（SRC_NOT_SUPPORTED）先同源直读重试一次，仍 error 自然落下方转码兜底；
      // 真不支持的格式首装即报 SRC_NOT_SUPPORTED，直落转码（重试无意义）
      if (el.error?.code !== MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED && !directRetried) {
        directRetried = true;
        console.warn(`播放源瞬时错误（code=${el.error?.code ?? "?"}），直读重试：`, this.srcPath);
        this.retryDirect();
        return;
      }
      transcodeRetried = true;
      clearTimeout(slowTimer);
      this.transcoding = true;
      this.transcodingSlow = false;
      slowTimer = setTimeout(() => {
        if (this.transcoding) this.transcodingSlow = true;
      }, 300);
      try {
        const r = await getMediaSource(id, true);
        if (preview.current?.assetId !== id || !el) return;
        if (this.kind === "video" && r.fps != null) this.fps = r.fps;
        this.srcPath = r.path;
        el.src = convertFileSrc(r.path);
        el.loop = this.loop;
        this.play();
      } catch (e) {
        if (preview.current?.assetId !== id) return;
        console.error("音视频转码兜底失败：", e);
        preview.kind = "other";
        preview.error = errText(e);
      } finally {
        clearTimeout(slowTimer);
        this.transcoding = false;
        this.transcodingSlow = false;
      }
    },
    // 直读重试（瞬时错误恢复，2026-09-24）：src 不变调 load() 清 MediaError 重走管线，
    // loadedmetadata 后恢复原播放位置续播；重试仍 error 由事件链落转码兜底（directRetried 已占位）
    retryDirect(): void {
      const v = el;
      if (!v) return;
      const resume = this.currentTime;
      const onMeta = (): void => {
        v.removeEventListener("loadedmetadata", onMeta);
        if (resume > 0 && resume < (v.duration || 0)) v.currentTime = resume;
        this.play();
      };
      v.addEventListener("loadedmetadata", onMeta);
      v.load();
    },
    // 释放（切素材/退出预览）：暂停并卸载源，状态归零（音量/静音保留用户设置）
    release(): void {
      if (el) {
        el.pause();
        el.removeAttribute("src");
        el.load();
      }
      curAssetId = null;
      transcodeRetried = false;
      curPath = null;
      pathTranscodeRetried = false;
      directRetried = false;
      pathDirectRetried = false;
      this.kind = null;
      this.srcPath = null;
      clearTimeout(slowTimer);
      this.transcoding = false;
      this.transcodingSlow = false;
      this.playing = false;
      this.currentTime = 0;
      this.duration = 0;
      this.fps = null;
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
    // 播放条拖动起止（PlayBarVideo pointerdown/up）：拖动中 timeupdate 不回写；松手对齐真实位置
    setScrubbing(on: boolean): void {
      this.scrubbing = on;
      if (!on && el) this.currentTime = el.currentTime;
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
    // 当前画面设为预览图（5.2 视频）：ffmpeg 截帧走缩略图管线，前端即时换卡片图（同 setAsThumb 技巧）；
    // toast 反馈 + modifiedAt URL 版本戳 + 视图外兜底 refresh（2026-08-20 与动图侧同修复）
    async setVideoThumb(): Promise<void> {
      const p = usePreviewStore().current;
      // ws 合成假 id 禁发 assetId 类命令（PRD 02 步骤 1 只读，cover 钮已由播放条 spec 滤除）
      if (!p || p.assetId.startsWith("ws:")) return;
      const toast = useToastStore();
      try {
        const r = await invoke<{ thumbPath: string; modifiedAt: string; thumbFrame: string; thumbWidth: number; thumbHeight: number }>("set_video_thumb", {
          assetId: p.assetId,
          atSecs: this.currentTime,
        });
        const content = useContentStore();
        const target = content.placements.find(
          (x) => x.assetId === p.assetId && x.folderId === p.folderId,
        );
        if (target) {
          target.modifiedAt = r.modifiedAt; // URL 版本戳（AssetCard / ListRowCard thumbUrl ?v=）
          // 封面帧标记同步（2026-08-23 修复）：自动播放范围起点 = thumbFrame，此前不更新要重启才生效
          target.thumbFrame = r.thumbFrame;
          target.thumbW = r.thumbWidth; // 布局比例跟缩略图走（5.4）
          target.thumbH = r.thumbHeight;
          target.thumbPath = null;
          await nextTick();
          target.thumbPath = r.thumbPath;
        } else {
          await content.refresh();
        }
        toast.notify({ message: "已设为预览图", icon: "success" });
      } catch (e) {
        toast.notify({ message: String(e), icon: "fail" });
      }
    },
  },
});
