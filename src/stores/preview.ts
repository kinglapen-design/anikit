// 预览会话（4.3）：中栏预览模式的状态与行为。assetKey 非空 = 预览模式（ContentArea 据此切换）。
// 图片查看器（缩放/平移/旋转/翻转）与序列帧播放（rAF 引擎/帧条/封面帧）共用本 store；
// 容器尺寸由 PreviewPanel 的 ResizeObserver 写入，缩放瞄点数学集中在 zoomAt。
import { defineStore } from "pinia";
import { nextTick } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useContentStore } from "./content";

// 与后端 commands/preview.rs 对应（camelCase，07_接口文档）
export interface FrameEntry {
  name: string; // 帧文件名（标题-序号.扩展名）
  fullPath: string; // 帧原图绝对路径（大图显示）
  stripPath: string | null; // 帧条小图绝对路径；null = 后台生成中（preview-strips-ready 渐进填充）
}

interface SequenceFrames {
  fps: number;
  thumbFrame: string | null;
  frames: FrameEntry[];
}

// get_asset_file_path 返回：pending = 高清层后台解析中（preview-full-ready 到达后替换）
interface AssetFilePath {
  path: string | null;
  pending: boolean;
}

// image = 可显示图（直渲或 psd 缓存）；sequence = 序列帧；other = 格式图标保底
export type PreviewKind = "image" | "sequence" | "other";
// 带图片源文件（psd/psb/ai/svg）：工具栏去旋转翻转、加默认应用打开
const SOURCE_EXTS = ["psd", "psb", "ai", "svg"];

const ZOOM_MIN = 0.02;
const ZOOM_MAX = 32;
export const RATES = [0.25, 0.5, 1, 2, 4];

// 序列帧播放引擎（模块级单例，store 外持有避免响应式开销）
let rafId = 0;
let lastT = 0;
let acc = 0;
// 预览就绪事件监听（App 存活期注册一次；多窗口各自注册，事件广播按 assetId 匹配生效）
let unlistenFull: (() => void) | undefined;
let unlistenStrips: (() => void) | undefined;

export const usePreviewStore = defineStore("preview", {
  state: () => ({
    assetKey: null as string | null, // `${assetId}|${folderId}`，null = 非预览模式
    kind: "image" as PreviewKind,
    filePath: null as string | null, // 当前显示图（原图 / psd 缓存 / 当前帧）绝对路径
    // 图片查看器：zoom 相对显示图像素 1:1；pan 为图中心相对容器中心的偏移（px）
    viewW: 0,
    viewH: 0,
    naturalW: 0, // 显示图实际尺寸（img onload 写入；psd 缓存图以缓存尺寸为准）
    naturalH: 0,
    zoom: 1,
    panX: 0,
    panY: 0,
    rotate: 0 as 0 | 90 | 180 | 270,
    flipH: false,
    // 序列帧
    seq: null as SequenceFrames | null,
    frame: 0, // 当前帧索引（frames 数组下标，非帧号）
    playing: false,
    rate: 1,
    fullscreen: false,
  }),
  getters: {
    // 当前预览的 PlacementView（按 key 在当前视口列表里找）
    current(): import("./content").PlacementView | null {
      const content = useContentStore();
      if (!this.assetKey) return null;
      return (
        content.visiblePlacements.find((p) => `${p.assetId}|${p.folderId}` === this.assetKey) ?? null
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
    // 旋转 90/270 后视觉宽高互换（fit 与导航器用）
    visualW(): number {
      return this.rotate % 180 === 0 ? this.naturalW : this.naturalH;
    },
    visualH(): number {
      return this.rotate % 180 === 0 ? this.naturalH : this.naturalW;
    },
    // 适合窗口缩放（留 24px 边距，不放大超过 1:1……允许放大——小图也铺满；Eagle 适合窗口即铺满）
    fitZoom(): number {
      if (!this.viewW || !this.viewH || !this.visualW || !this.visualH) return 1;
      const pad = 48;
      return Math.min(
        (this.viewW - pad) / this.visualW,
        (this.viewH - pad) / this.visualH,
      );
    },
    // 图片是否溢出容器（导航器出现条件）
    overflow(): boolean {
      if (this.kind === "other" || !this.visualW || !this.visualH) return false;
      return (
        this.visualW * this.zoom > this.viewW + 1 || this.visualH * this.zoom > this.viewH + 1
      );
    },
  },
  actions: {
    // 预览就绪事件监听（App 启动注册一次）：PSD 高清 / 帧条小图后台生成完成后渐进替换
    async initPreviewListener(): Promise<void> {
      if (unlistenFull && unlistenStrips) return;
      unlistenFull = await listen<{ assetId: string; path: string }>("preview-full-ready", (e) => {
        if (this.current?.assetId === e.payload.assetId && this.kind === "image") {
          this.filePath = e.payload.path; // onImageLoad 按像素比换算保持视觉
        }
      });
      unlistenStrips = await listen<{ assetId: string; items: { name: string; stripPath: string }[] }>(
        "preview-strips-ready",
        (e) => {
          if (!this.seq || this.current?.assetId !== e.payload.assetId) return;
          for (const item of e.payload.items) {
            const f = this.seq.frames.find((x) => x.name === item.name);
            if (f) f.stripPath = item.stripPath;
          }
        },
      );
    },

    // 进入预览（双击卡片 / 查看大图按钮）
    async open(key: string): Promise<void> {
      this.assetKey = key;
      await this.loadAsset();
    },
    // 退出预览
    close(): void {
      this.stopEngine();
      if (this.fullscreen) void this.setFullscreen(false);
      this.assetKey = null;
      this.seq = null;
      this.playing = false;
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
    // 按当前 PlacementView 判定类型并加载显示路径。
    // 渐进加载（Eagle 式，2026-07-25 用户定）：先秒显 info 缩略图（thumbPath），
    // 真路径到达后替换高清（大图解析不阻塞首显；psd/psb 为全尺寸缓存）——
    // 同时保证切换素材时旧图不残留。
    async loadAsset(): Promise<void> {
      this.stopEngine();
      this.playing = false;
      this.seq = null;
      this.frame = 0;
      this.resetView();
      const p = this.current;
      if (!p) {
        this.assetKey = null;
        return;
      }
      this.filePath = p.thumbPath; // 秒显：info 缩略图（格式图标同路径，other 情形也先盖住旧图）
      if (p.frameCount != null && !p.animated) {
        this.kind = "sequence";
        const seq = await invoke<SequenceFrames>("list_sequence_frames", { assetId: p.assetId });
        if (this.current?.assetId !== p.assetId) return; // await 期间已切走
        this.seq = seq;
        // 封面帧为初始帧（thumbFrame 是文件名）
        const cover = seq.thumbFrame ? seq.frames.findIndex((f) => f.name === seq.thumbFrame) : -1;
        this.frame = cover >= 0 ? cover : 0;
        this.filePath = seq.frames[this.frame]?.fullPath ?? null;
        // 进入自动播放（2026-07-25 用户定）；帧条小图后台生成，不阻塞播放
        if (seq.frames.length >= 2) this.play();
        return;
      }
      const r = await invoke<AssetFilePath>("get_asset_file_path", { assetId: p.assetId });
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
    resetView(): void {
      this.zoom = 1;
      this.panX = 0;
      this.panY = 0;
      this.rotate = 0;
      this.flipH = false;
      this.naturalW = 0;
      this.naturalH = 0;
    },
    // img onload：记录显示图实际尺寸，初始化为适合窗口（序列帧同尺寸帧切换不重置视图）。
    // 渐进替换（缩略图 → 高清，同比例）：按新旧像素比换算 zoom，视觉尺寸不动、pan 不动（容器坐标系）
    onImageLoad(w: number, h: number): void {
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
      const cX = cx - this.viewW / 2;
      const cY = cy - this.viewH / 2;
      const k = z / this.zoom;
      this.panX = cX - (cX - this.panX) * k;
      this.panY = cY - (cY - this.panY) * k;
      this.zoom = z;
    },
    fit(): void {
      this.zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, this.fitZoom));
      this.panX = 0;
      this.panY = 0;
    },
    actual(): void {
      this.zoom = 1;
      this.panX = 0;
      this.panY = 0;
    },
    rotateCw(): void {
      this.rotate = ((this.rotate + 90) % 360) as 0 | 90 | 180 | 270;
      this.fit();
    },
    toggleFlip(): void {
      this.flipH = !this.flipH;
    },
    // —— 序列帧 ——
    setFrame(i: number): void {
      if (!this.seq || this.seq.frames.length === 0) return;
      this.frame = Math.min(this.seq.frames.length - 1, Math.max(0, i));
      this.filePath = this.seq.frames[this.frame].fullPath;
      this.preloadAround();
    },
    stepFrame(delta: number): void {
      if (!this.seq) return;
      const n = this.seq.frames.length;
      // 逐帧步进到头停（播放中才循环）
      this.setFrame(Math.min(n - 1, Math.max(0, this.frame + delta)));
    },
    play(): void {
      if (!this.seq || this.seq.frames.length < 2 || this.playing) return;
      this.playing = true;
      lastT = performance.now();
      acc = 0;
      const tick = (t: number): void => {
        if (!this.playing || !this.seq) return;
        acc += t - lastT;
        lastT = t;
        const interval = 1000 / (this.seq.fps * this.rate);
        const n = this.seq.frames.length;
        while (acc >= interval) {
          acc -= interval;
          this.frame = (this.frame + 1) % n; // 播放循环
        }
        this.filePath = this.seq.frames[this.frame].fullPath;
        rafId = requestAnimationFrame(tick);
      };
      rafId = requestAnimationFrame(tick);
    },
    pause(): void {
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
    // 预加载相邻帧（防切帧白闪）
    preloadAround(): void {
      if (!this.seq) return;
      const n = this.seq.frames.length;
      for (const d of [1, 2, -1]) {
        const f = this.seq.frames[(this.frame + d + n) % n];
        if (f) {
          const img = new Image();
          img.src = convertFileSrc(f.fullPath);
        }
      }
    },
    setRate(r: number): void {
      this.rate = r;
    },
    // 当前帧设为预览图：后端重建缩略图 + 记封面帧，前端即时换卡片图（2026-07-25 定案）
    async setAsThumb(): Promise<void> {
      const p = this.current;
      if (!p || !this.seq) return;
      const frameName = this.seq.frames[this.frame]?.name;
      if (!frameName) return;
      const r = await invoke<{ thumbPath: string }>("set_thumb_frame", {
        assetId: p.assetId,
        frameName,
      });
      // 即时更新卡片缩略图：路径不变（同词干重建），置空再还原迫使 <img> 重载（asset 协议不走 HTTP 缓存）
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
    // 显示器全屏（2026-07-25 定案）：预览区 fixed 铺满 + 窗口 fullscreen，Esc 退出
    async setFullscreen(on: boolean): Promise<void> {
      this.fullscreen = on;
      await getCurrentWindow().setFullscreen(on);
    },
    async openDefaultApp(): Promise<void> {
      const p = this.current;
      if (p) await invoke("open_with_default_app", { assetId: p.assetId });
    },
  },
});
