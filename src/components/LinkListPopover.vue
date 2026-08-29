<script setup lang="ts">
// 徽章悬停链接列表浮层（阶段七稿：docs/UI 重构/Design Token/组件/悬浮栏/悬停-tip/悬停-链接列表）：
// 悬停卡片徽章（origin ★ / 链接 icon）弹出该文件全部活位置列表。
// 壳 = 通用/半透明-1 + 1px 亮色/半透明-3 真 border + 小阴影 + blur 小 + 圆角 6；宽随内容（max-content，上限 360）。
// 行 = 双行结构（padding 8 / gap 4 / 行间 1px 半透明-3 顶线，首行去线；hover 底 半透明-3）：
//   上 = 文件夹图标 18（双色调原值）+ 全路径（首段 正文-加粗 二级、分隔 / 二级、后续段 三级）；
//   下 = 显示名（正文 三级）+ 徽章（origin = original-file-badge 14 品牌色原值 / 链接 = linked-file-badge 13 三级）。
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
        <div class="line">
          <AppIcon :name="r.isArchive ? 'archive' : 'folder-closed'" :size="18" class="deco" />
          <span class="path" v-tip="r.segments.join(' / ')">
            <template v-for="(s, si) in r.segments" :key="si">
              <span v-if="si > 0" class="sep">/</span><span class="seg" :class="{ head: si === 0 }">{{ s }}</span>
            </template>
          </span>
        </div>
        <div class="line name-line">
          <span class="name" v-tip="r.name">{{ r.name }}</span>
          <AppIcon v-if="r.origin" name="original-file-badge" :size="14" class="badge" />
          <AppIcon v-else name="linked-file-badge" :size="13" class="badge link" />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
/* 浮层壳（悬停-链接列表稿）：通用/半透明-1 + 1px 半透明-3 真 border + 小阴影 + blur 小 + 圆角 6；
   宽随内容（稿 182 = 内容快照），上限 360 超出走省略；z-2000 同悬停浮层档 */
.link-pop {
  position: fixed;
  z-index: 2000;
  display: flex;
  flex-direction: column;
  width: max-content;
  max-width: 360px;
  border: 1px solid var(--n-l-a3);
  border-radius: var(--n-radius-6);
  background: var(--n-g-a1);
  box-shadow: var(--n-shadow-sm);
  backdrop-filter: blur(var(--n-blur-sm));
  overflow: hidden;
}
/* 行（稿：双行 56 高 = padding 8 + 18 + gap 4 + 18；行间 1px 半透明-3 顶线，首行去线；hover 底 半透明-3） */
.row {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--n-space-4);
  padding: var(--n-space-8);
  border-top: 1px solid var(--n-l-a3);
  cursor: pointer;
}
.row.first {
  border-top: none;
}
.row:hover {
  background: var(--n-l-a3);
}
.line {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 18px;
  align-self: stretch;
  min-width: 0;
}
.deco {
  flex: none;
}
/* 全路径：首段 正文-加粗 二级 / 分隔符 二级 / 后续段 三级 */
.path {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
}
.seg {
  color: var(--n-l3);
}
.seg.head {
  color: var(--n-l2);
}
.sep {
  color: var(--n-l2);
  flex: none;
}
/* 下行：显示名（正文 三级）+ 徽章右置（space-between，稿） */
.name-line {
  justify-content: space-between;
  gap: var(--n-space-8);
}
.name {
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.badge {
  flex: none;
}
.badge.link {
  color: var(--n-l3);
}
</style>
