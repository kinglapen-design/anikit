<script setup lang="ts">
// 详情模块 · 逐帧笔记（5.2 展示层）：有笔记才显示（无选中资产 / 无笔记整个区块不渲染）。
// 选中资产变化时调 get_notes 拉取（UI 只调命令）；item = 02_item 基础 item（内边距 4 6 /
// 圆角 6 / hover font-icon-6）：左侧时间（视频 mm:ss）或帧号（帧名 标题-00001.webp 提取序号）
// + 文本单行省略，悬停出 × 删除（set_note 空文本，返回全量直接替换 + noteCount 同步卡片角标）。
// 悬停浮帧画面（Eagle 式）：区块上方浮出该帧截图（get_note_thumb；容器 C3 / inset border-1 /
// 圆角 8 / --shadow-pop / 内边距 4，宽 ~160 按比例），未就绪 C4 占位；移出 / 切选中即关闭。
// 点击 item：打开预览并定位（序列帧/动图跳帧，视频 seek 到该时间点）。
import { computed, ref, watch } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";
import { formatDuration } from "../lib/formatDuration";
import { useContentStore } from "../stores/content";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";
import type { NoteEntry } from "../stores/previewNotes";

const content = useContentStore();
const open = ref(true);
const notes = ref<NoteEntry[]>([]);

// 当前选中的 PlacementView（无选中 = null）
const selected = computed(
  () => content.placements.find((p) => `${p.assetId}|${p.folderId}` === content.selectedAssetKey) ?? null,
);

// 选中变化 → 拉取笔记 + 关浮层；无选中清空（有笔记才显示由模板 v-if 保证）
watch(
  () => content.selectedAssetKey,
  async (key) => {
    closeThumb();
    notes.value = [];
    const sel = selected.value;
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

// item 左侧标识：视频 t:{秒} → mm:ss；序列帧/动图帧名（标题-00001.webp）→ 帧号数字
function label(key: string): string {
  if (key.startsWith("t:")) return formatDuration(Number(key.slice(2)) || 0);
  const m = key.match(/-(\d+)\.[^.]+$/);
  return m ? String(parseInt(m[1], 10)) : key;
}

// × 删除：set_note 空文本（后端约定）→ 返回全量直接替换；noteCount 同步卡片角标
async function remove(key: string): Promise<void> {
  const sel = selected.value;
  if (!sel) return;
  notes.value = await invoke<NoteEntry[]>("set_note", { assetId: sel.assetId, key, text: "" });
  content.setNoteCount(sel.assetId, notes.value.length);
  if (thumbKey.value === key) closeThumb();
}

// —— 悬停浮帧画面 ——
const thumbKey = ref(""); // 当前浮层服务的笔记 key（空 = 关闭）
const thumbUrl = ref<string | null>(null); // null = 未就绪 / 取不到（C4 占位）

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

// 点击 item：打开预览并定位——open 内部 await 完帧清单 / 播放源再返回，此处直接定位；
// 序列帧/动图跳帧并暂停（看定格帧），视频 seek 到该时间点（动图未解包完 seq 为空，定位跳过）
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
    usePreviewMediaStore().seek(Number(key.slice(2)));
  }
}
</script>

<template>
  <section v-if="notes.length > 0" class="detail-notes detail-block">
    <SectionHeader title="笔记" :count="notes.length" grip v-model:open="open" />
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
      </div>
    </div>
    <!-- 悬停浮帧画面：区块上方浮出，不吃鼠标事件（悬停语义全在 item 上） -->
    <div v-if="thumbKey" class="note-thumb">
      <img v-if="thumbUrl" class="frame" :src="thumbUrl" alt="" draggable="false" />
      <div v-else class="frame placeholder" />
    </div>
  </section>
</template>

<style scoped>
.detail-notes {
  position: relative; /* 浮帧画面锚点 */
  display: flex;
  flex-direction: column;
}
.items {
  display: flex;
  flex-direction: column;
}
/* 笔记 item（02_item 基础 item）：内边距 4 6 / 圆角 6 / 间距 6 / hover font-icon-6 */
.item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-6);
  border-radius: var(--radius-6);
  cursor: default;
}
.item:hover {
  background: var(--font-icon-6);
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
/* × 删除：悬停 item 才出（同 Eagle 列表行内按钮） */
.del {
  opacity: 0;
  color: var(--font-icon-3);
}
.item:hover .del {
  opacity: 1;
}
.del:hover {
  color: var(--font-icon-2);
}
/* 悬停浮帧画面：区块上方浮出 / 宽 ~160 按比例 / 容器 C3 / inset border-1 /
   圆角 8 / --shadow-pop / 内边距 4；不吃鼠标事件防截断 hover 链 */
.note-thumb {
  position: absolute;
  bottom: calc(100% + var(--space-8));
  left: var(--space-16);
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
