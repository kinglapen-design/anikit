<script setup lang="ts">
// 素材卡片业务壳（7.1 内容容器轮改造）：视觉芯 = cards/FileCard（缩略图满铺 + 底部渐变罩名称/简介 +
// 格式角标统一 FormatBadge 铬），本壳承担业务装配与旧机制对接——
// box 绝对定位（虚拟滚动）/ 悬停播放层查表分发（2026-09-21 R5 注册表化：kind → cardSpecs
// HoverLayerSpec——视频 HoverVideoLayer / 动图 AnimImgLayer / 序列帧 HoverSequenceLayer /
// 音频 HoverAudioLayer，注入 FileCard 默认插槽 = 缩略图上、渐变罩下）/
// 自动播放闸（autoplayGate + IntersectionObserver 可见性）/ 就地重命名（selection.renamingKey 驱动，
// FileCard naming 位）/ 位置徽章 + LinkListPopover / 笔记角标 / 选择模式 picked·picker-self / 拖拽半透明。
// 口径变化：格式角标不再按格式设色（FormatBadge 统一铬，§16——formatBadgeColor 仅留"静态图不出角标"
// null 判定）；位置徽章 14px 右下各 12（kc-corner-badge 位，§16/§18）；悬停播放触发区 = 整卡
//（新稿卡即图满铺，旧仅缩略图区；渐变罩层在播放层之上、pointer-events 穿透——2026-09-08 修复：
// 罩区吃 pointermove 致 scrub 丢跟随，仅名称文本行恢复交互不 scrub，v-tip / 双击重命名保留）；
// 笔记角标保持旧视觉旧位（右上 4——与悬停胶囊 12 位的关系稿未画，记 09 待办）。
// 文本卡（2026-09-22 R6 三变体接线 · text 变体，稿 = 文件卡片-txt-md）：txt / md / docx 内容区 =
// 词干标题 + 正文滚动预览（cardTextFor 懒加载，modifiedAt 版本戳自动失效）；ws 合成项 2026-09-26
// 起接通（by-path 命令 read_text/read_docx_by_path，假 id 仅作前端缓存键；wsPath 缺失仍降级 general）。
// 图片类无悬停控件（2026-08-10 用户定）；查看方式"自动播放动图（含序列帧）/ 自动播放视频"开关不变。
// 音频卡（2026-09-22 R6 三变体接线 · audio 变体，稿 = 文件卡片-音频）：内容区换 FileCard
// kind="audio" 竖线波形（peaks = cardPeaksFor 懒加载 ← get_audio_peaks；旧 PNG 波形缩略图
// 退出卡片层，预览播放条 / 冲突弹窗沿用）——悬停层 HoverAudioLayer 收窄为播放+seek 交互层，
// 状态三通道（playing / progress / duration）回传本壳 → FileCard：扫过 = WaveContent 品牌色
// clip-path（与根数解耦）、胶囊 = overlays MediaCapsule（播放钮 playToggle → 层 togglePlay）；
// ws 合成项 2026-09-26 起接波形填充卡（get_audio_peaks_by_path，产物缓存 .anikit/peaks/
// {urlsafe(rel)}-{mtime}-{size}.json），悬停播放层 / 悬停胶囊仍禁（播放走 assetId 命令 + 无时长数据，
// capsuleText 传空串 = MediaCapsule 整体不渲染）。
// 字体卡（2026-09-22 R6 三变体接线 · font 变体，稿 = 文件卡片-字体）：内容区 = 示例字块以
// 该字体真渲染（cardFontFor 懒加载 = fontInfo + FontFace 注册家族名；示例字按 hasCjk 分流——
// 中文四行 / 拉丁三行，与后端样张字样口径同源）；简介行 = 字重 style；右上 FontInstallButton
//（installed 态；点击 installToggle → install/uninstall_font + toast，同预览面板口径）；
// ws 合成项 2026-09-26 起接通（get_font_info_by_path + FontFace 家族名按编码 relPath），
// 安装钮不出（install/uninstall 是 assetId 系统级命令，installable=false）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import FileCard from "./cards/FileCard.vue";
import LinkListPopover from "./LinkListPopover.vue";
import { type Box } from "../lib/layouts";
import { dragView } from "../lib/dragOperation";
import { hoverLayerSpecFor } from "../lib/cardSpecs";
import { cardTextFor, isTextCard } from "../lib/cardText";
import { cardPeaksFor } from "../lib/cardPeaks";
import { cardFontFor, cardFontSetInstalled, FONT_LINES_CJK, FONT_LINES_LATIN, type CardFont } from "../lib/cardFonts";
import { installFont, uninstallFont } from "../lib/fontOps";
import {
  displayName as assetDisplayName,
  formatBadgeColor,
  isAudioExt,
  isFontExt,
  isSequence,
  isVideoExt,
} from "../lib/formatBadge";
import { formatDuration } from "../lib/formatDuration";
import { formatSize } from "../lib/formatSize";
import { formatDetailDate } from "../lib/detailSections";
import { renamePlacement } from "../lib/placementOps";
import { acquireAutoplay, releaseAutoplay, type AutoplayPool } from "../lib/autoplayGate";
import { useUiStore } from "../stores/ui";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { placementKey, placementRef, useContentStore } from "../stores/content";
import { useSearchStore } from "../stores/search";
import { useToastStore } from "../stores/toast";
import { useWsStore } from "../stores/ws";
import type { PlacementView } from "../stores/content";

const props = withDefaults(
  defineProps<{
    asset: PlacementView;
    box: Box;
    panelId?: string; // 属主面板 id（AssetGrid 注入）——语义命中徽标查 search store per-panel 命中表用
    selected?: boolean;
    renaming?: boolean;
    faded?: "source" | "cut" | null; // 拖拽源半透明 / 被剪切半透明（2026-07-28 复验反馈 6①/8）
  }>(),
  { selected: false, renaming: false, faded: null },
);
const ui = useUiStore();
const thumbUrl = computed(() =>
  // ?v=modifiedAt = 缩略图 URL 版本戳：设封面后 modifiedAt 变 → URL 变 → img 重新加载
  //（asset 协议同 URL 有启发式缓存，2026-08-20 用户实机"静帧不热刷新"修复；query 不影响协议路径解析）；
  // &e=thumbEpoch = 清缓存重建代际戳（2026-10-13 设置面板清 lib-thumbs 闭环：重建不刷 modifiedAt，
  // 同 URL 会滞留旧图/破图；epoch 为 0 也带，URL 形状恒定。网格卡先接——PreviewNavigator /
  // SectionPinned / anikitLink / DupScanCard 不动）
  props.asset.thumbPath
    ? `${convertFileSrc(props.asset.thumbPath)}?v=${encodeURIComponent(props.asset.modifiedAt ?? "")}&e=${ui.thumbEpoch}`
    : null,
);
// 工作空间磁盘实况项（PRD 02 步骤 1，assetId = ws: 合成假 id）：悬停播放层 / 自动播放
// 2026-10-23 放行（PRD 02 技术口径「卡片悬停播放与自动播放」条）——层内按 wsPath 分流按路径命令；
// 序列帧无 ws 语义（无打包概念）不出；wsPath 缺失（rootPath 未就绪窗口期）仍不挂载
const isWs = computed(() => props.asset.assetId.startsWith("ws:"));
// 动图（5.1）：GIF / WEBP 动图；与序列帧区分（序列帧同样带 frameCount）。
// ws 无 animated 元数据：GIF 按扩展名直认（AnimImgLayer 原文件直渲，静态 GIF 罕见且直渲无害）；
// 动态 WebP 需魔数探测 ANIM 位（待办 2026-10-23），未接前 ws WebP 一律无悬停播放
const isAnim = computed(() => props.asset.animated || (isWs.value && props.asset.ext.toLowerCase() === "gif"));
// 序列帧（2026-07-22 用户定）：格式角标显示"序列帧"，名称后缀显示为 .序列帧（仅显示层，数据不动）
const isSeq = computed(() => isSequence(props.asset));

// 悬停播放分发（序列帧归动图范围，2026-07-27 用户定：打包就是为了看动画效果）；
// 图片 / 工程 / 格式图标无悬停控件（2026-08-10 用户定：图片不用悬停）
type HoverKind = "anim" | "sequence" | "video" | "audio" | "other";
const kind = computed<HoverKind>(() => {
  const ext = props.asset.ext.toLowerCase();
  if (isAnim.value) return "anim";
  if (isSeq.value) return "sequence";
  if (isVideoExt(ext)) return "video";
  if (isAudioExt(ext)) return "audio";
  return "other";
});
// 悬停层查表（R5 注册表化，cardSpecs）：未注册（other）= null = 无悬停层；
// autoplay 档 = animPlaying（悬停 || 自动播放名额）挂载并传 auto=!hoverActive，否则仅悬停挂载
const hoverLayer = computed(() => hoverLayerSpecFor(kind.value));

// 悬停态：驱动播放层挂载（视频/音频）与动图播放——7.1 起挂壳根（整卡即图）
const hovered = ref(false);

// —— 文本卡（R6 三变体接线 · text 变体）：txt / md / docx 内容区 = 词干标题 + 正文滚动预览 ——
// ws 合成项 2026-09-26 起接通（by-path 命令）；wsPath 缺失（rootPath 未就绪的窗口期）仍降级 general
const textCardActive = computed(() => isTextCard(props.asset) && (!isWs.value || !!props.asset.wsPath));
const textBody = ref("");
const textTitle = computed(() => props.asset.name.replace(/\.[^.]+$/, ""));
// 懒拉正文（虚拟滚动渲染即视口内）；modifiedAt 随 watch 源变化 → 编辑保存后自动重拉
watch(
  [() => props.asset.assetId, () => props.asset.modifiedAt, textCardActive],
  async ([id, , active]) => {
    if (!active) {
      textBody.value = "";
      return;
    }
    const text = await cardTextFor(props.asset);
    if (props.asset.assetId !== id) return; // await 期间卡片已被虚拟滚动复用到别的资产
    textBody.value = text;
  },
  { immediate: true },
);
// —— 音频卡（R6 三变体接线 · audio 变体）：内容区 = 竖线波形（peaks 懒加载，口径仿文本卡）——
// ws 合成项 2026-09-26 起接通（get_audio_peaks_by_path）；悬停播放层 / 胶囊 2026-10-23 放行
const audioCardActive = computed(() => kind.value === "audio" && (!isWs.value || !!props.asset.wsPath));
const audioPeaks = ref<number[]>([]);
watch(
  [() => props.asset.assetId, () => props.asset.modifiedAt, audioCardActive],
  async ([id, , active]) => {
    if (!active) {
      audioPeaks.value = [];
      return;
    }
    const peaks = await cardPeaksFor(props.asset);
    if (props.asset.assetId !== id) return; // await 期间卡片已被虚拟滚动复用到别的资产
    audioPeaks.value = peaks;
  },
  { immediate: true },
);
// 悬停播放层状态回传（HoverAudioLayer → FileCard）：playing = 扫过恒显 + 胶囊图标；
// progress 0..1 = 扫过裁切；duration = 胶囊时长兜底（asset.duration 缺时）
const audioPlaying = ref(false);
const audioProgress = ref(0);
const audioDuration = ref(0);
const hoverLayerInst = ref<{ togglePlay?: () => void }>();
// 胶囊时长：ws 无时长元数据（2026-10-23 放行）——未悬停装载前不出胶囊（不显示 0:00），
// 悬停后由元素元数据回填（duration emit）；库侧时长缺失边界同愈（原显示 0:00）
const audioCapsule = computed(() => {
  const d = props.asset.duration ?? (audioDuration.value > 0 ? audioDuration.value : 0);
  return d > 0 ? formatDuration(d) : "";
});
// 胶囊播放钮（FileCard playToggle 上抛）→ 悬停层 togglePlay 回转
function onPlayToggle(): void {
  if (audioCardActive.value) hoverLayerInst.value?.togglePlay?.();
}
// —— 字体卡（R6 三变体接线 · font 变体）：示例字以该字体真渲染（FontFace 会话级常驻注册）——
// 示例字分流（稿注「日/韩环境更换」的字体维度落地）：cmap 覆盖中文 → 中文四行（稿值），
// 否则拉丁三行（与后端样张字样同口径）；系统语言维度（日韩替换文本）稿未给内容仍待供稿
// ws 合成项 2026-09-26 起接通（get_font_info_by_path）；安装钮模板侧 installable=false 不出
const fontCardActive = computed(() => isFontExt(props.asset.ext.toLowerCase()) && (!isWs.value || !!props.asset.wsPath));
const cardFont = ref<CardFont | null>(null);
watch(
  [() => props.asset.assetId, () => props.asset.modifiedAt, fontCardActive],
  async ([id, , active]) => {
    if (!active) {
      cardFont.value = null;
      return;
    }
    const font = await cardFontFor(props.asset);
    if (props.asset.assetId !== id) return; // await 期间卡片已被虚拟滚动复用到别的资产
    cardFont.value = font;
  },
  { immediate: true },
);
const fontLines = computed(() => (cardFont.value?.info.hasCjk ? FONT_LINES_CJK : FONT_LINES_LATIN));
const fontBusy = ref(false); // 安装 / 卸载在途（同 PreviewFont 口径）
// 字体安装钮（FileCard installToggle 上抛）→ install/uninstall_font + 缓存写回 + toast
async function onInstallToggle(): Promise<void> {
  const font = cardFont.value;
  if (!fontCardActive.value || !font || fontBusy.value) return;
  fontBusy.value = true;
  const id = props.asset.assetId;
  try {
    const r = font.info.installed ? await uninstallFont(id) : await installFont(id);
    if (props.asset.assetId !== id) return;
    cardFont.value = { ...font, info: { ...font.info, installed: r.installed } };
    cardFontSetInstalled(id, r.installed);
    useToastStore().notify({ message: r.installed ? "已安装到系统" : "已从系统卸载", icon: "success" });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
  } finally {
    fontBusy.value = false;
  }
}
// 拖拽期间全局禁用悬停层（2026-07-29 复验二轮 5，JS 闸）：拖拽激活时视同未悬停——
// 播放层 v-if 条件即刻转假，走既有退出路径（卸载停播 / 动图还原封面 / 序列帧停轮播）
const hoverActive = computed(() => hovered.value && !dragView.active);
// 悬停层卸载（mouseleave / 拖拽起）即停播、组件无 emit 出口——状态复位（下次 hover 从 0 扫过）
watch(hoverActive, (v) => {
  if (!v) {
    audioPlaying.value = false;
    audioProgress.value = 0;
  }
});

// —— 就地重命名（6.1 C2，2026-07-28 定案）：输入态在 FileCard（naming prop），
// Enter / 失焦 = 提交（空或不变不动），Esc = 取消；滚出虚拟滚动销毁 = 取消（AssetGrid 先清
// renamingKey，blur 晚于清理到达时按 key 失配判取消）；确认调 rename_placement 后 content.refresh()（在 ops 内）
const selection = useSelectionStore();
const picker = usePickerStore();
const content = useContentStore();
const cardKey = placementKey(props.asset);
// 选择模式（6.2 一轮，PRD 03_数据链接 04 四重标识之四）：已选卡片压暗（30% 档）+ 品牌绿大勾；
// 行级匹配（2026-09-19 BUG 3 三元组独立选择 → 2026-10-11 补 name 维：同夹重复链接各自亮勾）
const picked = computed(() => picker.active && picker.isPicked("asset", props.asset.assetId, props.asset.folderId, props.asset.name));
// 发起文件自身（2026-08-24 用户定）：剪切档半透明 + 禁选（togglePick 同拦），视觉明确"不可选"
const pickerSelf = computed(() => picker.active && picker.origin?.kind === "asset" && picker.origin.id === props.asset.assetId);
// 重命名提交 / 取消（FileCard nameCommit / nameCancel 上抛；空/不变判定在本壳，TreeRowItem 口径）；
// 工作空间（PRD 02 步骤 2）走 ws_rename 按路径改名（assetId = ws: 合成假 id 严禁 rename_placement）
async function onRenameCommit(name: string): Promise<void> {
  selection.renamingKey = null;
  if (!name || name === props.asset.name) return;
  if (isWs.value) {
    try {
      await useWsStore().rename(props.asset.assetId.slice(3), name);
    } catch (e) {
      useToastStore().notify({ message: String(e), icon: "fail" });
    }
    return;
  }
  await renamePlacement(placementRef(props.asset), name);
}
function onRenameCancel(): void {
  selection.renamingKey = null;
}
// 双击名称 = 就地重命名（2026-07-29 复验三轮，renamingKey 既有机制——
// 双击第一下 click 已把卡片单选入域，此处仅置重命名目标）；
// 回收站视口只读不可重命名（2026-07-29 二轮，与 F2 / 菜单同口径闸）；
// 工作空间（PRD 02 步骤 2）放开：按路径 ws_rename（提交分支在 onRenameCommit）
function startRename(): void {
  if (useContentStore().isRecycleView) return;
  selection.renamingKey = cardKey;
}

// 动图播放（5.3）：悬停或"自动播放动图"开关（序列帧同归此开关，2026-07-27 用户定）；
// 2026-09-02 起由 AnimImgLayer 原生 <img> 播放（阶段四帧流卡片层退役，回滚原生）；
// 进入延迟分档收编 useHoverPlayer：悬停 300 / 自动播放 100
// 自动播放数量闸（2.3）：开关驱动的自动播放申请名额（满员排队 FIFO 唤醒），悬停播放直通——
// 小卡片高密度时窗口内 200+ 卡片同时解码致全局卡顿（hover 目录树滞后）；
// 动图 / 视频 / 序列帧三处自动播放共用本卡 gateRequested/autoGranted（一卡同时至多一种 kind）。
// 可见性联动（2026-08-12 用户实机：纯 FIFO 名额被缓冲区不可见卡占住、头尾可见卡排队等死）——
// IntersectionObserver 判定真实可见，仅可见才申请、滚出视口立即释放（视口外暂停的落地）
const autoGranted = ref(false);
let gateRequested = false;
const cardVisible = ref(false);
const rootEl = ref<HTMLElement>();
let visibilityObs: IntersectionObserver | undefined;
const gatePool = computed<AutoplayPool>(() => (kind.value === "video" ? "video" : "anim"));
const wantAutoplay = computed(() => {
  if (isWs.value && !props.asset.wsPath) return false; // wsPath 未就绪窗口期不申请（2026-10-23 放行后仅余此闸）
  if (!cardVisible.value) return false;
  if (kind.value === "anim" || kind.value === "sequence") return ui.autoPlayGif;
  if (kind.value === "video") return ui.autoPlayVideo;
  return false;
});
watch(
  wantAutoplay,
  (want) => {
    if (want && !gateRequested) {
      gateRequested = true;
      autoGranted.value = acquireAutoplay(cardKey, gatePool.value, () => {
        autoGranted.value = true;
      });
    } else if (!want && gateRequested) {
      gateRequested = false;
      releaseAutoplay(cardKey, gatePool.value);
      autoGranted.value = false;
    }
  },
  { immediate: true },
);
// 工作空间缩略图视口优先（2026-09-26 性能优化 C）：ws 卡真实可见且缺缩略图 → 上报 ws store
// 合批后 priority 插队队首（恢复滚动位置落在目录中部时，可见卡不等前面全部生成完；
// 其余缺失项仍由 loadDir 全量请求排队列尾部后台补发）；库资产 wsPath 恒 null 不受影响
watch([cardVisible, () => props.asset.thumbPath], ([visible, thumb]) => {
  if (!visible || thumb !== null || !isWs.value) return;
  const p = props.asset.wsPath;
  if (p) useWsStore().requestVisibleThumbs([p]);
});
const animPlaying = computed(() => hoverActive.value || autoGranted.value);
// 动图播放（2026-09-02 回滚原生 <img>，帧流卡片层退役——实机 CPU/风扇失控，详见 AnimImgLayer 头注）：
// animPlaying 且 kind==='anim' 时模板挂载 AnimImgLayer；序列帧归 HoverSequenceLayer。
onMounted(() => {
  // root null = webview 视口：网格 BUFFER 缓冲区内但视口外的卡判不可见不占名额
  visibilityObs = new IntersectionObserver((entries) => {
    cardVisible.value = entries[0]?.isIntersecting ?? false;
  });
  if (rootEl.value) visibilityObs.observe(rootEl.value);
});
onBeforeUnmount(() => {
  visibilityObs?.disconnect();
  if (gateRequested) {
    gateRequested = false;
    releaseAutoplay(cardKey, gatePool.value);
  }
});

// 角标文案：音频带 BPM 显示"格式 / BPM: 值"（Eagle 同格式，2026-07-26 用户截图定稿）
const badge = computed(() => {
  if (isSeq.value) return "序列帧";
  const ext = props.asset.ext.toUpperCase();
  return props.asset.bpm != null ? `${ext} / BPM: ${Math.round(props.asset.bpm)}` : ext;
});
// 角标显隐（2026-08-30 口径变化）：格式角标统一 FormatBadge 铬（§16），不再按格式设色——
// formatBadgeColor 仅留 null 判定（静态图不出角标，色值废弃）；查看方式 showBadge 开关不变
const showFormatBadge = computed(
  () => ui.showBadge && formatBadgeColor(props.asset.ext, isSeq.value, isAnim.value) !== null,
);
// —— 语义命中徽标（2026-11-02 用户实机：语义搜索"能定位到视频，但卡片上看不到时间戳"）——
// 语义结果态直显命中段时间：best 段起点 mm:ss（formatDuration）+ 多段 "+N"；跳转本体不变——
// 双击卡片 → preview.open → seekToSegment 定位到命中段（preview.ts:221，徽标纯展示不拦事件）
const search = useSearchStore();
const semanticHit = computed(() => (props.panelId ? search.semanticHitFor(props.panelId, props.asset.assetId) : null));
const semanticBadge = computed(() => {
  const hit = semanticHit.value;
  if (!hit) return "";
  const time = formatDuration(hit.best.startMs / 1000);
  const extra = hit.segments.length - 1;
  return extra > 0 ? `${time} +${extra}` : time;
});
// —— 位置徽章（6.2 四轮走查③：「有链接」（活位置 > 1）才上徽章——origin ★ / 链接 icon，
//    单位置文件无徽章；⑤悬停 = LinkListPopover 全部链接列表）——
// 视图菜单「显示链接标识」总开关（ui.showLinkBadge，2026-09-06）关时两侧都不上徽章
const linkCount = computed(() => content.placementCountByAsset.get(props.asset.assetId) ?? 0);
const showOriginBadge = computed(() => ui.showLinkBadge && props.asset.origin && linkCount.value > 1);
const showLinkBadge = computed(() => ui.showLinkBadge && !props.asset.origin && linkCount.value > 1);
const badgeAnchorRef = ref<HTMLElement>();
const linkListOpen = ref(false);
let linkListCloseTimer = 0;
function openLinkList(): void {
  window.clearTimeout(linkListCloseTimer);
  linkListOpen.value = true;
}
function closeLinkListSoon(): void {
  // hover 桥 120ms 容错：徽章 → 浮层间移动不闪关（浮层 mouseenter 回调再取消）
  linkListCloseTimer = window.setTimeout(() => (linkListOpen.value = false), 120);
}
function onPopoverHover(inside: boolean): void {
  if (inside) window.clearTimeout(linkListCloseTimer);
  else closeLinkListSoon();
}
onBeforeUnmount(() => window.clearTimeout(linkListCloseTimer));
const displayName = computed(() => assetDisplayName(props.asset, ui.showExt));
// 简介行：字体卡 = 字重 style（稿「Light」；未载完 / 无样式名回退通用档），其余按查看方式档
const cardInfo = computed(() => {
  if (!ui.showBrief) return "";
  if (fontCardActive.value) return cardFont.value?.info.style || meta.value;
  return meta.value;
});
// 简介行内容按查看方式「显示简介」下拉分流（2026-08-19 接入）：
// 尺寸 = 音视频时长（mm:ss，5.1）/ 动图与其他按 w×h；
// 空值兜底（2026-08-19 用户定）：档内无数据一律回退文件大小（恒有）
const meta = computed(() => {
  if (ui.briefMode === "fileSize") return formatSize(props.asset.size);
  if (ui.briefMode === "createdAt")
    return props.asset.createdAt ? formatDetailDate(props.asset.createdAt) : formatSize(props.asset.size);
  if (ui.briefMode === "modifiedAt") return formatDetailDate(props.asset.modifiedAt);
  if (props.asset.duration != null && !isAnim.value) return formatDuration(props.asset.duration);
  return props.asset.w != null && props.asset.h != null
    ? `${props.asset.w} x ${props.asset.h}`
    : formatSize(props.asset.size);
});
</script>

<template>
  <div
    ref="rootEl"
    class="asset-card"
    :class="{ picked, 'picker-self': pickerSelf, 'fade-source': faded === 'source', 'fade-cut': faded === 'cut' }"
    :style="{
      left: `${box.left}px`,
      top: `${box.top}px`,
      width: `${box.width}px`,
      height: `${box.height}px`,
    }"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <!-- 视觉芯 = cards/FileCard（general 壳：图满铺 + 渐变罩名称/简介 + 格式角标统一铬）；
         悬停播放层注入默认插槽（缩略图上、渐变罩下）；video/audio 的悬停胶囊 UI 换代留播放层组件内 -->
    <FileCard
      :kind="textCardActive ? 'text' : audioCardActive ? 'audio' : fontCardActive ? 'font' : 'general'"
      :name="displayName"
      :info="cardInfo"
      :badge="showFormatBadge ? badge : ''"
      :thumb-url="thumbUrl ?? ''"
      :thumb-fit="asset.thumbKind === 'formatIcon' ? 'icon' : ui.layoutMode === 'grid' ? 'contain' : 'cover'"
      :text-title="textCardActive ? textTitle : ''"
      :text-body="textCardActive ? textBody : ''"
      :peaks="audioCardActive ? audioPeaks : []"
      :capsule-text="audioCardActive ? audioCapsule : ''"
      :playing="audioCardActive && audioPlaying"
      :progress="audioCardActive ? audioProgress : 0"
      :font-lines="fontCardActive ? fontLines : []"
      :font-family="fontCardActive ? (cardFont?.family ?? '') : ''"
      :installed="fontCardActive && (cardFont?.info.installed ?? false)"
      :installable="!isWs"
      :selected="selected"
      :naming="renaming"
      :picked="picked"
      @name-edit="startRename"
      @name-commit="onRenameCommit"
      @name-cancel="onRenameCancel"
      @play-toggle="onPlayToggle"
      @install-toggle="onInstallToggle"
    >
      <!-- 悬停播放层 = cardSpecs 查表分发（R5 注册表化，等价原四分支链）：
           autoplay 档（video / anim / sequence）挂 animPlaying 并传 auto=!hoverActive；
           audio 档仅悬停挂载不传 auto；other 未注册 = 无层。ws 合成项 2026-10-23 放行
           （层内 wsPath 分流按路径命令；序列帧无 ws 语义，ws 项天然不命中 sequence kind）。
           状态三通道（playing / progress / duration）仅 HoverAudioLayer 发，他层监听无害 -->
      <component
        :is="hoverLayer?.layer"
        v-if="hoverLayer && (hoverLayer.autoplay ? animPlaying : hoverActive)"
        ref="hoverLayerInst"
        v-bind="hoverLayer.autoplay ? { asset, auto: !hoverActive } : { asset }"
        @playing="audioPlaying = $event"
        @progress="audioProgress = $event"
        @duration="audioDuration = $event"
      />
      <!-- 位置徽章（6.2 四轮走查③⑤）：origin ★ / 链接 icon（活位置 > 1 才上），14px 右下各 12
           （kc-corner-badge 位，§16/§18）；悬停 = 全部链接列表浮层（LinkListPopover，可点行跳转）。
           入 FileCard badges 槽（clip 外根内）：徽章成 .kc-media 后代，hover 徽章时卡 hover 态保持
           （位移/阴影/描边不回退——旧在壳层时徽章非 .kc-media 子树，hover 即脱节，2026-09-23 用户实机修） -->
      <template #badges>
        <span
          v-if="showOriginBadge || showLinkBadge"
          ref="badgeAnchorRef"
          class="card-badges"
          :data-badge="showOriginBadge ? 'origin' : 'link'"
          @mouseenter="openLinkList"
          @mouseleave="closeLinkListSoon"
        >
          <AppIcon v-if="showOriginBadge" name="original-file-badge" :size="14" />
          <AppIcon v-else name="linked-file-badge" :size="14" class="link-badge-icon" />
        </span>
      </template>
    </FileCard>
    <!-- 笔记徽章（2026-09-09 四轮用户定稿）：kc-float 角标铬「笔记 N」（笔记 = 雅黑 / 数量 = mono，
         数字档 12/16 bold 二级）——旧 16px 品牌圆点数字右上口径作废（压悬停胶囊）；
         位置 = 名称上方 6px、左缘与名称对齐（罩 padding 左 12）；隐藏简介时随名称行下移（36→56 档差） -->
    <span v-if="asset.noteCount" class="note-badge kc-float" :class="{ 'no-brief': !ui.showBrief }">
      <span class="nb-label">笔记</span><span class="nb-count">{{ asset.noteCount }}</span>
    </span>
    <!-- 语义命中徽标（2026-11-02 用户实机定）：语义搜索结果态直显命中段时间（best 段起点 mm:ss，
         多段 "+N" 并记）——kc-float 铬右上各 12（FormatBadge 左上位的镜像）；仅视频/动图/序列帧
         有命中（音频/字体不在分析范围，右上无胶囊/安装钮冲突） -->
    <span v-if="semanticBadge" class="semantic-badge kc-float">{{ semanticBadge }}</span>
    <LinkListPopover
      v-if="linkListOpen"
      :asset="asset"
      :anchor="badgeAnchorRef"
      @close="linkListOpen = false"
      @hover="onPopoverHover"
    />
    <!-- 选择模式已选大勾（6.2 一轮）：绿圆白勾居中整卡（新稿卡即图）；不随压暗（壳层，压暗在卡内） -->
    <span v-if="picked" class="pick-check">
      <AppIcon name="check" :size="22" />
    </span>
  </div>
</template>

<style scoped>
/* 壳：虚拟滚动 box 绝对定位；选中环 / 常态描边 / hover 位移全归 FileCard（cards.css kc 族） */
.asset-card {
  position: absolute;
  cursor: default;
}
/* 拖拽源 / 被剪切整体半透明（2026-07-28 复验反馈 6①/8，速查 §5 档） */
.asset-card.fade-source {
  opacity: 0.4;
}
.asset-card.fade-cut {
  opacity: 0.5;
}
/* 笔记徽章（2026-09-09 四轮用户定稿）：角标同款 kc-float 铬（padding 2/6 半透明-2 + 立体感 + blur 小 +
   圆角 4）；内容「笔记 N」——标签雅黑 / 数量 mono，数字档 12/16 bold 亮色/二级。
   位置 = 名称上方 6px、左缘与名称对齐（12）；隐藏简介时名称行下移 20（简介行 16 + gap 4），徽章跟随。
   底缘距卡底 = 罩底 padding 12 + 名称行 18 + 间距 6 = 36；有简介再 +20 = 56。 */
.note-badge {
  position: absolute;
  left: var(--n-space-12);
  bottom: 56px;
  z-index: 6;
  gap: var(--n-space-4); /* 「笔记」与数量间距 */
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
  pointer-events: none;
}
.note-badge.no-brief {
  bottom: 36px;
}
.nb-label {
  font-family: var(--n-font-family); /* 雅黑（正文档族） */
}
.nb-count {
  font-family: var(--n-font-mono); /* 数量 = Fira Mono（数字档族） */
}
/* 语义命中徽标（2026-11-02 定）：kc-float 铬（同笔记徽章 / FormatBadge 同源），右上各 12
   （FormatBadge 左上位的镜像）；mono 数字档 12/16 bold 二级（同笔记数量档）；纯展示不拦事件 */
.semantic-badge {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 6;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
  pointer-events: none;
}
/* 位置徽章：14×14，卡片右下各 12（kc-corner-badge 位）；可悬停出链接列表浮层。
   色档 = 亮色/一级（2026-09-23 用户实机定：三级在深底图上看不清；定位/渲染已入 FileCard badges 槽） */
.card-badges {
  position: absolute;
  right: var(--n-space-12);
  bottom: var(--n-space-12);
  z-index: 6;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px;
  color: var(--n-l1);
}
/* 壳层附件（笔记角标 / 已选大勾）随卡芯 hover 位移 -4px（2026-08-30 用户实机：
   徽章没随卡上移——卡芯 .kc-media:hover translateY(-4px) 250ms，壳层附件同参数跟随）。
   位置徽章不入此列（2026-09-23）：已入 FileCard badges 槽 = 卡芯子树，随父级位移天然同步——
   再叠壳层补偿会走双份（-8px 比卡片幅度大，同日走查反馈） */
.note-badge,
.semantic-badge,
.pick-check {
  transition: transform var(--n-dur-250) var(--n-ease-in-out);
}
.asset-card:hover .note-badge {
  transform: translateY(-4px);
}
.asset-card:hover .semantic-badge {
  transform: translateY(-4px);
}
.asset-card:hover .pick-check {
  transform: translate(-50%, calc(-50% - 4px));
}
/* 选择模式已选（§12）：位置徽章（卡内 badges 槽，clip 外不被卡内压暗规则覆盖）/ 壳层笔记角标
   同压暗 30% 档（卡内压暗归 FileCard picked）；大勾不压（在壳层独立） */
.asset-card.picked .card-badges,
.asset-card.picked .note-badge,
.asset-card.picked .semantic-badge {
  opacity: 0.3;
}
/* 选择模式发起文件自身（2026-08-24 用户定）：剪切档半透明 + 禁选光标（不可选的视觉信号） */
.asset-card.picker-self {
  opacity: 0.5;
  cursor: not-allowed;
}
/* 已选大勾（2026-08-23 用户贴稿）：成功色圆底 44 + check 22，居中整卡（新稿卡即图满铺） */
.pick-check {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  z-index: 9;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: var(--n-success);
  color: var(--n-l1);
  pointer-events: none;
}
</style>
