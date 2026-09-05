<script setup lang="ts">
// 文件链接位置行列表（双行结构，2026-09-05 自 LinkListPopover 抽出共用）：
// 两处消费 = 卡片徽章 hover 弹窗（LinkListPopover 壳）与右键「打开文件链接所在位置」二级菜单
//（ContextMenu flyout linkList 面板，走查：二级菜单和徽章 Hover 弹窗一致）。
// 行 = 双行 56（padding 8 / gap 4 / 行间 1px 半透明-3 顶线，首行去线；hover 底 半透明-3）：
//   上 = 文件夹图标 18（双色调原值，档案目录换 archive）+ 路径两段（所属区块 / 所在目录，档案带 .档案 后缀）；
//   下 = 显示名（正文 三级）+ 徽章右置（origin = original-file-badge 14 品牌原值 / 链接 = linked-file-badge 13 三级）。
// 点击行 = 跳转定位（该位置视口 + 选中卡片）后 emit jump（关闭行为归父级：popover 关浮层 / 菜单关菜单）。
// 空态（活位置零条，仅菜单语境可达）：单行「（无）」占位不响应点击。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";
import { jumpToAssetPlacement } from "../lib/relationOps";
import { useContentStore, type FolderNode } from "../stores/content";

const props = defineProps<{ assetId: string }>();
const emit = defineEmits<{ jump: [] }>();
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
  segments: string[]; // 路径两段：区块 / 所在目录（档案目录带 .档案 后缀；完整父级链不放，2026-08-29 用户定）
  name: string;
  origin: boolean;
  isArchive: boolean; // 所在目录是档案（图标换 archive）
}

const rows = computed<Row[]>(() =>
  content.placements
    .filter((p) => p.assetId === props.assetId)
    .map((p) => {
      if (p.folderId === "root") {
        return { key: `${p.assetId}|root|${p.name}`, folderId: "root", segments: ["未分类"], name: p.name, origin: p.origin, isArchive: false };
      }
      const trail = pathOf(p.folderId) ?? [];
      const dir = trail[trail.length - 1];
      const isArchive = dir?.kind === "archive";
      const top = trail[0];
      const blockName = content.blocks.find((b) => b.id === (top?.block ?? "dir"))?.name ?? "目录";
      // 两级截止：区块 + 所在目录（中间父级省略）；dir 恒存在（非 root 必有父链）
      const segments = [blockName, ...(dir ? [isArchive ? `${dir.name}.档案` : dir.name] : [])];
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
  jumpToAssetPlacement(props.assetId, r.folderId);
  emit("jump");
}
</script>

<template>
  <div class="link-rows">
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
    <div v-if="rows.length === 0" class="empty">（无）</div>
  </div>
</template>

<style scoped>
.link-rows {
  display: flex;
  flex-direction: column;
  width: max-content;
  min-width: 180px; /* 2026-09-05 走查：内容太窄看着累，最小宽 180（用户定） */
  max-width: 360px;
}
/* 行（悬停-链接列表稿：双行 56 高 = padding 8 + 18 + gap 4 + 18；行间 1px 半透明-3 顶线，首行去线；hover 底 半透明-3） */
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
/* 空态（仅菜单语境可达）：单行占位，三级色 */
.empty {
  padding: var(--n-space-4) var(--n-space-8);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
}
</style>
