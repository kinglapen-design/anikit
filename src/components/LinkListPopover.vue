<script setup lang="ts">
// 徽章悬停链接列表浮层（6.2 四轮走查⑤，用户效果图）：悬停卡片徽章（origin ★ / 链接 icon）
// 弹出该文件全部活位置列表——行 = 文件夹图标 18 + 全路径（区块 / 父级… / 目录，常态色段
// 标签色分隔；档案目录「名称.档案」后缀；未分类 = 未分类）+ 显示名 + 徽章；
// 点击行 = 跳转定位（该位置视口 + 选中卡片，同链接区块 item / 关系 item 口径）后关闭。
// 机制：Teleport body fixed 定位（徽章上方，右对齐；上方不够翻下方），150ms 延迟出，
// 徽章→浮层 hover 桥（120ms 容错），Esc / 行点击关闭。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import { jumpToAssetPlacement } from "../lib/relationOps";
import { useContentStore, type FolderNode, type PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView; anchor?: HTMLElement }>();
const emit = defineEmits<{ close: []; hover: [inside: boolean] }>();
const content = useContentStore();

// —— 路径链（区块 / 父级… / 目录）：DFS 找父链（FolderNode 无 parent 指针）；root = null ——
function pathOf(folderId: string): FolderNode[] | null {
  const walk = (nodes: FolderNode[], trail: FolderNode[]): FolderNode[] | null => {
    for (const n of nodes) {
      if (n.id === folderId) return [...trail, n];
      const hit = walk(n.children, [...trail, n]);
      if (hit) return hit;
    }
    return null;
  };
  return walk(content.folders, []);
}

interface Row {
  key: string;
  folderId: string;
  segments: string[]; // 全路径段（区块 / … / 目录；档案目录带 .档案 后缀）
  name: string;
  origin: boolean;
  isArchive: boolean; // 所在目录是档案（图标换 archive）
}

const rows = computed<Row[]>(() =>
  content.placements
    .filter((p) => p.assetId === props.asset.assetId)
    .map((p) => {
      if (p.folderId === "root") {
        return { key: `${p.assetId}|root|${p.name}`, folderId: "root", segments: ["未分类"], name: p.name, origin: p.origin, isArchive: false };
      }
      const trail = pathOf(p.folderId) ?? [];
      const dir = trail[trail.length - 1];
      const isArchive = dir?.kind === "archive";
      const top = trail[0];
      const blockName = content.blocks.find((b) => b.id === (top?.block ?? "dir"))?.name ?? "目录";
      const segments = [
        blockName,
        ...trail.map((n) => (n.kind === "archive" ? `${n.name}.档案` : n.name)),
      ];
      return {
        key: `${p.assetId}|${p.folderId}|${p.name}`,
        folderId: p.folderId,
        segments,
        name: p.name,
        origin: p.origin,
        isArchive,
      };
    }),
);

function jump(r: Row): void {
  jumpToAssetPlacement(props.asset.assetId, r.folderId);
  emit("close");
}

// —— 定位（徽章上方右对齐；上方空间不足翻下方）——
const pos = ref({ left: 0, top: 0 });
const popRef = ref<HTMLElement>();
function place(): void {
  const a = props.anchor;
  if (!a) return;
  const r = a.getBoundingClientRect();
  const el = popRef.value;
  const h = el?.offsetHeight ?? 0;
  const w = el?.offsetWidth ?? 0;
  let top = r.top - h - 4;
  if (top < 8) top = r.bottom + 4; // 上方不够翻下方
  let left = r.right - w;
  if (left < 8) left = 8;
  pos.value = { left, top };
}

// —— 开关：150ms 延迟出（防划过）；Esc 关闭 ——
const shown = ref(false);
let timer = 0;
onMounted(() => {
  timer = window.setTimeout(() => {
    shown.value = true;
    requestAnimationFrame(place);
  }, 150);
  window.addEventListener("keydown", onKey, true);
});
onBeforeUnmount(() => {
  window.clearTimeout(timer);
  window.removeEventListener("keydown", onKey, true);
});
function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.stopPropagation();
    emit("close");
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="shown"
      ref="popRef"
      class="link-pop"
      :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
      @mouseenter="emit('hover', true)"
      @mouseleave="emit('hover', false)"
    >
      <div v-for="(r, i) in rows" :key="r.key" class="row" :class="{ first: i === 0 }" @click="jump(r)">
        <AppIcon :name="r.isArchive ? 'archive' : 'folder-closed'" :size="18" class="deco" />
        <span class="path" v-tip="r.segments.join(' / ')">
          <template v-for="(s, si) in r.segments" :key="si">
            <span v-if="si > 0" class="sep">/</span><span :class="si === 0 ? 'seg block' : 'seg'">{{ s }}</span>
          </template>
        </span>
        <span class="name" v-tip="r.name">{{ r.name }}</span>
        <AppIcon v-if="r.origin" name="original-file-badge" :size="15" class="badge" />
        <AppIcon v-else name="linked-file-badge" :size="13" class="badge link" />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
/* 浮层（用户效果图）：C4 底 + 1px 亮-选中真 border + 圆角 8 + --shadow-pop；z-2000 同悬停浮层档 */
.link-pop {
  position: fixed;
  z-index: 2000;
  display: flex;
  flex-direction: column;
  min-width: 260px;
  max-width: 360px;
  border: 1px solid var(--line-active);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: var(--shadow-pop);
  overflow: hidden;
}
/* 行（效果图：44 高 / padding 8 12 / 行间 1px 暗-悬停线，首行去线） */
.row {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  height: 44px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-top: 1px solid var(--line-hover);
  cursor: pointer;
}
.row.first {
  border-top: none;
}
.row:hover {
  background: var(--c5);
}
.deco {
  flex: none;
  color: var(--text-normal);
}
.path {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.seg {
  color: var(--text-label);
}
.seg.block {
  color: var(--text-normal);
}
.sep {
  color: var(--text-label);
  flex: none;
}
.name {
  flex: none;
  max-width: 40%;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.badge {
  flex: none;
}
.badge.link {
  color: var(--text-label);
}
</style>
