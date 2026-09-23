<script setup lang="ts">
// 链接关系 tab（7.1 二十三轮贴稿，稿 = 右侧边栏-详情面板-链接关系 + 链接/音频/关系/缩略图/污染文件 五区块稿）：
// 区块壳 = padding 12 / gap 12 / border-top 通用/三级；标题行 = items/SectionTitle block 型
//（标题 + expand 折叠钮 + 右侧动作组；稿按钮组 = 链接+添加两钮 → withMore=false；
// link 钮 = 剪贴板有链接才显示（稿 prose），检测业务未接 → 本轮不渲染，09 待办）。
// **标题行恒渲染 + 折叠钮/动作组常显**（staticActions，2026-08-31 用户实机定：「没有内容标题也要在，
// 标题上的添加按钮本身就是入口」——区块稿常态帧即画出按钮组；空数据仅不渲染内容容器，
// 展开/折叠零高度变化旧口径不变）。
// 五区块业务（旧 SectionLinks / SectionAudio / SectionRelations 移植，逻辑不变）：
//   链接 = LinkFileItem 列表（当前目录置顶 / 档案目录 .档案 后缀 / 添加 = 链接管理弹窗（仅文件形态）/
//         × = 移除该条链接 / 点 item = 跳转定位）；
//   音频 = LinkAudioItem（voice 关系恒单向：使用方 → 音频；单例 audio 同时只播一条；添加 = 选择模式过滤
//         非音频；当前目标是音频 → 标题「使用者」+ 使用方行（无添加钮，只展示/跳转））；
//   关系 = LinkRelationItem（kind=free；点 chip = 称谓输入态二次修改 / 添加 = 选择模式 / × = 删除 /
//         点 item = 跳转定位）；
//   缩略图 = 伴生缩略图（8.1，PRD 02-15，2026-09-23 接业务）：调用侧卡 = 本工程绑定的 hidden
//         伴生资产（名称 + 引用数 users，稿「雪牧_自动.webp + 2」）；被调用侧 = 当前目标是 hidden
//         缩略图时列引用它的使用方行（无 count）；点 item = 跳转定位（伴生资产 hidden →
//         「缩略图」快速访问视图选中）；× = 解除绑定（unbind_thumb_asset，仅清工程侧引用）；
//         添加 = 选择模式 thumb 用途（picker 第五用途，单选直回 = set_thumb_asset 绑定/换绑）；
//   污染文件 = 占位（info 文件夹异物文件业务后续立项，同上；稿标题行无动作组 → actions=false）。
// 排序 = 面板内拖标题行（本 tab 唯一可排序组，2026-08-31 用户定；DetailSections 旧 secDrag 让位范式移植，
// 松手写 ui.detailLinksOrder 持久化）；折叠 = 标题行点击（会话级，dragView.suppressClick 守卫在调用方）。
import { computed, nextTick, onBeforeUnmount, reactive, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import SectionTitle from "../items/SectionTitle.vue";
import LinkFileItem from "../items/LinkFileItem.vue";
import LinkAudioItem from "../items/LinkAudioItem.vue";
import LinkRelationItem from "../items/LinkRelationItem.vue";
import LinkThumbItem from "../items/LinkThumbItem.vue";
import TwoToneIcon from "../items/TwoToneIcon.vue";
import AppIcon from "../AppIcon.vue";
import {
  DETAIL_LINK_BLOCK_TITLES,
  findFolderNode,
  type DetailLinkBlockId,
  type DetailTarget,
  type PlacementLink,
} from "../../lib/detailSections";
import { syncAssetLinks, unbindThumbAsset } from "../../lib/placementOps";
import {
  jumpToAssetPlacement,
  jumpToRelationTarget,
  relationsForTarget,
  removeRelation,
  updateRelation,
  type PickerTarget,
  type RelationView,
} from "../../lib/relationOps";
import { getMediaSource } from "../../lib/mediaOps";
import { isAudioExt } from "../../lib/formatBadge";
import { currentLibrary } from "../../lib/libraryOps";
import { dragView } from "../../lib/dragOperation";
import { useContentStore } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { usePickerStore } from "../../stores/picker";
import { useTabsStore } from "../../stores/tabs";
import { useToastStore } from "../../stores/toast";
import { useUiStore } from "../../stores/ui";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
const ui = useUiStore();
const content = useContentStore();
const selection = useSelectionStore();
const picker = usePickerStore();
const toast = useToastStore();

const file = computed(() => (props.target.kind === "file" ? props.target : null));

// —— 区块折叠（会话级，默认展开；空数据不渲染内容容器——展开/折叠零高度变化旧口径） ——
const openMap = reactive<Record<string, boolean>>({});
function isOpen(id: string): boolean {
  return openMap[id] !== false;
}
function toggleOpen(id: string): void {
  if (dragView.suppressClick) return; // 拖拽让位松手后紧随 click 误折叠守卫（调用方口径）
  openMap[id] = !isOpen(id);
}

// —— 排序（ui.detailLinksOrder 持久化；块高不定，DOM 快照中线判定，旧 DetailSections 范式） ——
const order = computed(() => ui.detailLinksOrder as DetailLinkBlockId[]);
const tabRef = ref<HTMLElement>();
const secDrag = reactive({ id: "", startY: 0, dy: 0, from: -1, to: -1 });
const secSettling = ref(false); // 松手归位帧：禁块 transform 过渡
let secMoved = false;
let secSpans: { mid: number }[] = [];
let secStep = 0;

function blockEls(): HTMLElement[] {
  const root = tabRef.value;
  if (!root) return [];
  return [...root.querySelectorAll<HTMLElement>(":scope > .lb-block")];
}

function onBlockPointerDown(e: PointerEvent): void {
  if (e.button !== 0) return;
  const t = e.target as HTMLElement;
  if (t.closest(".act") || t.closest(".chev")) return; // 动作钮 / 展开钮上不启动拖拽
  const title = t.closest(".st");
  if (!title) return; // 仅区块标题行可拖
  const block = title.closest(".lb-block");
  if (!block) return;
  const els = blockEls();
  const from = els.indexOf(block as HTMLElement);
  if (from < 0) return;
  let acc = 0;
  secSpans = els.map((el) => {
    const h = el.offsetHeight;
    const s = { mid: acc + h / 2 };
    acc += h;
    return s;
  });
  secStep = els[from]?.offsetHeight ?? 0;
  Object.assign(secDrag, { id: order.value[from] ?? "", startY: e.clientY, dy: 0, from, to: from });
  secMoved = false;
  window.addEventListener("pointermove", onSecDragMove);
  window.addEventListener("pointerup", onSecDragEnd, { once: true });
  window.addEventListener("blur", onSecDragCancel, { once: true });
  document.addEventListener("mouseleave", onSecDragCancel, { once: true });
}

function onSecDragMove(e: PointerEvent): void {
  if (!secDrag.id) return;
  if ((e.buttons & 1) === 0) {
    onSecDragCancel(); // 僵尸自清：pointerup 丢失（窗外松手等）
    return;
  }
  secDrag.dy = e.clientY - secDrag.startY;
  if (Math.abs(secDrag.dy) > 3) secMoved = true;
  const c = (secSpans[secDrag.from]?.mid ?? 0) + secDrag.dy;
  let to = secDrag.from;
  for (let i = 0; i < secSpans.length; i++) {
    if (i > secDrag.from && c > secSpans[i].mid) to = i;
    // 上移：首个被越过的中线即落点（须 break，跨多块上移会少落一位——旧 DetailSections 踩坑已修）
    if (i < secDrag.from && c < secSpans[i].mid) {
      to = i;
      break;
    }
  }
  secDrag.to = to;
}

function onSecDragEnd(): void {
  offSecDrag();
  const { from, to } = secDrag;
  const moved = secMoved;
  secDrag.id = "";
  if (!moved || from < 0 || to === from) return;
  const draggedId = order.value[from];
  if (draggedId) ui.moveDetailLinkTo(draggedId, to);
  // 抑制紧随 click 误折叠（与引擎 suppressClick 同口径）
  dragView.suppressClick = true;
  setTimeout(() => (dragView.suppressClick = false), 0);
  // 松手即到位：让位块视觉本已在目标位，清 transform 必须禁过渡
  secSettling.value = true;
  setTimeout(() => (secSettling.value = false), 50);
}

function onSecDragCancel(): void {
  offSecDrag();
  secDrag.id = "";
}

function offSecDrag(): void {
  window.removeEventListener("pointermove", onSecDragMove);
  window.removeEventListener("pointerup", onSecDragEnd);
  window.removeEventListener("blur", onSecDragCancel);
  document.removeEventListener("mouseleave", onSecDragCancel);
}

// 让位位移：拖动块跟随指针（无过渡、置顶、g1 底盖缝）；被跨越块反向让出一步
function blockStyle(i: number): string {
  if (!secDrag.id) return "";
  if (i === secDrag.from) {
    return `transform: translateY(${secDrag.dy}px); transition: none; position: relative; z-index: 2; background: var(--n-g1);`;
  }
  if (secSettling.value) return "";
  const { from, to } = secDrag;
  if (from < to && i > from && i <= to) return `transform: translateY(${-secStep}px)`;
  if (to < from && i >= to && i < from) return `transform: translateY(${secStep}px)`;
  return "";
}

// 区块标题（音频区块在使用者视角换「使用者」，旧业务口径）
const isAudioTarget = computed(() => file.value !== null && isAudioExt(file.value.placement.ext));
function titleOf(id: DetailLinkBlockId): string {
  if (id === "audio" && isAudioTarget.value) return "使用者";
  return DETAIL_LINK_BLOCK_TITLES[id];
}

// 区块动作组显隐（稿口径）：污染文件恒无（占位）；音频区块在「使用者」视角（当前目标是音频）
// 同样无添加钮——voice 语义 = 使用方 → 音频，从音频发起只会造出 音频→音频 的语义乱链
//（"使用者"区块混入它自己"链接"的音频，2026-09-19 用户实机"链接音频加了不显示/对不上"）
function hasActions(id: DetailLinkBlockId): boolean {
  if (id === "pollution") return false;
  if (id === "audio" && isAudioTarget.value) return false;
  return true;
}

// ================= 链接区块（SectionLinks 移植） =================
const linkItems = computed<PlacementLink[]>(() => {
  if (!file.value) return [];
  const list = file.value.detail.placements;
  // 当前目录的 item 永远第一行（6.2 四轮走查②；folderId + name 双键匹配，重复链接不错位）
  const cur = file.value.placement;
  const i = list.findIndex((p) => (p.folderId ?? "root") === cur.folderId && p.name === cur.name);
  return i > 0 ? [list[i], ...list.slice(0, i), ...list.slice(i + 1)] : list;
});
// 所属目录显示名：path 末段；所在目录是档案 → 「名称.档案」（2026-08-15 定案，前端树判定）
function dirName(p: PlacementLink): string {
  const name = p.path[p.path.length - 1] ?? "";
  if (p.folderId && findFolderNode(content.folders, p.folderId)?.kind === "archive") {
    return `${name}.档案`;
  }
  return name;
}
// 添加 = 链接管理弹窗（17 稿；仅文件形态——档案的链接管理随档案多位置立项）
function openLinkManage(): void {
  if (!file.value) return;
  selection.linkManageDraft = { assetId: file.value.placement.assetId, name: file.value.placement.name };
}
// × = 移除该条链接记录（直接删 Placement，不进回收站——2026-08-24 用户定；归 0 后端护栏报错）
async function removeLink(p: PlacementLink): Promise<void> {
  if (!file.value) return;
  const assetId = file.value.placement.assetId;
  await syncAssetLinks(assetId, [], [{ folderId: p.folderId ?? "root", origin: p.origin, name: p.name }], []);
}
function jumpToLink(p: PlacementLink): void {
  if (!file.value) return;
  jumpToAssetPlacement(file.value.placement.assetId, p.folderId ?? "root");
}

// ================= 关系 / 音频共用（relations 视图） =================
// 当前目标端点（关系匹配键：kind + id）
const targetRef = computed<{ kind: "asset" | "archive"; id: string }>(() =>
  props.target.kind === "file"
    ? { kind: "asset", id: props.target.placement.assetId }
    : { kind: "archive", id: props.target.node.id },
);
const allViews = computed(() =>
  relationsForTarget(content.relations, currentLibrary.name ?? "", targetRef.value),
);
// 关系区块 = kind free（voice 归音频区块——6.2 四轮走查防泄漏口径）
const relationViews = computed(() => allViews.value.filter((v) => v.relation.kind === "free"));
// 音频区块 = kind voice
const audioViews = computed(() => allViews.value.filter((v) => v.relation.kind === "voice"));

// 发起文件（选择模式 origin）：文件 = 当前 Placement；档案 = 当前档案节点（封面快照供备选区左标）
function originOf(): PickerTarget {
  if (props.target.kind === "file") {
    const p = props.target.placement;
    return { kind: "asset", id: p.assetId, name: p.name, ext: p.ext, thumb: p.thumbPath, folderId: p.folderId };
  }
  const n = props.target.node;
  return {
    kind: "archive",
    id: n.id,
    name: n.name,
    thumb: content.archives[n.id]?.coverPath ?? content.folderThumbMap.get(n.id) ?? null,
  };
}

// 添加钮分派：链接 = 链接管理弹窗 / 音频 = 选择模式 voice（过滤非音频）/ 关系 = 选择模式 relation /
// 缩略图 = 选择模式 thumb（picker 第五用途：过滤可作伴生图，点图直绑，2026-09-23 点亮）；
// 污染文件 = 占位不接业务（2026-08-31 用户定）
function onAdd(id: DetailLinkBlockId): void {
  if (id === "links") openLinkManage();
  else if (id === "audio") picker.enterVoice(originOf());
  else if (id === "relations") picker.enterRelation(originOf());
  else if (id === "thumb") picker.enterThumb(originOf());
}

// —— 关系称谓二次修改（稿注：点击关系变成输入框；LinkRelationItem editing 态自包含 draft） ——
// 写字段按当前目标端位：目标是 from → 改 label；是 to → 改 reverseLabel
const editingId = ref("");
async function commitRelation(v: RelationView, value: string): Promise<void> {
  if (editingId.value !== v.relation.id) return;
  editingId.value = "";
  const next = value.trim();
  if (!next || next === v.label) return;
  try {
    if (v.role === "from") await updateRelation(v.relation.id, next, v.relation.reverseLabel ?? "");
    else await updateRelation(v.relation.id, v.relation.label, next);
    // 就地更新 + 重拉对齐（桩共享数组下元素级写入不触发重算，整表替换最稳——旧件口径）
    const cur = content.relations.find((r) => r.id === v.relation.id);
    if (cur) {
      if (v.role === "from") cur.label = next;
      else cur.reverseLabel = next;
    }
    await content.refreshRelations();
    toast.notify({ message: "已更新关系" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}
async function removeRelationRow(v: RelationView, kindLabel: string): Promise<void> {
  try {
    await removeRelation(v.relation.id);
    const i = content.relations.findIndex((r) => r.id === v.relation.id);
    if (i >= 0) content.relations.splice(i, 1);
    toast.notify({ message: `已删除${kindLabel}` });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// —— 音频播放（稿：点击播放 / 切换 item 播放暂停；单例元素同时只播一条，mp3/wav 直渲、wma 转码兜底） ——
const mediaRef = ref<HTMLAudioElement>();
const playingId = ref(""); // 正在出声的 relation id（"" = 无）
const loadingId = ref(""); // 源解析在途（连点防抖）
const playing = ref(false);

async function togglePlay(v: RelationView): Promise<void> {
  const el = mediaRef.value;
  if (!el) return;
  if (playingId.value === v.relation.id && el.src) {
    if (el.paused) void el.play().catch(() => {});
    else el.pause();
    return;
  }
  if (loadingId.value === v.relation.id) return;
  loadingId.value = v.relation.id;
  try {
    const r = await getMediaSource(v.other.id);
    el.src = convertFileSrc(r.path);
    playingId.value = v.relation.id;
    // 直读优先的播放错误回退（2026-09-13）：直解不了的音频（wma 等）error 后转码兜底重试一次
    el.onerror = () => void onSourceError(v);
    void el.play().catch(() => {});
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    loadingId.value = "";
  }
}

// 播放错误回退（直读优先口径）：转码兜底重试一次；仍失败 toast 原样文案
let transcodeRetriedId = "";
async function onSourceError(v: RelationView): Promise<void> {
  const el = mediaRef.value;
  if (!el || !el.currentSrc || transcodeRetriedId === v.relation.id) return;
  transcodeRetriedId = v.relation.id;
  try {
    const r = await getMediaSource(v.other.id, true);
    el.src = convertFileSrc(r.path);
    void el.play().catch(() => {});
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

onBeforeUnmount(() => {
  const el = mediaRef.value;
  if (el) {
    el.pause();
    el.removeAttribute("src");
    el.load();
  }
});

// ================= 缩略图区块（8.1 伴生缩略图，2026-09-23 接业务） =================
// 调用侧 = 本工程绑定的 hidden 伴生资产一张卡（users = 它被引用的总数，稿「名称 + 2」）；
// 被调用侧 = 当前目标是 hidden 缩略图资产时（「缩略图」快速访问视图点开详情），列引用它的使用方行
const thumbCall = computed(() => (file.value ? (file.value.detail.thumbAsset ?? null) : null));
const thumbUsers = computed(() => (file.value ? (file.value.detail.thumbUsedBy ?? []) : []));

// 点 item 跳转定位（2026-08-24「光显示在那是没用的」同口径）：调用侧伴生资产 hidden，
// 普通视图过滤不可见 → 跳「缩略图」快速访问视图选中；被调用侧使用方 = 普通资产走既有跳转
function jumpToThumb(assetId: string): void {
  const p = content.placements.find((pl) => pl.assetId === assetId);
  if (!p) {
    toast.notify({ message: "目标文件不在本库任何文件夹中（可能在回收站）", icon: "fail" });
    return;
  }
  const key = `${p.assetId}|${p.folderId}`;
  useTabsStore().openView("quick:thumbnail");
  content.requestReveal("quick:thumbnail", key);
  void nextTick(() => selection.selectOnly(key));
}
function jumpToThumbUser(assetId: string): void {
  const p =
    content.placements.find((pl) => pl.assetId === assetId && pl.origin) ??
    content.placements.find((pl) => pl.assetId === assetId);
  if (!p) {
    toast.notify({ message: "目标文件不在本库任何文件夹中（可能在回收站）", icon: "fail" });
    return;
  }
  jumpToAssetPlacement(assetId, p.folderId);
}

// × = 解除绑定（仅清工程侧 thumb_asset_id，hidden 侧不动——还原为素材走卡片右键「解除缩略图」）
function unbindCurrentThumb(): void {
  if (!file.value) return;
  void unbindThumbAsset(file.value.placement.assetId);
}

// 区块是否有内容（空态不渲染内容容器，展开/折叠零高度变化）
function hasContent(id: DetailLinkBlockId): boolean {
  if (id === "links") return linkItems.value.length > 0;
  if (id === "audio") return audioViews.value.length > 0;
  if (id === "relations") return relationViews.value.length > 0;
  if (id === "thumb") return thumbCall.value !== null || thumbUsers.value.length > 0;
  return false; // 污染文件 = 占位（业务未接）
}
</script>

<template>
  <div
    ref="tabRef"
    class="links-tab"
    :class="{ settling: secSettling }"
    @pointerdown="onBlockPointerDown"
  >
    <section v-for="(id, i) in order" :key="id" class="lb-block" :style="blockStyle(i)">
      <SectionTitle
        variant="block"
        :title="titleOf(id)"
        :expanded="isOpen(id)"
        :with-more="false"
        :actions="hasActions(id)"
        static-actions
        @toggle="toggleOpen(id)"
        @add="onAdd(id)"
      />
      <template v-if="isOpen(id) && hasContent(id)">
        <!-- 链接区块：链接卡片列表（稿 gap 8） -->
        <div v-if="id === 'links'" class="lb-body">
          <LinkFileItem
            v-for="(p, pi) in linkItems"
            :key="`${p.folderId ?? 'root'}|${p.name}|${pi}`"
            :block-name="p.blockName"
            :folder-name="dirName(p)"
            :file-name="p.name"
            :origin="p.origin && linkItems.length > 1"
            :linked="!p.origin && linkItems.length > 1"
            @click="jumpToLink(p)"
            @close="removeLink(p)"
          />
        </div>
        <!-- 音频区块：音频卡（使用方视角）/ 使用者行（音频视角） -->
        <div v-else-if="id === 'audio'" class="lb-body">
          <template v-for="v in audioViews" :key="v.relation.id">
            <LinkAudioItem
              v-if="v.role === 'from'"
              :name="v.other.name"
              :playing="playingId === v.relation.id && playing"
              @click="jumpToRelationTarget(v.other)"
              @toggle-play="togglePlay(v)"
              @close="removeRelationRow(v, '链接音频')"
            />
            <!-- 使用者行（当前目标是音频：显示"在哪些文件里使用过"；ni-card 壳 + 滑入关闭钮，
                 无通用文件图标不猜造——档案出 archive 双色调，文件无图标沿旧件口径） -->
            <div v-else class="user-row ni-card ni-hover" @click="jumpToRelationTarget(v.other)">
              <TwoToneIcon v-if="v.other.kind === 'archive'" name="archive" :size="18" />
              <span class="nm">{{ v.other.name }}</span>
              <span class="ni-slide">
                <span class="ni-slide-inner">
                  <button class="ni-close" @click.stop="removeRelationRow(v, '链接音频')">
                    <AppIcon name="close-small" :size="14" />
                  </button>
                </span>
              </span>
            </div>
          </template>
        </div>
        <!-- 关系区块：关系卡（chip = 称谓，点击进输入态） -->
        <div v-else-if="id === 'relations'" class="lb-body">
          <LinkRelationItem
            v-for="v in relationViews"
            :key="v.relation.id"
            :relation="v.label"
            :target="v.other.name"
            :editing="editingId === v.relation.id"
            @click="jumpToRelationTarget(v.other)"
            @edit-relation="editingId = v.relation.id"
            @commit="(val) => commitRelation(v, val)"
            @cancel="editingId = ''"
            @close="removeRelationRow(v, '关系')"
          />
        </div>
        <!-- 缩略图区块（8.1，2026-09-23 接业务）：调用侧伴生资产卡（名称 + 引用数）/ 被调用侧使用方行 -->
        <div v-else-if="id === 'thumb'" class="lb-body">
          <LinkThumbItem
            v-if="thumbCall"
            :name="thumbCall.name"
            :count="thumbCall.users ?? undefined"
            @click="jumpToThumb(thumbCall.assetId)"
            @close="unbindCurrentThumb"
          />
          <LinkThumbItem
            v-for="u in thumbUsers"
            :key="u.assetId"
            :name="u.name"
            @click="jumpToThumbUser(u.assetId)"
            @close="void unbindThumbAsset(u.assetId)"
          />
        </div>
      </template>
    </section>
    <!-- 音频单例元素（同时只播一条；组件卸载即停） -->
    <audio
      ref="mediaRef"
      class="hidden-media"
      preload="auto"
      @play="playing = true"
      @pause="playing = false"
      @ended="playing = false"
    />
  </div>
</template>

<style scoped>
/* 链接关系 tab：区块壳（稿：padding 12 / gap 12 / border-top 通用/三级）；
   transform 过渡 = 标题行拖拽让位动画（拖动块 inline 禁过渡跟随指针，归位帧 .settling 禁过渡） */
.links-tab {
  display: flex;
  flex-direction: column;
}
.lb-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
.links-tab.settling .lb-block {
  transition: none;
}
/* 区块内容列表（稿：卡片列表 gap 8） */
.lb-body {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* item 宽随父容器（稿值 240 = 栏宽 264 - 左右 padding 12×2 画布语境，卡片族尺寸模型口径） */
.lb-body :deep(.ni-card) {
  width: 100%;
}
/* 使用者行（音频视角）：ni-card 壳 + 图标/名称 + 滑入关闭钮（42 高同族档） */
.user-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 42px;
  padding: var(--n-space-12);
  cursor: pointer;
}
.user-row .nm {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.hidden-media {
  display: none;
}
</style>
