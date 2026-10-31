// 预览会话（4.3/5.2）：中栏预览模式的状态与行为。assetKey 非空 = 预览模式（ContentArea 据此切换）。
// per-panel 会话化（2026-09-25，根治分屏双预览互抢 / 左右跳）：每 dockview panel 一会话——
// buildPreviewStore 工厂产 store，注册表 previewSession(panelId) 懒建；属主 = 会话本身
// （state.panelId = 工厂参数，创建后不变），面板组件经 lib/previewSessionKey provide/inject。
// 图片查看器（缩放/平移/旋转/翻转）与帧播放（序列帧 rAF 引擎按帧时长/rate 驱动；
// 动图 = 阶段四帧流（协议 v2）：StreamSession 收帧渲染 canvas，播放/暂停/倍速/定位由会话命令控制）共用本 store；
// 音视频播放状态在 previewMedia（<video> 元素驱动）、逐帧笔记在 previewNotes（本 store 负责进入/退出时的接线与复位）。
// 容器尺寸由 PreviewPanel 的 ResizeObserver 写入，缩放瞄点数学集中在 zoomAt。
import { defineStore, getActivePinia } from "pinia";
import { markRaw, nextTick } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { placementKey, useContentStore } from "./content";
import { dropPreviewMediaSession, eachPreviewMediaSession, previewMediaSession } from "./previewMedia";
import { dropPreviewNotesSession, previewNotesSession } from "./previewNotes";
import { useToastStore } from "./toast";
import { isAudioExt, isDocxExt, isFontExt, isSequence, isStaticImageExt, isTextExt, isVideoExt } from "../lib/formatBadge";
import { getAssetFilePath, listSequenceFrames, prepareAnimStrips, type FrameEntry } from "../lib/mediaOps";
import { isWsPlacement } from "./ws";
import { openPathWithDefaultApp } from "../lib/wsOps";
import { useSearchStore } from "./search";
import { useTabsStore } from "./tabs";
import { StreamSession, type StreamFrame } from "../lib/streamClient";
import { segmentFrameCount, type SegmentRange } from "../lib/semantic";

// 播放帧（5.2 改版）：命令原始结构见 lib/mediaOps（FrameEntry.delayMs 序列帧恒 null）——
// 加载时统一预填 delayMs（动图用后端逐帧下发值；序列帧按 1000/fps），播放引擎按帧时长/rate 驱动。
interface PlayFrame extends FrameEntry {
  delayMs: number;
}

// image = 可显示图；sequence = 序列帧；anim = 动图（GIF/WEBP 动图）；audio/video = 音视频；
// font = 字体（5.6）；text = 纯文本（5.6，txt 编辑 / md 渲染）；docx = Word 文档（R6，只读渲染）；
// other = 格式图标保底
export type PreviewKind = "image" | "sequence" | "anim" | "audio" | "video" | "font" | "text" | "docx" | "other";
// 带图片源文件（psd/psb/ai/svg）：工具栏去旋转翻转、加默认应用打开
const SOURCE_EXTS = ["psd", "psb", "ai", "svg"];

// 预览导航列表（current/index/total/step 同一数据源）：ws 搜索态 = 搜索结果集（ws 命中跨目录
// 合成、只存于 search store wsEntries，视口目录缓存查不到——2026-10-03 修复双击搜索结果是空开）；
// 库视口非搜索态 = 视口显示序（content.displayOrderFor：分组视口 = 组拼接扁平序——2026-10-08
// 修复分组下翻页按未分组序走、与网格所见"下一个"不符）；搜索态分组失效平铺（AssetGrid 同口径），
// 保 placementsFor 未分组序（库搜索 = 同集过滤子集，天然兼容）
function previewList(panelId: string, viewKey: string, content: ReturnType<typeof useContentStore>): import("./content").PlacementView[] {
  const search = useSearchStore();
  if (viewKey.startsWith("ws:")) {
    if (search.isActive(panelId)) return search.wsPlacementsFor(panelId);
    return content.placementsFor(viewKey);
  }
  if (search.isActive(panelId)) return search.resultPlacementsFor(panelId, viewKey);
  return content.displayOrderFor(viewKey);
}

const ZOOM_MIN = 0.02;
const ZOOM_MAX = 32;
export const RATES = [8, 4, 3, 2, 1.75, 1.5, 1.25, 1, 0.75, 0.5, 0.25];

// 出帧短边 = 显示短边（帧短边 × zoom × DPR——2026-08-22 用户实机"模糊"修复：CSS 像素未计 DPR，
// 4K/150% 屏 1.5 倍欠采样；DPR >2 不追，收益边际）；无帧时视口短边兜底，clamp [16,2048]
//（上限收紧 4096→2048：大源高帧率下 WS 吞吐 / putImageData 主线程预算有限）
function streamEdge(naturalW: number, naturalH: number, viewW: number, viewH: number, zoom: number): number {
  const base = naturalW > 0 ? Math.min(naturalW, naturalH) : Math.min(viewW, viewH);
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const e = Math.ceil((base || 320) * zoom * dpr);
  return Math.min(2048, Math.max(16, e));
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

// 预览就绪事件监听（App 存活期注册一次；多窗口各自注册）：事件广播扇出全部已建会话，按 assetId 匹配生效
let unlistenFull: (() => void) | undefined;
let unlistenStrips: (() => void) | undefined;
let unlistenHidden: (() => void) | undefined;

function buildPreviewStore(panelId: string) {
  // 帧播放引擎（会话级，store 外持有避免响应式开销）
  let rafId = 0;
  let lastT = 0;
  let acc = 0;
  // 动图帧流会话（会话级单例：本面板预览同时至多一个；播放控制命令透传，帧渲染经 state.animFrame 暴露）
  let streamSession: StreamSession | null = null;
  // seek 拖动目标帧（非 null 时忽略在途旧帧——防播放条帧号被 seek 前的旧帧拉回跳动）
  let seekTarget: number | null = null;

  function stopStream(): void {
    streamSession?.stop();
    streamSession = null;
    seekTarget = null;
  }

  // 全屏前的工作视图暂存（2026-09-17 实机走查：全屏 = 临时充满看大，退出还原用户取景，
  // 不污染抓手/缩放好的工作视图——详见 setFullscreen）
  let savedView: { zoom: number; panX: number; panY: number; fitMode: boolean } | null = null;

  return defineStore(`preview#${panelId}`, {
    state: () => ({
      assetKey: null as string | null, // placementKey 口径（撞键行带后缀），null = 非预览模式
      // 预览属主面板（2026-09-25 用户实机三报根治）：per-panel 会话化后属主 = 会话本身
      //（工厂参数，创建后不变）——新标签 / 分屏新 panel 各持各的会话，互不抢预览、不左右跳
      panelId,
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
      // 字体预览演示文本正文字号 px（标题 ×2；2026-09-26 用户定：预览区底部悬浮滑杆废，
      // 字号控制归预览工具栏尺寸滑块——PreviewTitleBar/PreviewFont 经本值共享，会话级记忆）
      fontSize: 15,
      // 帧播放（序列帧 rAF 引擎 / 动图帧流共用状态：frame = 当前帧索引，playing/rate 双向同步会话）
      seq: null as { thumbFrame: string | null; frames: PlayFrame[] } | null,
      segment: null as SegmentRange | null,
      frame: 0, // 当前帧索引（frames 数组下标 / 动图源帧索引，非帧号）
      playing: false,
      rate: 1,
      fullscreen: false,
      // 动图帧流当前帧（PreviewStreamCanvas 渲染用；markRaw 赋值防深度代理）与总帧数
      // （max(metadata frameCount, 已见 idx+1)——播放条分母，不依赖磁盘帧图）
      animFrame: null as StreamFrame | null,
      animTotal: 0,
      // 动图帧条小图（key = `f:{索引}` → stripPath；prepare_anim_strips 已缓存清单 +
      // preview-strips-ready 事件渐进填充）
      animStrips: {} as Record<string, string>,
      // md 文本编辑（文本编辑稿轮）：编辑态标记（PreviewText 进出写入；FloatBarHost 据以切 MarkDown 工具栏）
      mdEditing: false,
      // md 大纲栏开关（2026-10-15 大纲轮，PRD 04「md 大纲栏」）：会话级——切素材保持，关预览复位
      mdOutline: false,
      mdActionSeq: 0, // 工具栏动作信号（search.focusSeq 同款模式：seq 推进 = 新动作待执行）
      mdActionName: "",
      // 「打开编辑器」md 直进编辑态一次性信号（提示词轮，lib/promptOps openPromptPlacement 置位）：
      // 置 true 后 open() 文本资产，PreviewText 内容就绪后消费（直进编辑态）；txt 恒编辑态无需置
      pendingMdEdit: false,
    }),
    getters: {
      // 当前预览的 PlacementView（按 key 在属主面板视口列表里找）；
      // 包内视图（5.2，2026-08-21）：placementsFor 恒空——回退全量 placements 按 key 精确匹配，
      // 详情面板笔记跳帧在包内可直接开预览定位（index/total 仍按视口 = 空，上/下一个自然无效）
      // ws 搜索态（2026-10-03 修复）：ws 搜索命中跨目录合成、只存于 search store wsEntries，
      // 视口目录缓存与库 placements 都查不到——搜索态优先在搜索结果集里找，否则双击打开静默无效
      current(): import("./content").PlacementView | null {
        const content = useContentStore();
        if (!this.assetKey) return null;
        return (
          previewList(panelId, useTabsStore().viewKeyOf(panelId), content).find((p) => placementKey(p) === this.assetKey) ??
          content.placements.find((p) => placementKey(p) === this.assetKey) ??
          null
        );
      },
      // 索引 x/y（属主面板视口顺序；ws 搜索态 = 搜索结果集顺序，与网格同序）
      index(): number {
        const content = useContentStore();
        if (!this.assetKey) return 0;
        const list = previewList(panelId, useTabsStore().viewKeyOf(panelId), content);
        return list.findIndex((p) => placementKey(p) === this.assetKey);
      },
      total(): number {
        return previewList(panelId, useTabsStore().viewKeyOf(panelId), useContentStore()).length;
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
      // 进入预览（双击卡片 / 查看大图按钮）：属主 = 会话本身（per-panel 工厂参数），不再快照激活面板
      async open(key: string, segment?: import("../lib/semantic").SegmentHit | null): Promise<void> {
        this.assetKey = key;
        await this.loadAsset();
        if (this.assetKey !== key) return;
        const target = segment === undefined ? useSearchStore().semanticHitFor(panelId, key.split("|")[0])?.best : segment;
        if (target) this.seekToSegment(target);
      },
      // 退出预览
      close(): void {
        this.stopEngine();
        stopStream();
        previewMediaSession(panelId).release();
        previewNotesSession(panelId).reset();
        if (this.fullscreen) void this.setFullscreen(false);
        this.assetKey = null;
        this.seq = null;
        this.segment = null;
        this.playing = false;
        this.animFrame = null;
        this.animTotal = 0;
        this.animStrips = {};
        this.mdEditing = false;
        this.mdOutline = false;
        this.pendingMdEdit = false; // 一次性信号不带出会话（关闭即作废）
      },
      // md 编辑态进出（PreviewText 写入；FloatBarHost 读以切悬浮栏 MarkDown 工具栏模式）
      setMdEditing(v: boolean): void {
        this.mdEditing = v;
      },
      // md 大纲栏开关（PreviewText 读；FloatBarMarkdown 大纲钮经 requestMdAction("outline") 翻转）
      setMdOutline(v: boolean): void {
        this.mdOutline = v;
      },
      // 悬浮栏 MarkDown 工具栏动作 → PreviewText 执行（seq 推进通知，search.focusSeq 同款模式）
      requestMdAction(name: string): void {
        this.mdActionName = name;
        this.mdActionSeq++;
      },
      // 上/下一个素材（属主面板视口显示序——分组视口 = 组拼接序，到头停；ws 搜索态 = 搜索结果集顺序，previewList 同源）
      async step(delta: number): Promise<void> {
        const list = previewList(panelId, useTabsStore().viewKeyOf(panelId), useContentStore());
        const next = this.index + delta;
        if (next < 0 || next >= list.length) return;
        const p = list[next];
        this.assetKey = placementKey(p);
        await this.loadAsset();
      },
      // 按当前 PlacementView 判定类型并加载显示路径（5.2 kind 分派）：
      // frameCount!=null && !animated → sequence；animated → anim；音频/视频扩展名 → audio/video；
      // 其余走 get_asset_file_path（image/other）。渐进加载（Eagle 式）：先秒显 info 缩略图，就绪后替换。
      async loadAsset(): Promise<void> {
        this.stopEngine();
        stopStream();
        const media = previewMediaSession(panelId);
        media.release();
        const notes = previewNotesSession(panelId);
        notes.reset();
        this.playing = false;
        this.seq = null;
        this.frame = 0;
        this.error = null;
        this.animFrame = null;
        this.animTotal = 0;
        this.animStrips = {};
        this.resetView();
        this.segment = null;
        const p = this.current;
        if (!p) {
          this.assetKey = null;
          return;
        }
        this.segment = p.segment ?? null;
        this.filePath = p.thumbPath; // 秒显：info 缩略图（格式图标同路径，other 情形也先盖住旧图）
        const ext = p.ext.toLowerCase();
        // 工作空间磁盘实况（PRD 02 步骤 1 + 2026-10-23 文档预览放行）：assetId = ws: 合成假 id，
        // 一律走按路径分支，不碰 assetId 命令——图片 / 动图（GIF 及 WEBP 动图 Chromium 原生循环）/ SVG
        // 直 convertFileSrc 源路径（秒显后换源）；音视频走 get_media_source_by_path（previewMedia.loadPath，
        // error 后 forceTranscode 兜底，踩坑 075 口径）；字体 / 文本 / docx 进对应预览页（组件内按 wsPath
        // 分流按路径命令，宿主零配合；md / txt 只读不编辑——写回磁盘链路另立）；其余（工程 / 未知格式）
        // 优雅降级 other（格式图标保底，不报错）
        if (isWsPlacement(p)) {
          const wsPath = p.wsPath;
          if (!wsPath) {
            this.kind = "other";
            this.filePath = null;
            return;
          }
          if (isAudioExt(ext)) {
            this.kind = "audio";
            await media.loadPath(wsPath, "audio");
            return;
          }
          if (isVideoExt(ext)) {
            this.kind = "video";
            await media.loadPath(wsPath, "video", p.fps ?? null);
            return;
          }
          if (isStaticImageExt(ext) || ext === "gif" || ext === "svg") {
            this.kind = "image";
            this.filePath = wsPath;
            return;
          }
          if (isFontExt(ext)) {
            this.kind = "font";
            return;
          }
          if (isTextExt(ext)) {
            this.kind = "text";
            return;
          }
          if (isDocxExt(ext)) {
            this.kind = "docx";
            return;
          }
          this.kind = "other"; // 工程 / 未知格式：格式图标保底
          this.filePath = null;
          return;
        }
        if (isSequence(p)) {
          this.kind = "sequence";
          const r = await listSequenceFrames(p.assetId);
          if (this.current?.assetId !== p.assetId) return; // await 期间已切走
          this.segment = r.segment ?? this.segment;
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
          // 播放源代理（8.1）收在 loadAnim 内部（开流源 = playAssetId ?? assetId）；
          // 此处置预览目标自身——"已切走"守卫按 current 判定
          await this.loadAnim(p.assetId);
          return;
        }
        // 工程资产静态伴生（2026-09-02 用户实机报：静态 fla 预览滚轮缩放失效——落 other 无 viewable）：
        // 伴生 hidden 为静态图时预览 = 该图本体（image 全功能：缩放/平移/查看器控件）；
        // 动态伴生已由上支 animated 代理接住（kind=anim）；解析失败落回下方原流程（other 保底）
        if (p.playAssetId) {
          const r = await getAssetFilePath(p.playAssetId);
          if (this.current?.assetId !== p.assetId) return; // await 期间已切走
          if (r.path) {
            this.kind = "image";
            this.filePath = r.path;
            return;
          }
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
        // 字体（5.6）：预览页演示文本（@font-face 组件内加载）+ 其它页元数据与系统级安装
        if (isFontExt(ext)) {
          this.kind = "font";
          return;
        }
        // 纯文本（5.6）：txt = APP 内编辑（防抖自动保存）；md = markdown-it 只读渲染（编辑归 6.2）
        if (isTextExt(ext)) {
          this.kind = "text";
          return;
        }
        // docx（R6）：后端段落结构提取，前端只读渲染（组件内自取数据，同 font/text 无需宿主配合）
        if (isDocxExt(ext)) {
          this.kind = "docx";
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
      // 全部不依赖磁盘帧图（frames/ 导入解包已废除）。普通动图帧流失败回退原生 <img>；
      // 虚拟片段失败保持封面并报错，不得退回播放整个源文件。
      async loadAnim(assetId: string): Promise<void> {
        stopStream();
        this.animFrame = null;
        this.animTotal = 0;
        this.animStrips = {};
        // 播放源代理（8.1）：fla 等工程的伴生 WebP——开流 / 帧条 / 回退图源全走伴生资产；
        // "已切走"守卫恒按预览目标（assetId = current，fla）判定
        const sourceId = this.current?.playAssetId ?? assetId;
        const edge = streamEdge(this.naturalW, this.naturalH, this.viewW, this.viewH, this.zoom);
        const s = await StreamSession.start(sourceId, edge, (f) => this.onStreamFrame(f));
        if (this.current?.assetId !== assetId || this.kind !== "anim") {
          s?.stop(); // await 期间已切走
          return;
        }
        streamSession = s;
        this.segment = s?.segment ?? this.segment;
        if (s) {
          this.playing = true; // 会话默认即播（进入自动播放，同 2026-07-25 用户定）
          s.setRate(this.rate); // 倍速跨素材保持，同步进新会话
          this.animTotal = segmentFrameCount(this.segment) || (this.current?.frameCount ?? 0);
          this.frame = this.segment?.startFrame ?? this.frame;
          // 封面帧定位（对齐原 initFrame 行为：封面帧为初始帧，播放中 seek 后继续播）：
          // thumbFrame 新格式 = `f:{索引}`；存量帧名（{标题}-00042.webp）按尾号兼容解析
          const cover = animCoverIndex(this.current?.thumbFrame);
          if (cover !== null && cover > 0) this.setFrame(cover);
        } else if (this.segment) {
          this.playing = false;
          this.kind = "other";
          this.error = "动图片段帧流不可用，无法按原始帧范围播放";
          return;
        } else {
          this.playing = false;
          const f = await getAssetFilePath(sourceId);
          if (this.current?.assetId !== assetId || this.kind !== "anim") return;
          if (f.path) this.filePath = f.path;
        }
        // 帧条与逐帧笔记不依赖帧清单：帧条按需解码渐进（源 = 播放源），笔记归预览目标自身
        void previewNotesSession(panelId).load();
        try {
          const r = await prepareAnimStrips(sourceId);
          if (this.current?.assetId !== assetId || this.kind !== "anim") return;
          for (const item of r.items) {
            this.animStrips[item.name] = item.stripPath;
          }
        } catch (e) {
          console.warn("动图帧条准备失败（帧条暂缺，播放不受影响）：", e);
        }
      },
      // 帧流帧到达（会话 onFrame 回调）：更新画面帧 / 帧号 / 总帧数 / natural（尺寸热更新时换算保视觉）
      onStreamFrame(f: StreamFrame): void {
        if (this.kind !== "anim") return;
        // seek 期间忽略在途旧帧（帧号不被拉回跳动；目标帧到达后恢复）
        if (seekTarget !== null) {
          if (f.frameIndex !== seekTarget) return;
          seekTarget = null;
        }
        this.animFrame = markRaw(f);
        this.frame = f.frameIndex;
        if (!this.segment && f.frameIndex + 1 > this.animTotal) this.animTotal = f.frameIndex + 1;
        if (this.naturalW !== f.w || this.naturalH !== f.h) this.onImageLoad(f.w, f.h, true);
      },
      // 出帧尺寸跟随显示尺寸（帧短边 × zoom；PreviewPanel watchEffect 驱动：viewW/H/zoom 变化）
      syncAnimSize(): void {
        if (!streamSession || this.kind !== "anim") return;
        streamSession.setSize(streamEdge(this.naturalW, this.naturalH, this.viewW, this.viewH, this.zoom));
      },
      // 帧播放进入点（序列帧/动图共用）：封面帧为初始帧，进入自动播放（2026-07-25 用户定）
      initFrame(): void {
        if (!this.seq) return;
        const start = this.segment?.startFrame ?? 0;
        const end = this.segment?.endFrame ?? this.seq.frames.length - 1;
        const cover = this.seq.thumbFrame
          ? this.seq.frames.findIndex((f) => f.name === this.seq!.thumbFrame)
          : -1;
        this.frame = cover >= start && cover <= end ? cover : start;
        const f = this.seq.frames[this.frame];
        if (f?.fullPath) this.filePath = f.fullPath;
        if (end > start) this.play();
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
        if (this.kind === "anim" && streamSession && !fromStream) return;
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
      seekToSegment(hit: import("../lib/semantic").SegmentHit): void {
        if (this.kind === "video") {
          previewMediaSession(panelId).seek(hit.startMs / 1000);
          return;
        }
        if (this.kind !== "anim" && this.kind !== "sequence") return;
        if (hit.startFrame === null) {
          useToastStore().notify({ message: "该语义片段缺少原始帧索引，无法定位", icon: "fail" });
          return;
        }
        if (this.kind === "anim" && !streamSession) {
          useToastStore().notify({ message: "动图帧流未就绪，暂时无法定位到该帧", icon: "fail" });
          return;
        }
        this.setFrame(hit.startFrame);
      },
      setFrame(i: number): void {
        const start = this.segment?.startFrame ?? 0;
        if (this.kind === "anim" && streamSession) {
          const fallback = this.animTotal > 0 ? this.animTotal - 1 : Math.max(0, i);
          const end = this.segment?.endFrame ?? fallback;
          const t = Math.min(end, Math.max(start, i));
          this.frame = t; // 乐观更新帧号；目标帧到达前忽略在途旧帧
          seekTarget = t;
          streamSession.seek(t);
          return;
        }
        if (!this.seq || this.seq.frames.length === 0) return;
        const end = this.segment?.endFrame ?? this.seq.frames.length - 1;
        this.frame = Math.min(end, Math.max(start, i));
        const f = this.seq.frames[this.frame];
        if (f.fullPath) this.filePath = f.fullPath; // 路径缺失兜底：保持当前画面
        this.preloadAround();
      },
      stepFrame(delta: number): void {
        if (this.kind === "anim" && streamSession) {
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
          if (!streamSession || this.playing) return;
          this.playing = true;
          streamSession.setPlaying(true);
          return;
        }
        if (!this.seq || this.playing) return;
        if (this.segment ? segmentFrameCount(this.segment) < 2 : this.seq.frames.length < 2) return;
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
            const next = this.segment
              ? this.frame >= (this.segment.endFrame ?? this.frame) ? (this.segment.startFrame ?? 0) : this.frame + 1
              : (this.frame + 1) % n;
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
          streamSession?.setPlaying(false);
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
        if (!this.seq || this.seq.frames.length === 0) return;
        const n = this.seq.frames.length;
        const start = this.segment?.startFrame ?? 0;
        const count = segmentFrameCount(this.segment) || n;
        for (const d of [1, 2, -1]) {
          const offset = ((this.frame - start + d) % count + count) % count;
          const f = this.seq.frames[start + offset];
          if (f?.fullPath) {
            const img = new Image();
            img.src = convertFileSrc(f.fullPath);
          }
        }
      },
      setRate(r: number): void {
        this.rate = r;
        if (this.kind === "anim") streamSession?.setRate(r);
      },
      // 当前帧设为预览图（序列帧 / 动图通用，5.2）：后端重建缩略图 + 记封面帧，前端即时换卡片图。
      // 动图传 `f:{索引}`（origin 解码抽帧，2026-08-20 解包废除）；序列帧传 frames/ 内帧名。
      // 结果 toast 反馈（2026-08-20 用户：点击无反馈）；target.modifiedAt 同步 = 缩略图 URL 版本戳
      //（asset 协议同 URL 启发式缓存致静帧不热刷新，用户实机报）；视图外资产兜底全量 refresh
      async setAsThumb(): Promise<void> {
        const p = this.current;
        if (!p || isWsPlacement(p)) return; // ws 合成假 id 禁发 assetId 类命令（步骤 1 只读，cover 钮已滤）
        const frameName =
          this.kind === "anim" ? `f:${this.frame}` : this.seq ? this.seq.frames[this.frame]?.name : undefined;
        if (!frameName) return;
        const toast = useToastStore();
        try {
          // 播放源代理（8.1）：fla 预览时设封面 = 改伴生 hidden 资产的封面帧与缩略图——
          // fla 卡片经视图代理同步换图（target 仍按 fla 定位刷新版本戳与布局尺寸）
          const r = await invoke<{ thumbPath: string; modifiedAt: string; thumbWidth: number; thumbHeight: number }>("set_thumb_frame", {
            assetId: p.playAssetId ?? p.assetId,
            frameName,
          });
          // 即时更新卡片缩略图：路径不变（同词干重建），置空再还原迫使 <img> 重载（asset 协议不走 HTTP 缓存）
          const content = useContentStore();
          const target = content.placements.find(
            (x) => x.assetId === p.assetId && x.folderId === p.folderId,
          );
          if (target) {
            target.modifiedAt = r.modifiedAt; // URL 版本戳（AssetCard / ListRowCard thumbUrl ?v=）
            target.thumbW = r.thumbWidth; // 布局比例跟缩略图走（5.4）
            target.thumbH = r.thumbHeight;
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
      // 显示器全屏（2026-07-25 定案）：预览区 fixed 铺满 + 窗口 fullscreen，Esc 退出；
      // 进入 = 充满铺满（2026-09-17 实机走查：抓手/缩放已退出「适合窗口」跟随的视图，
      // 全屏也要放大填充）——暂存工作视图先 fit()（容器随全屏变化后 RO 再精确跟随），
      // 退出全屏还原取景（全屏 = 临时看大，不改用户工作取景）
      async setFullscreen(on: boolean): Promise<void> {
        if (on === this.fullscreen) return;
        if (on && this.viewable) {
          savedView = { zoom: this.zoom, panX: this.panX, panY: this.panY, fitMode: this.fitMode };
          this.fit();
        }
        this.fullscreen = on;
        await getCurrentWindow().setFullscreen(on);
        if (!on && savedView) {
          this.zoom = savedView.zoom;
          this.panX = savedView.panX;
          this.panY = savedView.panY;
          this.fitMode = savedView.fitMode;
          savedView = null;
        }
      },
      async openDefaultApp(): Promise<void> {
        const p = this.current;
        if (!p) return;
        // ws 按路径版（2026-10-23 文档预览放行：open_path_with_default_app 既有命令，
        // 与 ws 右键菜单「在默认应用打开」同一出口；ws 无最近使用语义，不刷 refreshRecent）
        if (isWsPlacement(p)) {
          if (p.wsPath) await openPathWithDefaultApp(p.wsPath);
          return;
        }
        await invoke("open_with_default_app", { assetId: p.assetId });
        // 成功后直接重拉最近使用（2026-07-29 复验反馈：双保险，同 AssetCard.openDefault）
        await useContentStore().refreshRecent();
      },
    },
  });
}

// —— 会话注册表（per-panel 懒建；panel 关闭时 dropPreviewSession 联动销毁 media/notes 会话）——
// 独立预览窗口的会话属主 id（PreviewWindow 专用，非 dockview panel）
export const WINDOW_PANEL_ID = "__window__";
export type PreviewSession = ReturnType<ReturnType<typeof buildPreviewStore>>;
const sessionCache = new Map<string, PreviewSession>();

// 属主面板的预览会话（无则懒建）
export function previewSession(panelId: string): PreviewSession {
  let s = sessionCache.get(panelId);
  if (!s) {
    s = buildPreviewStore(panelId)();
    sessionCache.set(panelId, s);
  }
  return s;
}

// 当前激活面板的预览会话（只查已建，不懒建；无激活面板 / 未建会话返回 null）
export function activePreviewSession(): PreviewSession | null {
  const id = useTabsStore().activeTabId;
  if (!id) return null;
  return sessionCache.get(id) ?? null;
}

// 按属主面板 id 只查已建会话（不懒建；tabs 定向退预览用——导航目标 panel 可能从未建过会话）
export function peekPreviewSession(panelId: string): PreviewSession | null {
  return sessionCache.get(panelId) ?? null;
}

// 遍历全部已建会话（预览就绪事件扇出用）
export function eachPreviewSession(fn: (s: PreviewSession) => void): void {
  for (const s of sessionCache.values()) fn(s);
}

// 销毁属主面板会话（panel 关闭联动）：先 close() 停引擎/帧流/媒体/笔记，再 dispose 并删缓存
export function dropPreviewSession(panelId: string): void {
  const s = sessionCache.get(panelId);
  if (s) {
    s.close();
    s.$dispose();
    sessionCache.delete(panelId);
    // $dispose 只停 effect scope、不清 pinia state 键（动态 id 键会只增不减）——顺手删 state 键
    const pinia = getActivePinia();
    if (pinia) delete (pinia.state.value as Record<string, unknown>)[s.$id];
  }
  dropPreviewMediaSession(panelId);
  dropPreviewNotesSession(panelId);
}

// 预览就绪事件监听（App 启动注册一次）：PSD 高清 / 帧条小图渐进（序列帧与动图共用事件）——
// 广播事件扇出到全部已建会话，各会话按自身 current 匹配生效
export async function initPreviewListeners(): Promise<void> {
  if (unlistenFull && unlistenStrips && unlistenHidden) return;
  unlistenFull = await listen<{ assetId: string; path: string }>("preview-full-ready", (e) => {
    eachPreviewSession((s) => {
      if (s.current?.assetId === e.payload.assetId && s.kind === "image") {
        s.filePath = e.payload.path; // onImageLoad 按像素比换算保持视觉
      }
    });
  });
  unlistenStrips = await listen<{ assetId: string; items: { name: string; stripPath: string }[] }>(
    "preview-strips-ready",
    (e) => {
      eachPreviewSession((s) => {
        // 播放源代理（8.1）：fla 预览的帧条任务按伴生资产提交——事件 assetId = 伴生 id 同样放行
        if (s.current?.assetId !== e.payload.assetId && s.current?.playAssetId !== e.payload.assetId) return;
        if (s.kind === "anim") {
          // 动图：name = `f:{索引}`，渐进填充帧条缓存
          for (const item of e.payload.items) {
            s.animStrips[item.name] = item.stripPath;
          }
          return;
        }
        if (!s.seq) return;
        for (const item of e.payload.items) {
          const f = s.seq.frames.find((x) => x.name === item.name);
          if (f) f.stripPath = item.stripPath;
        }
      });
    },
  );
  // 主窗口隐藏到托盘（关闭按钮 / 全局热键）：暂停双引擎播放
  // （2026-07-27 用户定：最小化保持播放，隐藏到托盘停止；重新显示后手动再播）
  unlistenHidden = await listen("main-window-hidden", () => {
    eachPreviewSession((s) => s.pause());
    eachPreviewMediaSession((m) => m.pause());
  });
}
