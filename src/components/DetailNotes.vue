<script setup lang="ts">
// 详情模块 · 逐帧笔记（5.2 展示层）：有笔记才显示（无选中资产 / 无笔记整个区块不渲染）。
// 数据源：选中资产变化 / previewNotes.version（写删笔记）变化时调 get_notes 拉取（热刷新）。
// item = 02_item 06 关联文件夹同款（C4 底 / hover C5 / 圆角 8 / 内边距 4 4 4 8 / 间距 6 /
// inset 1px border-1）：左侧时间（视频 帧号/mm:ss 随"时间显示"设置，5.3）或帧号（副文本档）+ 文本单行省略，
// × 常态不占位隐藏、悬停 item 展开 24px 露出（删除 = set_note 空文本 + noteCount 同步角标）。
// 悬停浮帧画面（Eagle 式）：item 正上方贴近浮出该帧截图（get_note_thumb；C3 / inset border-1 /
// 圆角 8 / --shadow-pop / 内边距 4，宽 160 按比例），未就绪 C4 占位；移出 / 切选中即关闭。
// 点击 item：打开预览并定位（序列帧/动图跳帧并暂停，视频 seek 该时间点）+ 打开笔记编辑浮层。
import { computed, ref, watch } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";
import { formatDuration } from "../lib/formatDuration";
import { frameOfTime, totalFrames } from "../lib/formatFrame";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";
import { usePreviewNotesStore, type NoteEntry } from "../stores/previewNotes";

const content = useContentStore();
const ui = useUiStore();
const notesStore = usePreviewNotesStore();
const open = ref(true);
const notes = ref<NoteEntry[]>([]);

// —— 悬停浮帧画面 ——（声明须在下方 watch 之前：watch immediate 首轮即调 closeThumb 读 thumbKey，TDZ 会炸）
const thumbKey = ref(""); // 当前浮层服务的笔记 key（空 = 关闭）
const thumbUrl = ref<string | null>(null); // null = 未就绪 / 取不到（C4 占位）

// 当前选中的 PlacementView（无选中 = null）
const selected = computed(
  () => content.placements.find((p) => `${p.assetId}|${p.folderId}` === content.selectedAssetKey) ?? null,
);

// 选中变化 / 笔记数据变更（预览浮层写删、本组件删除）→ 拉取笔记 + 关浮层
watch(
  [() => content.selectedAssetKey, () => notesStore.version],
  async ([key]) => {
    closeThumb();
    const sel = selected.value;
    notes.value = [];
    if (!sel) return;
    try {
      const list = await invoke<NoteEntry[]>("get_notes", { assetId: sel.assetId });
      if (content.selectedAssetKey === key) notes.value = list; // await 期间已切走防错配
    } catch (e) {
      console.warn("笔记拉取失败（静默降级）：", e);
    }
  },
  { immediate: true },
);

// item 左侧标识：视频 t:{秒} → 帧号（帧数模式且素材 fps 可用，5.3 帧化；底层 key 仍 `t:{秒}` 不变）
// 或 mm:ss（时间码模式 / fps 缺失 / 音频）；序列帧/动图帧名（标题-00001.webp）→ 帧号数字
function label(key: string): string {
  if (key.startsWith("t:")) {
    const secs = Number(key.slice(2)) || 0;
    const fps = selected.value?.fps;
    if (ui.timeDisplay === "frames" && fps != null && fps > 0) {
      const total = totalFrames(selected.value?.duration ?? 0, fps);
      return String(frameOfTime(secs, fps, total));
    }
    return formatDuration(secs);
  }
  const m = key.match(/-(\d+)\.[^.]+$/);
  return m ? String(parseInt(m[1], 10)) : key;
}

// × 删除：set_note 空文本（后端约定）→ 返回全量直接替换；noteCount 同步卡片角标 + 版本号热刷新
async function remove(key: string): Promise<void> {
  const sel = selected.value;
  if (!sel) return;
  notes.value = await invoke<NoteEntry[]>("set_note", { assetId: sel.assetId, key, text: "" });
  content.setNoteCount(sel.assetId, notes.value.length);
  notesStore.version += 1;
  if (thumbKey.value === key) closeThumb();
}

// —— 悬停浮帧画面 ——（thumbKey / thumbUrl 声明已上移至 watch 之前）
async function showThumb(key: string): Promise<void> {
  const sel = selected.value;
  if (!sel) return;
  thumbKey.value = key;
  thumbUrl.value = null;
  try {
    const r = await invoke<{ path: string | null }>("get_note_thumb", { assetId: sel.assetId, key });
    if (thumbKey.value === key && r.path) thumbUrl.value = convertFileSrc(r.path);
  } catch (e) {
    console.warn("笔记帧画面获取失败（静默降级）：", e);
  }
}
function closeThumb(): void {
  thumbKey.value = "";
  thumbUrl.value = null;
}

// 点击 item：打开预览并定位（open 内部 await 完帧清单 / 播放源再返回，此处直接定位）+
// 打开笔记编辑浮层（2026-07-26 用户：跳转到对应时间戳并打开编辑框）
async function jump(key: string): Promise<void> {
  const sel = selected.value;
  if (!sel) return;
  const preview = usePreviewStore();
  await preview.open(`${sel.assetId}|${sel.folderId}`);
  if (preview.kind === "sequence" || preview.kind === "anim") {
    const i = preview.seq?.frames.findIndex((f) => f.name === key) ?? -1;
    if (i >= 0) {
      preview.pause();
      preview.setFrame(i);
    }
  } else if (preview.kind === "video" && key.startsWith("t:")) {
    usePreviewMediaStore().pause();
    usePreviewMediaStore().seek(Number(key.slice(2)));
  }
  // 打开笔记编辑浮层（toggle 内含暂停与懒加载；已打开则不动）
  if (!notesStore.open) notesStore.toggle();
}
</script>

<template>
  <section v-if="notes.length > 0" class="detail-notes detail-block">
    <SectionHeader title="笔记" grip v-model:open="open" />
    <div v-if="open" class="items">
      <div
        v-for="n in notes"
        :key="n.key"
        class="item"
        @mouseenter="showThumb(n.key)"
        @mouseleave="closeThumb"
        @click="jump(n.key)"
      >
        <span class="pos">{{ label(n.key) }}</span>
        <span class="text">{{ n.text }}</span>
        <IconButton name="clear" small class="del" title="删除笔记" @click.stop="remove(n.key)" />
        <!-- 悬停浮帧画面：item 正上方贴近浮出，不吃鼠标事件（悬停语义全在 item 上） -->
        <div v-if="thumbKey === n.key" class="note-thumb">
          <img v-if="thumbUrl" class="frame" :src="thumbUrl" alt="" draggable="false" />
          <div v-else class="frame placeholder" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 区块容器：flex column 必须有（detail-block 公共规则 gap 8 = 副标题与内容间距才生效） */
.detail-notes {
  display: flex;
  flex-direction: column;
}
/* 子项容器：间距 8（与关联文件夹 / 标签区块统一，2026-07-26 用户） */
.items {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 笔记 item（02_item 06 关联文件夹同款）：C4 底 / hover C5 / inset 1px border-1 / 圆角 8 /
   内边距 4 4 4 8 / 间距 6；relative 供浮帧画面锚定 */
.item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-4) var(--space-4) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  cursor: default;
}
.item:hover {
  background: var(--c5);
}
/* 左侧时间 / 帧号：副文本档，等宽对齐位 */
.pos {
  flex: none;
  min-width: 34px;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
}
/* 文本：font-icon-2 / 单行省略 */
.text {
  flex: 1;
  min-width: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* × 删除：常态不占位隐藏（宽度 0 留给文本），悬停 item 展开 24px 露出（02_item 06 同款）；
   选择器带 .item 提权，覆盖 IconButton.small 的固定 width 24（同 DetailLinks 既有处理） */
.item .del {
  width: 0;
  padding-left: 0;
  padding-right: 0;
  opacity: 0;
  overflow: hidden;
  color: var(--font-icon-3);
  transition:
    width 0.12s ease,
    opacity 0.12s ease,
    color 0.12s ease;
}
.item:hover .del {
  width: 24px;
  opacity: 1;
}
.del:hover {
  color: var(--font-icon-2);
}
/* 悬停浮帧画面：item 正上方贴近浮出 / 宽 160 按比例 / 容器 C3 / inset border-1 /
   圆角 8 / --shadow-pop / 内边距 4；不吃鼠标事件防截断 hover 链 */
.note-thumb {
  position: absolute;
  bottom: calc(100% + var(--space-2));
  left: 0;
  width: 160px;
  padding: var(--space-4);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    inset 0 0 0 1px var(--border-1),
    var(--shadow-pop);
  pointer-events: none;
  z-index: 10;
}
.frame {
  display: block;
  width: 100%;
  height: auto;
  border-radius: var(--radius-4);
}
/* 未就绪占位：C4（高度给常规横屏比例，图到即换） */
.placeholder {
  height: 88px;
  background: var(--c4);
}
</style>
