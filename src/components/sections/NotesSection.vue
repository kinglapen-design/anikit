<script setup lang="ts">
// 逐帧笔记卡片（自定义组件 tab 卡片，2026-10-12 接回换代——2026-08-31 三 tab 化摘出待补稿的
// DetailNotes 业务本体移植，样式换代：旧 SectionHeader → items/SectionTitle block 型（折叠钮常显、
// 无动作组——添加走预览悬浮栏笔记钮）、旧 02_item 06 稿 item → items/NoteItem（笔记 item 稿件，
// 时间码数字档 + ni-slide 滑入删除））：
// 仅库内文件形态（assetId 级——档案无逐帧笔记；回收站选中沿旧口径不渲染）；标题行恒渲染
//（链接 tab 五区块同款 2026-08-31 口径：没有内容标题也要在），无笔记仅不渲染列表容器。
// 业务不变：watch 目标 assetId / notesStore.version 拉取（get_notes 热刷新）；
// 时间码 = 视频 帧号/mm:ss 随「时间显示」设置（5.3 帧化，fps 可用才显帧号）/ 动图 f:索引 /
// 序列帧帧名 → 帧号；× = set_note 空文本（noteCount 角标同步在 store 收尾）；
// 悬停浮帧画面（Eagle 式）：行正上方浮出该帧截图（get_note_thumb），未就绪 g4 占位，
// 移出 / 切选中即关；点击行 = 打开预览定位（序列帧/动图跳帧暂停，视频 seek）+ 打开笔记编辑浮层。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import SectionTitle from "../items/SectionTitle.vue";
import NoteItem from "../items/NoteItem.vue";
import { formatDuration } from "../../lib/formatDuration";
import { frameOfTime, totalFrames } from "../../lib/formatFrame";
import { placementKey } from "../../stores/content";
import { useUiStore } from "../../stores/ui";
import { previewSession } from "../../stores/preview";
import { previewMediaSession } from "../../stores/previewMedia";
import { previewNotesSession, usePreviewNotesStore, type NoteEntry } from "../../stores/previewNotes";
import { useTabsStore } from "../../stores/tabs";
import type { DetailTarget } from "../../lib/detailSections";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
const ui = useUiStore();
const notesStore = usePreviewNotesStore();
const open = ref(true);
const notes = ref<NoteEntry[]>([]);

// 仅库内文件形态有逐帧笔记（回收站选中沿旧 DetailNotes 口径不渲染——placements 外资产不开预览跳帧）
const file = computed(() =>
  props.target.kind === "file" && props.target.placement.folderId !== "recycle" ? props.target : null,
);

// —— 悬停浮帧画面 ——（声明须在下方 watch 之前：watch immediate 首轮即调 closeThumb 读 thumbKey，TDZ 会炸）
const thumbKey = ref(""); // 当前浮层服务的笔记 key（空 = 关闭）
const thumbUrl = ref<string | null>(null); // null = 未就绪 / 取不到（g4 占位）

// 目标变化 / 笔记数据变更（预览浮层写删、本组件删除）→ 拉取笔记 + 关浮层
//（命令封装在 previewNotes store，失败静默降级）
watch(
  [() => file.value?.placement.assetId, () => notesStore.version],
  async ([assetId]) => {
    closeThumb();
    notes.value = [];
    if (!assetId) return;
    const list = await notesStore.loadNotes(assetId);
    if (list && file.value?.placement.assetId === assetId) notes.value = list; // await 期间已切走防错配
  },
  { immediate: true },
);

// 行左时间码：视频 t:{秒} → 帧号（帧数模式且素材 fps 可用，5.3 帧化；底层 key 仍 `t:{秒}` 不变）
// 或 mm:ss（时间码模式 / fps 缺失 / 音频）；动图 f:{索引}（2026-08-21 补，解包废除漏适配）与
// 序列帧帧名（标题_00001.png；存量 `-` 分隔帧名兼容，2026-09-23 归一名自 - 改 _）→ 帧号数字
function label(key: string): string {
  if (key.startsWith("t:")) {
    const secs = Number(key.slice(2)) || 0;
    const fps = file.value?.placement.fps;
    if (ui.timeDisplay === "frames" && fps != null && fps > 0) {
      const total = totalFrames(file.value?.placement.duration ?? 0, fps);
      return String(frameOfTime(secs, fps, total));
    }
    return formatDuration(secs);
  }
  if (key.startsWith("f:")) {
    const i = Number(key.slice(2));
    return Number.isFinite(i) ? String(i + 1) : key;
  }
  const m = key.match(/[-_](\d+)\.[^.]+$/);
  return m ? String(parseInt(m[1], 10)) : key;
}

// × 删除：set_note 空文本（后端约定）→ 返回全量直接替换；noteCount 角标同步 + 版本号热刷新在 store 收尾
async function remove(key: string): Promise<void> {
  const assetId = file.value?.placement.assetId;
  if (!assetId) return;
  notes.value = await notesStore.saveNote(assetId, key, "");
  if (thumbKey.value === key) closeThumb();
}

// —— 悬停浮帧画面（mouseenter/leave 经 attrs fallthrough 落 NoteItem 根行） ——
async function showThumb(key: string): Promise<void> {
  const assetId = file.value?.placement.assetId;
  if (!assetId) return;
  thumbKey.value = key;
  thumbUrl.value = null;
  try {
    const r = await notesStore.getNoteThumb(assetId, key);
    if (thumbKey.value === key && r.path) thumbUrl.value = convertFileSrc(r.path);
  } catch (e) {
    console.warn("笔记帧画面获取失败（静默降级）：", e);
  }
}
function closeThumb(): void {
  thumbKey.value = "";
  thumbUrl.value = null;
}

// 点击行：打开预览并定位（open 内部 await 完帧清单 / 播放源再返回，此处直接定位）+
// 打开笔记编辑浮层（2026-07-26 用户：跳转到对应时间戳并打开编辑框）；
// 本组件 = 右栏全局件无 panel 注入，路由激活 panel 的会话（2026-09-25 per-panel 会话化）
async function jump(key: string): Promise<void> {
  const sel = file.value?.placement;
  if (!sel) return;
  const pid = useTabsStore().activeTabId;
  if (!pid) return;
  const preview = previewSession(pid);
  await preview.open(placementKey(sel));
  // 动图（2026-08-21 修，解包废除漏适配）：seq 恒 null，按 f:{索引} 直接定位
  if (preview.kind === "anim") {
    if (key.startsWith("f:")) {
      const i = Number(key.slice(2));
      if (Number.isFinite(i) && i >= 0) {
        preview.pause();
        preview.setFrame(i);
      }
    }
  } else if (preview.kind === "sequence") {
    const i = preview.seq?.frames.findIndex((f) => f.name === key) ?? -1;
    if (i >= 0) {
      preview.pause();
      preview.setFrame(i);
    }
  } else if (preview.kind === "video" && key.startsWith("t:")) {
    const media = previewMediaSession(pid);
    media.pause();
    media.seek(Number(key.slice(2)));
  }
  // 打开笔记编辑浮层（toggle 内含暂停与懒加载；已打开则不动）
  const ns = previewNotesSession(pid);
  if (!ns.open) ns.toggle();
}
</script>

<template>
  <section v-if="file" class="notes-card">
    <SectionTitle
      variant="block"
      title="逐帧笔记"
      :expanded="open"
      :actions="false"
      static-actions
      @toggle="open = !open"
    />
    <!-- 无笔记仅不渲染列表容器（空数据不渲染容器口径）；悬停浮帧锚点 = 行外包 relative 槽 -->
    <div v-if="open && notes.length > 0" class="notes-body">
      <div
        v-for="n in notes"
        :key="n.key"
        class="note-slot"
        @mouseenter="showThumb(n.key)"
        @mouseleave="closeThumb"
      >
        <NoteItem :time="label(n.key)" :text="n.text" @click="jump(n.key)" @close="remove(n.key)" />
        <!-- 悬停浮帧画面：行正上方贴近浮出，不吃鼠标事件（悬停语义全在行上） -->
        <div v-if="thumbKey === n.key" class="note-thumb">
          <img v-if="thumbUrl" class="frame" :src="thumbUrl" alt="" draggable="false" />
          <div v-else class="frame placeholder" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 卡片壳（详情区块同款：padding 12 / gap 12 / border-top 通用/三级） */
.notes-card {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
}
/* 笔记列表（详情卡片列表同口径 gap 8） */
.notes-body {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 行槽 = 悬停浮帧锚点（NoteItem 根无定位） */
.note-slot {
  position: relative;
}
/* 悬停浮帧画面：行正上方贴近浮出 / 宽 160 按比例 / 容器 通用三级底 + inset 亮半透明-2 /
   圆角 8 / 立体感弹层 / 内边距 4；不吃鼠标事件防截断 hover 链 */
.note-thumb {
  position: absolute;
  bottom: calc(100% + var(--n-space-2));
  left: 0;
  width: 160px;
  padding: var(--n-space-4);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow:
    inset 0 0 0 1px var(--n-l-a2),
    var(--n-shadow-lg);
  pointer-events: none;
  z-index: 10;
}
.frame {
  display: block;
  width: 100%;
  height: auto;
  border-radius: var(--n-radius-4);
}
/* 未就绪占位：通用四级底（高度给常规横屏比例，图到即换） */
.placeholder {
  height: 88px;
  background: var(--n-g4);
}
</style>
