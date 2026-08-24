<script setup lang="ts">
// 区块 · 音频（6.2 三轮链接音频点亮，figma 03_数据 item §5 音频 item；PRD 03_数据链接 02 音色）：
// 数据 = relation kind=voice（from = 使用方文件/档案，to = 音频 asset），双向显示——
// 当前目标是使用方（role from）→ 音频 item（绿圆播放钮 24 + 名称 + 波形 49×18：稿"点击播放
// 可以播放音频，切换音频 item 播放暂停"——组件内单例元素同时只播一条，点它条即切换）；
// 当前目标是音频（role to）→ 使用方文件卡（图标 + 名称）。
// 点名称 / 文件卡 = 跳转定位（relationOps.jumpToRelationTarget，同关系 item 口径）；
// × = 删除（hover 滑入，DetailNotes .del 同款）；添加钮 = 选择模式文件模板**过滤非音频**（02 稿）。
import { computed, onBeforeUnmount, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import SectionHeader from "../SectionHeader.vue";
import AppIcon from "../AppIcon.vue";
import IconButton from "../IconButton.vue";
import { getMediaSource } from "../../lib/mediaOps";
import {
  jumpToRelationTarget,
  relationsForTarget,
  removeRelation,
  type PickerTarget,
  type RelationView,
} from "../../lib/relationOps";
import { currentLibrary } from "../../lib/libraryOps";
import { useContentStore } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useToastStore } from "../../stores/toast";
import type { DetailTarget } from "../../lib/detailSections";

const props = defineProps<{ target: DetailTarget; title: string }>();
const content = useContentStore();
const picker = usePickerStore();
const toast = useToastStore();
const open = ref(true);

// 当前目标端点（关系匹配键：kind + id）
const targetRef = computed<{ kind: "asset" | "archive"; id: string }>(() =>
  props.target.kind === "file"
    ? { kind: "asset", id: props.target.placement.assetId }
    : { kind: "archive", id: props.target.node.id },
);

// 音色关系清单（relations 在 content store，undo / 增删后自动响应；kind = voice 过滤）
const views = computed(() =>
  relationsForTarget(content.relations, currentLibrary.name ?? "", targetRef.value).filter(
    (v) => v.relation.kind === "voice",
  ),
);

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

// 添加 = 进入选择模式（文件模板 + 过滤非音频）
function addVoice(): void {
  picker.enterVoice(originOf());
}

// —— 播放（稿 §5：点击播放 / 切换 item 播放暂停）——
// 组件内单例元素：同时只播一条，点其他 item = 切源；mp3/wav 直渲、wma 转码兜底（getMediaSource 既有口径）
const mediaRef = ref<HTMLAudioElement>();
const playingId = ref(""); // 正在出声的 relation id（"" = 无）
const loadingId = ref(""); // 源解析在途（连点防抖）
const playing = ref(false);

async function togglePlay(v: RelationView): Promise<void> {
  const el = mediaRef.value;
  if (!el) return;
  // 同一条 = 播放 / 暂停切换
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
    void el.play().catch(() => {});
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    loadingId.value = "";
  }
}

// 音频波形小图（49×18）：info 波形缩略图（5.5 既有产物）mask；origin Placement 优先
function waveStyle(assetId: string): Record<string, string> | null {
  const p =
    content.placements.find((pl) => pl.assetId === assetId && pl.origin) ??
    content.placements.find((pl) => pl.assetId === assetId);
  if (!p?.thumbPath) return null;
  return { "--wave-url": `url("${convertFileSrc(p.thumbPath)}")` };
}

onBeforeUnmount(() => {
  const el = mediaRef.value;
  if (el) {
    el.pause();
    el.removeAttribute("src");
    el.load();
  }
});

// 删除（×，hover 滑入）
async function remove(v: RelationView): Promise<void> {
  try {
    await removeRelation(v.relation.id);
    const i = content.relations.findIndex((r) => r.id === v.relation.id);
    if (i >= 0) content.relations.splice(i, 1);
    toast.notify({ message: "已删除链接音频" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}
</script>

<template>
  <section class="section-audio detail-block">
    <SectionHeader :title="title" grip v-model:open="open">
      <template #tail>
        <IconButton name="add-small" small title="链接音频" @click="addVoice" />
      </template>
    </SectionHeader>
    <!-- 空数据不渲染容器（空区块展开/折叠零高度变化，2026-08-15 走查口径） -->
    <div v-if="open && views.length > 0" class="body">
      <div v-for="v in views" :key="v.relation.id" class="audio-row">
        <!-- 音频 item（§5 稿：C4 / 40 高 / padding 8 12 / gap 4 / 圆角 6 / 1px 暗-悬停）：
             当前目标是使用方（role from）→ other 恒为音频（voice 后端保证） -->
        <div v-if="v.role === 'from'" class="audio-item">
          <button class="play-btn" v-tip="playingId === v.relation.id && playing ? '暂停' : '播放'" @click.stop="togglePlay(v)">
            <AppIcon :name="playingId === v.relation.id && playing ? 'pause' : 'play'" :size="9" />
          </button>
          <span class="name" v-tip="{ text: '跳转到文件' }" @click="jumpToRelationTarget(v.other)">{{ v.other.name }}</span>
          <span v-if="waveStyle(v.other.id)" class="wave" :style="waveStyle(v.other.id)!" />
          <IconButton name="clear" small title="删除链接音频" class="del" @click.stop="remove(v)" />
        </div>
        <!-- 使用方文件卡（当前目标是音频：显示"在哪些文件里使用过"） -->
        <div v-else class="file-chip" v-tip="{ text: '跳转到文件' }" @click="jumpToRelationTarget(v.other)">
          <AppIcon v-if="v.other.kind === 'archive'" name="archive" :size="15" class="deco" />
          <span class="name">{{ v.other.name }}</span>
          <IconButton name="clear" small title="删除链接音频" class="del" @click.stop="remove(v)" />
        </div>
      </div>
    </div>
    <audio
      ref="mediaRef"
      class="hidden-media"
      preload="auto"
      @play="playing = true"
      @pause="playing = false"
      @ended="playing = false"
    />
  </section>
</template>

<style scoped>
.section-audio {
  display: flex;
  flex-direction: column;
}
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
.audio-row {
  display: flex;
  align-items: center;
}
/* 音频 item（§5 稿：C4 / 40 高 / padding 8 12 / gap 4 / 圆角 6 / 1px 暗-悬停） */
.audio-item {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  flex: 1;
  min-width: 0;
  height: 40px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
/* 播放钮（§5 稿：24 绿圆 --success / 1px 亮-选中 border / padding 4 4 4 6；常态色三角） */
.play-btn {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  box-sizing: border-box;
  padding: 4px 4px 4px 6px;
  border: 1px solid var(--line-active);
  border-radius: 15px;
  background: var(--success);
  color: var(--text-normal);
  cursor: pointer;
}
.name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
}
/* 波形小图（§5 稿：49×18 / 状态-成功-底色；info 波形缩略图 mask） */
.wave {
  flex: none;
  width: 49px;
  height: 18px;
  background: var(--success-bg);
  -webkit-mask-image: var(--wave-url);
  mask-image: var(--wave-url);
  -webkit-mask-size: contain;
  mask-size: contain;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
}
/* 使用方文件卡（同关系 item 文件卡口径：C4 / 36 高 / padding 8 12 / 圆角 6 / 1px 暗-悬停） */
.file-chip {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--space-6);
  height: 36px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  cursor: pointer;
}
.file-chip:hover {
  background: var(--c5);
}
.deco {
  flex: none;
  color: var(--text-normal);
}
.hidden-media {
  display: none;
}
/* ×：常态不占位（宽 0），悬停行从右往左滑入（速查表 §9，DetailNotes .del 同款） */
.audio-row .del {
  width: 0;
  padding-left: 0;
  padding-right: 0;
  overflow: hidden;
  opacity: 0;
  color: var(--text-label);
  flex: none;
  transition:
    width 0.12s ease,
    opacity 0.12s ease;
}
.audio-row:hover .del {
  width: 24px;
  opacity: 1;
}
</style>
