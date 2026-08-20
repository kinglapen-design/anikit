// 预览会话（4.3/5.2）：中栏预览模式的状态与行为。assetKey 非空 = 预览模式（ContentArea 据此切换）。
// 图片查看器（缩放/平移/旋转/翻转）与帧播放（序列帧 rAF 引擎按帧时长/rate 驱动；
// 动图 = 阶段四帧流：AnimStreamSession 收帧渲染 canvas，播放/暂停/倍速/定位由会话命令控制）共用本 store；
// 音视频播放状态在 previewMedia、逐帧笔记在 previewNotes（本 store 负责进入/退出时的接线与复位）。
// 容器尺寸由 PreviewPanel 的 ResizeObserver 写入，缩放瞄点数学集中在 zoomAt。
import { defineStore } from "pinia";
import { markRaw, nextTick } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useContentStore } from "./content";
import { usePreviewMediaStore } from "./previewMedia";
import { usePreviewNotesStore } from "./previewNotes";
import { useToastStore } from "./toast";
import { isAudioExt, isSequence, isVideoExt } from "../lib/formatBadge";
import { getAssetFilePath, listSequenceFrames, prepareAnimStrips, type FrameEntry } from "../lib/mediaOps";
import { AnimStreamSession, type AnimFrame } from "../lib/animStream";

// 播放帧（5.2 改版）：命令原始结构见 lib/mediaOps（FrameEntry.delayMs 序列帧恒 null）——
// 加载时统一预填 delayMs（动图用后端逐帧下发值；序列帧按 1000/fps），播放引擎按帧时长/rate 驱动。
interface PlayFrame extends FrameEntry {
  delayMs: number;
}

// image = 可显示图；sequence = 序列帧；anim = 动图（GIF/WEBP 动图）；audio/video = 音视频；other = 格式图标保底
export type PreviewKind = "image" | "sequence" | "anim" | "audio" | "video" | "other";
// 带图片源文件（psd/psb/ai/svg）：工具栏去旋转翻转、加默认应用打开
const SOURCE_EXTS = ["psd", "psb", "ai", "svg"];

const ZOOM_MIN = 0.02;
const ZOOM_MAX = 32;
export const RATES = [8, 4, 3, 2, 1.75, 1.5, 1.25, 1, 0.75, 0.5, 0.25];

// 帧播放引擎（模块级单例，store 外持有避免响应式开销）
let rafId = 0;
let lastT = 0;
let acc = 0;
// 动图帧流会话（模块级单例：预览同时至多一个；播放控制命令透传，帧渲染经 state.animFrame 暴露）
let animStream: AnimStreamSession | null = null;
// seek 拖动目标帧（非 null 时忽略在途旧帧——防播放条帧号被 seek 前的旧帧拉回跳动）
let seekTarget: number | null = null;

function stopAnimStream(): void {
  animStream?.stop();
  animStream = null;
  seekTarget = null;
}

// 出帧短边 = 显示短边（帧短边 × zoom；无帧时视口短边兜底），clamp [16,4096]（同后端口径）
function streamEdge(naturalW: number, naturalH: number, viewW: number, viewH: number, zoom: number): number {
  const base = naturalW > 0 ? Math.min(naturalW, naturalH) : Math.min(viewW, viewH);
  const e = Math.ceil(base * zoom);
  return Math.min(4096, Math.max(16, e || 320));
}

// 动图封面帧索引解析：新格式 `f:{索引0起}`；存量帧名（{标题}-00042.webp，序号 1 起）按尾号兼容；
// 其余（null / 视频 t: / 序列帧帧名）返回 null
function animCoverIndex(thumbFrame: string | null | undefined): number | null {
  if (!thumbFrame) return null;
  const f = thumbFrame.match(/^f:(\d+)$/);
  if (f) return Number(f[1]);
  const legacy = thumbFrame.match(/-(\d{5})\.webp$/);
  if (legacy) return Math.max(0, Number(legacy[1]) - 1);
  return null;
}

// 预览就绪事件监听（App 存活期注册一次；多窗口各自注册，事件广播按 assetId 匹配生效）
let unlistenFull: (() => void) | undefined;
let unlistenStrips: (() => void) | undefined;
let unlistenHidden: (() => void) | undefined;

export const usePreviewStore = defineStore("preview", {
  state: () => ({
    assetKey: null as string | null, // `${assetId}|${folderId}`，null = 非预览模式
    kind: "image" as PreviewKind,
    filePath: null as string | null, // 当前显示图（原图 / psd 缓存 / 当前帧）绝对路径
    error: null as string | null, // 媒体源解析失败等用户可见中文文案（5.2，画面提示）
    // 图片查看器：zoom 相对显示图像素 1:1；pan 为图中心相对容器中心的偏移（px）
    viewW: 0,
    viewH: 0,
    naturalW: 0, // 显示图实际尺寸（img onload / video loadedmetadata 写入）
    naturalH: 0,
    zoom: 1,
    panX: 0,
    panY: 0,
    // 当前视图处于"适合窗口"状态：容器尺寸变化（全屏切换 / 窗口拉伸）时自动跟随重新 fit；
    // 手动缩放 / 拖动 / 原图大小后置 false 不跟随（2026-07-26 用户：全屏画面要随容器撑满）
    fitMode: false,
    rotate: 0 as 0 | 90 | 180 | 270,
    flipH: false,
    // 帧播放（序列帧 rAF 引擎 / 动图帧流共用状态：frame = 当前帧索引，playing/rate 双向同步会话）
    seq: null as { thumbFrame: string | null; frames: PlayFrame[] } | null,
    frame: 0, // 当前帧索引（frames 数组下标 / 动图源帧索引，非帧号）
    playing: false,
    rate: 1,
    fullscreen: false,
    // 动图帧流当前帧（PreviewAnimCanvas 渲染用；markRaw 赋值防深度代理）与总帧数
    // （max(metadata frameCount, 已见 idx+1)——播放条分母，不依赖磁盘帧图）
    animFrame: null as AnimFrame | null,
    animTotal: 0,
    // 动图帧条小图（key = `f:{索引}` → stripPath；prepare_anim_strips 已缓存清单 +
    // preview-strips-ready 事件渐进填充）
    animStrips: {} as Record<string, string>,
  }),
  getters: {
    // 当前预览的 PlacementView（按 key 在当前视口列表里找）；
    // 包内视图（5.2，2026-08-21）：visiblePlacements 恒空——回退全量 placements 按 key 精确匹配，
    // 详情面板笔记跳帧在包内可直接开预览定位（index/total 仍按视口 = 空，上/下一个自然无效）
    current(): import("./content").PlacementView | null {
      const content = useContentStore();
      if (!this.assetKey) return null;
      return (
        content.visiblePlacements.find((p) => `${p.assetId}|${p.folderId}` === this.assetKey) ??
        content.placements.find((p) => `${p.assetId}|${p.folderId}` === this.assetKey) ??
        null
      );
    },
    // 索引 x/y（当前视口顺序）
    index(): number {
      const content = useContentStore();
      if (!this.assetKey) return 0;
      return content.visiblePlacements.findIndex(
        (p) => `${p.assetId}|${p.folderId}` === this.assetKey,
      );
    },
    total(): number {
      return useContentStore().visiblePlacements.length;
    },
    isSource(): boolean {
      const ext = this.current?.ext.toLowerCase() ?? "";
      return SOURCE_EXTS.includes(ext);
    },
    // 查看器控件（缩放组/旋转/翻转/适合窗口/原图大小）出现的 kind（5.2：audio/other 无）
    viewable(): boolean {
      return ["image", "sequence", "anim", "video"].includes(this.kind);
    },
    // 播放条出现的 kind（5.2：image/other 无）
    hasPlayBar(): boolean {
      return ["sequence", "anim", "audio", "video"].includes(this.kind);
    },
    // 旋转 90/270 后视觉宽高互换（fit 与导航器用）
    visualW(): number {
      return this.rotate % 180 === 0 ? this.naturalW : this.naturalH;
    },
    visualH(): number {
      return this.rotate % 180 === 0 ? this.naturalH : this.naturalW;
    },
    // 适合窗口缩放（小图也铺满；Eagle 适合窗口即铺满；全屏时无留白——画面按整个窗口铺满，2026-07-26 用户定）
    fitZoom(): number {
      if (!this.viewW || !this.viewH || !this.visualW || !this.visualH) return 1;
      const pad = this.fullscreen ? 0 : 48;
      return Math.min(
        (this.viewW - pad) / this.visualW,
        (this.viewH - pad) / this.visualH,
      );
    },
    // 图片是否溢出容器（导航器出现条件）
    overflow(): boolean {
      if (!this.viewable || !this.visualW || !this.visualH) return false;
      return (
        this.visualW * this.zoom > this.viewW + 1 || this.visualH * this.zoom > this.viewH + 1
      );
    },
  },
  actions: {
    // 预览就绪事件监听（App 启动注册一次）：PSD 高清 / 帧条小图渐进（序列帧与动图共用事件）
    async initPreviewListener(): Promise<void> {
      if (unlistenFull && unlistenStrips && unlistenHidden) return;
      unlistenFull = await listen<{ assetId: string; path: string }>("preview-full-ready", (e) => {
        if (this.current?.assetId === e.payload.assetId && this.kind === "image") {
          this.filePath = e.payload.path; // onImageLoad 按像素比换算保持视觉
        }
      });
      unlistenStrips = await listen<{ assetId: string; items: { name: string; stripPath: string }[] }>(
        "preview-strips-ready",
        (e) => {
          if (this.current?.assetId !== e.payload.assetId) return;
          if (this.kind === "anim") {
            // 动图：name = `f:{索引}`，渐进填充帧条缓存
            for (const item of e.payload.items) {
              this.animStrips[item.name] = item.stripPath;
            }
            return;
          }
          if (!this.seq) return;
          for (const item of e.payload.items) {
            const f = this.seq.frames.find((x) => x.name === item.name);
            if (f) f.stripPath = item.stripPath;
          }
        },
      );
      // 主窗口隐藏到托盘（关闭按钮 / 全局热键）：暂停双引擎播放
      // （2026-07-27 用户定：最小化保持播放，隐藏到托盘停止；重新显示后手动再播）
      unlistenHidden = await listen("main-window-hidden", () => {
        this.pause();
        usePreviewMediaStore().pause();
      });
    },

    // 进入预览（双击卡片 / 查看大图按钮）
    async open(key: string): Promise<void> {
      this.assetKey = key;
      await this.loadAsset();
    },
    // 退出预览
    close(): void {
      this.stopEngine();
      stopAnimStream();
      usePreviewMediaStore().release();
      usePreviewNotesStore().reset();
      if (this.fullscreen) void this.setFullscreen(false);
      this.assetKey = null;
      this.seq = null;
      this.playing = false;
      this.animFrame = null;
      this.animTotal = 0;
      this.animStrips = {};
    },
    // 上/下一个素材（当前视口顺序，到头停）
    async step(delta: number): Promise<void> {
      const content = useContentStore();
      const next = this.index + delta;
      if (next < 0 || next >= content.visiblePlacements.length) return;
      const p = content.visiblePlacements[next];
      this.assetKey = `${p.assetId}|${p.folderId}`;
      await this.loadAsset();
    },
    // 按当前 PlacementView 判定类型并加载显示路径（5.2 kind 分派）：
    // frameCount!=null && !animated → sequence；animated → anim；音频/视频扩展名 → audio/video；
    // 其余走 get_asset_file_path（image/other）。渐进加载（Eagle 式）：先秒显 info 缩略图，就绪后替换。
    async loadAsset(): Promise<void> {
      this.stopEngine();
      stopAnimStream();
      const media = usePreviewMediaStore();
      media.release();
      const notes = usePreviewNotesStore();
      notes.reset();
      this.playing = false;
      this.seq = null;
      this.frame = 0;
      this.error = null;
      this.animFrame = null;
      this.animTotal = 0;
      this.animStrips = {};
      this.resetView();
      const p = this.current;
      if (!p) {
        this.assetKey = null;
        return;
      }
      this.filePath = p.thumbPath; // 秒显：info 缩略图（格式图标同路径，other 情形也先盖住旧图）
      const ext = p.ext.toLowerCase();
      if (isSequence(p)) {
        this.kind = "sequence";
        const r = await listSequenceFrames(p.assetId);
        if (this.current?.assetId !== p.assetId) return; // await 期间已切走
        this.seq = {
          thumbFrame: r.thumbFrame,
          frames: r.frames.map((f) => ({ ...f, delayMs: 1000 / (r.fps ?? 24) })),
        };
        this.initFrame();
        void notes.load();
        return;
      }
      if (p.animated) {
        this.kind = "anim";
        await this.loadAnim(p.assetId);
        return;
      }
      if (isAudioExt(ext)) {
        this.kind = "audio";
        await media.load(p.assetId, "audio");
        return;
      }
      if (isVideoExt(ext)) {
        this.kind = "video";
        await media.load(p.assetId, "video", p.fps ?? null); // fps 随 PlacementView 透传（5.3 帧化）
        void notes.load();
        return;
      }
      const r = await getAssetFilePath(p.assetId);
      if (this.current?.assetId !== p.assetId) return; // await 期间已切走
      if (r.path) {
        this.kind = "image";
        this.filePath = r.path; // 高清层替换（onImageLoad 保持视觉尺寸）
      } else if (r.pending) {
        this.kind = "image"; // 保持缩略图，preview-full-ready 事件到达后替换
      } else {
        this.kind = "other"; // 格式图标保底（PreviewPanel 用 current.thumbPath 渲染）
        this.filePath = null;
      }
    },
    // 动图加载（阶段四·预览对接帧流 + 解包废除，2026-08-20 用户定案）：
    // 播放 = WS 帧流渲染 canvas（对 origin 运行时解码）；帧条 = prepare_anim_strips 按需解码
    // （cache/frames 会话级，事件渐进补齐）；设封面 / 笔记帧画面 = 后端 `f:{索引}` 抽帧。
    // 全部不依赖磁盘帧图（frames/ 导入解包已废除）。帧流失败回退原生 <img> 播源文件（无播放条）。
    async loadAnim(assetId: string): Promise<void> {
      stopAnimStream();
      this.animFrame = null;
      this.animTotal = 0;
      this.animStrips = {};
      const edge = streamEdge(this.naturalW, this.naturalH, this.viewW, this.viewH, this.zoom);
      const s = await AnimStreamSession.start(assetId, edge, (f) => this.onAnimFrame(f));
      if (this.current?.assetId !== assetId || this.kind !== "anim") {
        s?.stop(); // await 期间已切走
        return;
      }
      animStream = s;
      if (s) {
        this.playing = true; // 会话默认即播（进入自动播放，同 2026-07-25 用户定）
        s.setRate(this.rate); // 倍速跨素材保持，同步进新会话
        this.animTotal = this.current?.frameCount ?? 0;
        // 封面帧定位（对齐原 initFrame 行为：封面帧为初始帧，播放中 seek 后继续播）：
        // thumbFrame 新格式 = `f:{索引}`；存量帧名（{标题}-00042.webp）按尾号兼容解析
        const cover = animCoverIndex(this.current?.thumbFrame);
        if (cover !== null && cover > 0) this.setFrame(cover);
      } else {
        // 帧流失败回退：原生 <img> 播源文件（gif/webp 直渲格式返回源路径；取不到则保持缩略图）
        this.playing = false;
        const f = await getAssetFilePath(assetId);
        if (this.current?.assetId !== assetId || this.kind !== "anim") return;
        if (f.path) this.filePath = f.path;
      }
      // 帧条与逐帧笔记不依赖帧清单：帧条按需解码渐进，笔记 key = f:{索引}（后端 get_notes 附带旧 key 迁移）
      void usePreviewNotesStore().load();
      try {
        const r = await prepareAnimStrips(assetId);
        if (this.current?.assetId !== assetId || this.kind !== "anim") return;
        for (const item of r.items) {
          this.animStrips[item.name] = item.stripPath;
        }
      } catch (e) {
        console.warn("动图帧条准备失败（帧条暂缺，播放不受影响）：", e);
      }
    },
    // 帧流帧到达（会话 onFrame 回调）：更新画面帧 / 帧号 / 总帧数 / natural（尺寸热更新时换算保视觉）
    onAnimFrame(f: AnimFrame): void {
      if (this.kind !== "anim") return;
      // seek 期间忽略在途旧帧（帧号不被拉回跳动；目标帧到达后恢复）
      if (seekTarget !== null) {
        if (f.frameIndex !== seekTarget) return;
        seekTarget = null;
      }
      this.animFrame = markRaw(f);
      this.frame = f.frameIndex;
      if (f.frameIndex + 1 > this.animTotal) this.animTotal = f.frameIndex + 1;
      if (this.naturalW !== f.w || this.naturalH !== f.h) this.onImageLoad(f.w, f.h, true);
    },
    // 出帧尺寸跟随显示尺寸（帧短边 × zoom；PreviewPanel watchEffect 驱动：viewW/H/zoom 变化）
    syncAnimSize(): void {
      if (!animStream || this.kind !== "anim") return;
      animStream.setSize(streamEdge(this.naturalW, this.naturalH, this.viewW, this.viewH, this.zoom));
    },
    // 帧播放进入点（序列帧/动图共用）：封面帧为初始帧，进入自动播放（2026-07-25 用户定）
    initFrame(): void {
      if (!this.seq) return;
      const cover = this.seq.thumbFrame
        ? this.seq.frames.findIndex((f) => f.name === this.seq!.thumbFrame)
        : -1;
      this.frame = cover >= 0 ? cover : 0;
      const f = this.seq.frames[this.frame];
      if (f?.fullPath) this.filePath = f.fullPath;
      if (this.seq.frames.length >= 2) this.play();
    },
    resetView(): void {
      this.zoom = 1;
      this.panX = 0;
      this.panY = 0;
      this.rotate = 0;
      this.flipH = false;
      this.naturalW = 0;
      this.naturalH = 0;
    },
    // img onload / video loadedmetadata：记录显示图实际尺寸，初始化为适合窗口（同尺寸帧切换不重置视图）。
    // 渐进替换（缩略图 → 高清，同比例）：按新旧像素比换算 zoom，视觉尺寸不动、pan 不动（容器坐标系）。
    // fromStream = 动图帧流帧：帧流活着时占位 img 的 onload 不喂尺寸（320 缩略图会抢 natural）
    onImageLoad(w: number, h: number, fromStream = false): void {
      if (this.kind === "anim" && animStream && !fromStream) return;
      if (this.naturalW === w && this.naturalH === h) return;
      if (this.naturalW > 0 && this.naturalH > 0) {
        this.zoom = this.zoom * (this.naturalW / w);
        this.naturalW = w;
        this.naturalH = h;
        return;
      }
      this.naturalW = w;
      this.naturalH = h;
      this.fit();
    },
    // 缩放瞄点：保持容器内 (cx, cy) 点不动（滚轮传鼠标位置，滑块传容器中心）
    zoomAt(newZoom: number, cx: number, cy: number): void {
      const z = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, newZoom));
      const cX = cx - this.viewW / 2;      const cY = cy - this.viewH / 2;
      const k = z / this.zoom;
      this.panX = cX - (cX - this.panX) * k;
      this.panY = cY - (cY - this.panY) * k;
      this.zoom = z;
      this.fitMode = false; // 手动缩放退出"适合窗口"跟随
    },
    fit(): void {
      this.zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, this.fitZoom));
      this.panX = 0;
      this.panY = 0;
      this.fitMode = true;
    },
    actual(): void {
      this.zoom = 1;
      this.panX = 0;
      this.panY = 0;
      this.fitMode = false;
    },
    rotateCw(): void {
      this.rotate = ((this.rotate + 90) % 360) as 0 | 90 | 180 | 270;
      this.fit();
    },
    toggleFlip(): void {
      this.flipH = !this.flipH;
    },
    // —— 帧播放（序列帧 rAF 引擎；动图 = 帧流会话命令，5.2 交互语义不变）——
    setFrame(i: number): void {
      if (this.kind === "anim" && animStream) {
        const n = Math.max(1, this.animTotal);
        const t = Math.min(n - 1, Math.max(0, i));
        this.frame = t; // 乐观更新帧号；目标帧到达前忽略在途旧帧
        seekTarget = t;
        animStream.seek(t);
        return;
      }
      if (!this.seq || this.seq.frames.length === 0) return;
      this.frame = Math.min(this.seq.frames.length - 1, Math.max(0, i));
      const f = this.seq.frames[this.frame];
      if (f.fullPath) this.filePath = f.fullPath; // 路径缺失兜底：保持当前画面
      this.preloadAround();
    },
    stepFrame(delta: number): void {
      if (this.kind === "anim" && animStream) {
        if (this.playing) this.pause(); // 逐帧步进即暂停（2026-07-26 用户定）；步进到头停（clamp）
        this.setFrame(this.frame + delta);
        return;
      }
      if (!this.seq) return;
      const n = this.seq.frames.length;
      if (this.playing) this.pause();
      this.setFrame(Math.min(n - 1, Math.max(0, this.frame + delta)));
    },
    play(): void {
      if (this.kind === "anim") {
        if (!animStream || this.playing) return;
        this.playing = true;
        animStream.setPlaying(true);
        return;
      }
      if (!this.seq || this.seq.frames.length < 2 || this.playing) return;
      this.playing = true;
      lastT = performance.now();
      acc = 0;
      const tick = (t: number): void => {
        if (!this.playing || !this.seq) return;
        acc += t - lastT;
        lastT = t;
        const n = this.seq.frames.length;
        let guard = 0;
        while (guard++ < 8) {
          const interval = Math.max(16, this.seq.frames[this.frame].delayMs / this.rate);
          if (acc < interval) break;
          const next = (this.frame + 1) % n;
          if (!this.seq.frames[next].fullPath) {
            acc = 0; // 下一帧路径缺失：停住等就绪（不追帧防跳变；解包帧正常恒有路径）
            break;
          }
          acc -= interval;
          this.frame = next; // 播放循环
        }
        const f = this.seq.frames[this.frame];
        if (f.fullPath) this.filePath = f.fullPath;
        rafId = requestAnimationFrame(tick);
      };
      rafId = requestAnimationFrame(tick);
    },
    pause(): void {
      if (this.kind === "anim") {
        this.playing = false;
        animStream?.setPlaying(false);
        return;
      }
      this.playing = false;
      this.stopEngine();
    },
    togglePlay(): void {
      if (this.playing) this.pause();
      else this.play();
    },
    stopEngine(): void {
      if (rafId) cancelAnimationFrame(rafId);
      rafId = 0;
    },
    // 预加载相邻帧（防切帧白闪；路径缺失帧跳过）
    preloadAround(): void {
      if (!this.seq) return;
      const n = this.seq.frames.length;
      for (const d of [1, 2, -1]) {
        const f = this.seq.frames[(this.frame + d + n) % n];
        if (f?.fullPath) {
          const img = new Image();
          img.src = convertFileSrc(f.fullPath);
        }
      }
    },
    setRate(r: number): void {
      this.rate = r;
      if (this.kind === "anim") animStream?.setRate(r);
    },
    // 当前帧设为预览图（序列帧 / 动图通用，5.2）：后端重建缩略图 + 记封面帧，前端即时换卡片图。
    // 动图传 `f:{索引}`（origin 解码抽帧，2026-08-20 解包废除）；序列帧传 frames/ 内帧名。
    // 结果 toast 反馈（2026-08-20 用户：点击无反馈）；target.modifiedAt 同步 = 缩略图 URL 版本戳
    //（asset 协议同 URL 启发式缓存致静帧不热刷新，用户实机报）；视图外资产兜底全量 refresh
    async setAsThumb(): Promise<void> {
      const p = this.current;
      if (!p) return;
      const frameName =
        this.kind === "anim" ? `f:${this.frame}` : this.seq ? this.seq.frames[this.frame]?.name : undefined;
      if (!frameName) return;
      const toast = useToastStore();
      try {
        const r = await invoke<{ thumbPath: string; modifiedAt: string }>("set_thumb_frame", {
          assetId: p.assetId,
          frameName,
        });
        // 即时更新卡片缩略图：路径不变（同词干重建），置空再还原迫使 <img> 重载（asset 协议不走 HTTP 缓存）
        const content = useContentStore();
        const target = content.placements.find(
          (x) => x.assetId === p.assetId && x.folderId === p.folderId,
        );
        if (target) {
          target.modifiedAt = r.modifiedAt; // URL 版本戳（AssetCard / ListRowCard thumbUrl ?v=）
          target.thumbPath = null;
          await nextTick();
          target.thumbPath = r.thumbPath;
        } else {
          await content.refresh(); // 视图外资产（回收站等）兜底：全量重拉
        }
        toast.notify({ message: "已设为预览图", icon: "success" });
      } catch (e) {
        toast.notify({ message: String(e), icon: "fail" });
      }
    },
    // 显示器全屏（2026-07-25 定案）：预览区 fixed 铺满 + 窗口 fullscreen，Esc 退出
    async setFullscreen(on: boolean): Promise<void> {
      this.fullscreen = on;
      await getCurrentWindow().setFullscreen(on);
    },
    async openDefaultApp(): Promise<void> {
      const p = this.current;
      if (!p) return;
      await invoke("open_with_default_app", { assetId: p.assetId });
      // 成功后直接重拉最近使用（2026-07-29 复验反馈：双保险，同 AssetCard.openDefault）
      await useContentStore().refreshRecent();
    },
  },
});
