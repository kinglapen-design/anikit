<script setup lang="ts">
// 序列帧包内视图（5.2，PRD 04-序列帧"包内视图定案"）：素材卡片右键「打开序列帧包」进入
// （视口 key = seq:{assetId}，ContentArea 分流渲染）。帧卡片列表 = 素材卡片同款视觉
// （缩略图区 C2 / 简介区 C3 紧凑 36 / ::after 常态 1px 暗-悬停 + ::before 状态层 opacity 过渡），
// 恒按帧序平铺（不参与排序/分组/新增区），卡片尺寸随查看方式滑块。
// 缩略图 = cache/frames 帧条小图（与预览帧条同一份缓存；未缓存帧原图兜底 + preview-strips-ready
// 事件渐进填）。操作：双击 / 右键「打开预览」= 退回包所在文件夹视口打开预览定位该帧；
// 右键「重排序号 / 删除此张序列图」（多选 = 删除 N 张，Delete 键同径）；空白右键 = 重排序号。
// 选择模型为包内本地集（帧不是 Placement，不进全局选择域）；打开本视口时全局选择域已清空
//（assetCard 菜单 action 内 selection.clear()——防全局 Delete 误伤素材域）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import ScrollThumb from "./ScrollThumb.vue";
import { listSequenceFrames, type FrameEntry } from "../lib/mediaOps";
import { openContextMenu, type CtxEntry } from "../lib/contextMenus";
import { deleteSequenceFrames, renumberSequenceFrames, setSequenceFrameThumb } from "../lib/placementOps";
import { useContentStore } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { usePreviewStore } from "../stores/preview";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ assetId: string; panelActive: boolean }>();

const content = useContentStore();
const ui = useUiStore();

const frames = ref<FrameEntry[]>([]);
const thumbFrame = ref<string | null>(null);
const error = ref<string | null>(null); // 加载失败（包不存在 / 已解散）= 空态文案
const selected = ref<Set<string>>(new Set());
let anchorIdx = -1; // shift 范围选锚点（frames 数组下标）

// 卡片宽度随滑块（targetRowHeight = 素材网格行高）：按素材卡 260:220 比例折算
const cardWidth = computed(() => Math.round(ui.targetRowHeight * (260 / 220)));

async function load(): Promise<void> {
  try {
    const r = await listSequenceFrames(props.assetId);
    frames.value = r.frames;
    thumbFrame.value = r.thumbFrame;
    error.value = null;
    // 清单重拉后剔除已消失的选中帧（删帧 / undo 等外部变化）
    const alive = new Set(r.frames.map((f) => f.name));
    const next = new Set([...selected.value].filter((n) => alive.has(n)));
    if (next.size !== selected.value.size) selected.value = next;
  } catch (e) {
    frames.value = [];
    error.value = String(e);
  }
}

// 热刷新：目标资产 modifiedAt 变化（删帧 / 重排 / undo-redo 后的 content.refresh 带入）→ 重拉清单；
// 资产消失（已解散）→ modifiedAt 变 null 同样触发，listSequenceFrames 报错落空态
const assetModifiedAt = computed(
  () => content.placements.find((p) => p.assetId === props.assetId)?.modifiedAt ?? null,
);
watch(assetModifiedAt, () => void load());
watch(() => props.assetId, () => void load());

// 帧条小图渐进填充（与预览帧条同一事件）：未缓存帧后台生成批次推送
let unlistenStrips: (() => void) | undefined;
onMounted(async () => {
  void load();
  unlistenStrips = await listen<{ assetId: string; items: { name: string; stripPath: string }[] }>(
    "preview-strips-ready",
    (e) => {
      if (e.payload.assetId !== props.assetId) return;
      for (const item of e.payload.items) {
        const f = frames.value.find((x) => x.name === item.name);
        if (f) f.stripPath = item.stripPath;
      }
    },
  );
  window.addEventListener("keydown", onKeydown);
});
onBeforeUnmount(() => {
  try {
    unlistenStrips?.();
  } catch {
    /* 退订失败不阻断卸载（探针桩环境无回调表会抛 TypeError） */
  }
  window.removeEventListener("keydown", onKeydown);
});

function thumbUrl(f: FrameEntry): string {
  return convertFileSrc(f.stripPath ?? f.fullPath);
}

// —— 选择（click 单选 / ctrl 切换 / shift 范围；点空白清空）——
function clickFrame(e: MouseEvent, name: string, idx: number): void {
  if (usePickerStore().active) return; // 选择模式（6.2）：帧不是可选目标，帧选择禁用
  if (e.shiftKey && anchorIdx >= 0) {
    const [a, b] = [Math.min(anchorIdx, idx), Math.max(anchorIdx, idx)];
    const range = frames.value.slice(a, b + 1).map((f) => f.name);
    selected.value = e.ctrlKey || e.metaKey ? new Set([...selected.value, ...range]) : new Set(range);
    return;
  }
  if (e.ctrlKey || e.metaKey) {
    const next = new Set(selected.value);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    selected.value = next;
  } else {
    selected.value = new Set([name]);
  }
  anchorIdx = idx;
}

// 打开预览定位该帧：退回包所在文件夹视口（preview.current 依赖视口 Placement 解析）+ 打开 + 定位
async function openFramePreview(name: string): Promise<void> {
  const p =
    content.placements.find((x) => x.assetId === props.assetId && x.origin) ??
    content.placements.find((x) => x.assetId === props.assetId);
  if (!p) return;
  const idx = frames.value.findIndex((f) => f.name === name);
  useTabsStore().openView(`folder:${p.folderId}`);
  const preview = usePreviewStore();
  await preview.open(`${p.assetId}|${p.folderId}`);
  if (idx >= 0) preview.setFrame(idx);
}

// —— 右键菜单 ——
function frameMenu(e: MouseEvent, name: string, idx: number): void {
  // 选择模式（6.2，2026-08-24 白名单收窄）：帧不是可选目标，菜单仅留查看类「打开预览」
  if (usePickerStore().active) {
    openContextMenu(e, [{ label: "打开预览", action: () => void openFramePreview(name) }], undefined);
    return;
  }
  // 右键未选中项先单选（全库惯例）；已在多选域内保持选择
  if (!selected.value.has(name)) clickFrame(e, name, idx);
  const targets = [...selected.value];
  const entries: CtxEntry[] = [
    { label: "打开预览", action: () => void openFramePreview(name) },
    // 将当前帧设为预览图（2026-08-21 用户定）：直复用播放条同款 set_thumb_frame 业务；
    // 单帧语义——多选时恒对被右键帧生效
    {
      label: "将当前帧设为预览图",
      action: () => void setSequenceFrameThumb(props.assetId, name),
    },
    { label: "重排序号", action: () => void renumberSequenceFrames(props.assetId) },
    "divider",
    {
      label: targets.length > 1 ? `删除 ${targets.length} 张序列图` : "删除此张序列图",
      shortcut: "Delete",
      action: () => void deleteSequenceFrames(props.assetId, targets),
    },
  ];
  openContextMenu(e, entries, undefined);
}

function blankMenu(e: MouseEvent): void {
  if (usePickerStore().active) return; // 选择模式（6.2）：重排序号是编辑类，不出
  openContextMenu(e, [
    { label: "重排序号", action: () => void renumberSequenceFrames(props.assetId) },
  ]);
}

// Delete = 删除选中帧（全局 Delete 对本视口无操作——打开时全局选择域已清空）；
// 输入态（重命名等）不触发；仅本 panel 激活时响应
function onKeydown(e: KeyboardEvent): void {
  if (usePickerStore().active) return; // 选择模式（6.2）：删帧是编辑类，禁用
  if (!props.panelActive || e.key !== "Delete" || selected.value.size === 0) return;
  const t = e.target as HTMLElement;
  if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement) return;
  e.preventDefault();
  void deleteSequenceFrames(props.assetId, [...selected.value]);
}

const scrollRef = ref<HTMLElement>();
</script>

<template>
  <div class="seq-pack">
    <!-- 空态（包不存在 / 已解散 / 加载失败）：居中标签档文案 -->
    <div v-if="error" class="empty">{{ error }}</div>
    <div
      v-else
      ref="scrollRef"
      class="kit-scroll pack-scroll"
      @click.self="selected = new Set()"
      @contextmenu.self.prevent="blankMenu"
    >
      <div class="pack-grid" :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${cardWidth}px, 1fr))` }">
        <div
          v-for="(f, i) in frames"
          :key="f.name"
          class="frame-card"
          :class="{ selected: selected.has(f.name) }"
          @click="clickFrame($event, f.name, i)"
          @dblclick="openFramePreview(f.name)"
          @contextmenu.prevent.stop="frameMenu($event, f.name, i)"
        >
          <div class="thumb">
            <img class="img" :src="thumbUrl(f)" :alt="f.name" decoding="async" draggable="false" />
          </div>
          <div class="caption">
            <span class="name" v-tip="{ text: f.name, column: true }">{{ f.name }}</span>
            <span v-if="f.name === thumbFrame" class="cover-tag">封面</span>
          </div>
        </div>
      </div>
    </div>
    <ScrollThumb v-if="!error" :body="scrollRef" />
  </div>
</template>

<style scoped>
/* 包内视图容器：占满素材区（ContentArea .asset-section 已承担 padding 16 16 0），relative 供 ScrollThumb 锚定 */
.seq-pack {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.pack-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  /* 右缘留白：卡片右缘距中栏外边框 16（素材网格同口径） */
  padding-right: var(--space-16);
}
.pack-grid {
  display: grid;
  gap: var(--space-12);
  padding-bottom: var(--space-16);
}
/* 空态（3.6 搜索空态同款：居中标签档） */
.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
}
/* —— 帧卡片（素材卡片 14 稿紧凑形态同款视觉）：
     ::after = 常态 1px 暗-悬停（无过渡）；::before = 状态层 1.5px opacity 过渡
     （hover 亮-选中实色 / 选中品牌色——踩坑 036 口径，禁 border-color 过渡）—— */
.frame-card {
  position: relative;
  display: flex;
  flex-direction: column;
  aspect-ratio: 260 / 199; /* 素材卡 260:220 紧凑简介（57→36）折算 */
  border-radius: var(--radius-6);
  cursor: default;
}
.frame-card::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1px solid var(--line-hover);
  border-radius: var(--radius-6);
  pointer-events: none;
}
.frame-card::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 1;
  border: var(--border-strong) solid transparent;
  border-radius: var(--radius-6);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.12s ease;
}
.frame-card:hover::before {
  opacity: 1;
  border-color: var(--line-card-hover);
}
.frame-card.selected::before {
  opacity: 1;
  border-color: var(--brand-border);
}
/* 缩略图区：C2 底 / 圆角上沿；contain 完整显示不裁切（网格模式口径） */
.thumb {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c2);
  border-radius: var(--radius-6) var(--radius-6) 0 0;
  overflow: hidden;
}
.img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
/* 简介区紧凑形态（36 高）：C3 / inset 顶线 / 名称单行左对齐 + 封面标记 */
.caption {
  flex: none;
  height: 36px;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: 0 var(--space-8) 0 var(--space-12);
  background: var(--c3);
  box-shadow: inset 0 1px 0 0 var(--line-hover);
  border-radius: 0 0 var(--radius-6) var(--radius-6);
}
.name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 封面帧标记：标签档小字（复用简介档样式，不自创） */
.cover-tag {
  flex: none;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
}
</style>
