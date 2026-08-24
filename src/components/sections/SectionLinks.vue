<script setup lang="ts">
// 区块 · 链接区块（3.2 一轮只读点亮；6.2 三轮编辑点亮——添加钮 = 链接管理弹窗（17 稿，
// 用户定"链接区块的添加就是进入文件链接管理面板"），× = 移除该条链接记录（sync_asset_links
// 单条即时提交；origin 行同可移除——位置级模型一视同仁，后端护栏防位置归 0）：
// 两行卡（C4 / padding 8 8 8 12 / 圆角 6 / gap 8 / 1px 暗-悬停，hover C5）——
// 行1 = 文件夹图标 18 + 区块名（14 常态）/ 所属目录（14 标签色，`/` 分隔符色随前段）
// + ×（hover 从右往左滑入平时不占位，DetailNotes .del 同款）；
// 行2 = 文件显示名 12/17 标签色 + 原始文件徽章 15px（origin=true 才出）。
// 所属目录 = PlacementLink.path 末段；未分类 = 「目录 / 未分类」；目录是档案（folderId 查
// content.folders 树 kind === "archive"，不改后端）→ 显示「名称.档案」（如 目录 / 人物.档案）。
// 档案形态 / 无 Placement = 空内容（标题照常，2026-08-15 定案：无内容区块不隐藏）；
// 档案的链接管理随档案多位置立项（2026-08-24 用户定），添加钮仅文件形态出。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppIcon from "../AppIcon.vue";
import IconButton from "../IconButton.vue";
import { findFolderNode, type DetailTarget, type PlacementLink } from "../../lib/detailSections";
import { syncAssetLinks } from "../../lib/placementOps";
import { useContentStore } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";

const props = defineProps<{ target: DetailTarget; title: string }>();
const content = useContentStore();
const selection = useSelectionStore();
const open = ref(true);

const items = computed<PlacementLink[]>(() =>
  props.target.kind === "file" ? props.target.detail.placements : [],
);

// 所属目录显示名：path 末段；所在目录是档案 → 「名称.档案」（2026-08-15 定案，前端树判定）
function dirName(p: PlacementLink): string {
  const name = p.path[p.path.length - 1] ?? "";
  if (p.folderId && findFolderNode(content.folders, p.folderId)?.kind === "archive") {
    return `${name}.档案`;
  }
  return name;
}

// 添加 = 链接管理弹窗（17 稿；用户定"链接区块的添加就是进入文件链接管理面板"）
function openLinkManage(): void {
  if (props.target.kind !== "file") return;
  selection.linkManageDraft = { assetId: props.target.placement.assetId, name: props.target.placement.name };
}

// × = 移除该条链接记录（直接删 Placement，不进回收站——2026-08-24 用户定；归 0 后端护栏报错）；
// name 精确定位：同文件夹重复链接（2026-08-13 放开）只移除本行
async function removeLink(p: PlacementLink): Promise<void> {
  if (props.target.kind !== "file") return;
  const assetId = props.target.placement.assetId;
  await syncAssetLinks(assetId, [], [{ folderId: p.folderId ?? "root", origin: p.origin, name: p.name }], []);
}
</script>

<template>
  <section class="section-links detail-block">
    <SectionHeader :title="title" grip v-model:open="open">
      <template #tail>
        <!-- 添加钮 = 链接管理弹窗（仅文件形态；档案随多位置立项） -->
        <IconButton v-if="target.kind === 'file'" name="add-small" small title="链接管理" @click="openLinkManage" />
      </template>
    </SectionHeader>
    <!-- 空数据不渲染容器（2026-08-15 走查：父级 detail-block gap 会让空 .body 也撑出 8px，
         展开/折叠必须零高度变化） -->
    <div v-if="open && items.length > 0" class="body">
      <div v-for="(p, i) in items" :key="`${p.folderId ?? 'root'}|${p.name}|${i}`" class="link-item">
        <div class="row1">
          <AppIcon name="folder-closed" :size="18" class="deco" />
          <span class="seg block">{{ p.blockName }}</span>
          <span class="sep">/</span>
          <span class="seg dir" v-tip="dirName(p)">{{ dirName(p) }}</span>
          <span class="spacer" />
          <IconButton name="clear" small title="移除链接" class="del" @click="removeLink(p)" />
        </div>
        <!-- 行2（稿：名称 + 链接状态徽章，justify-between / padding 0 4 0 0 / gap 10）：
             原始文件徽章 15px 品牌色，origin=true 才出；链接 Placement 不出 -->
        <div class="row2">
          <span class="file-name">{{ p.name }}</span>
          <AppIcon v-if="p.origin" name="original-file-badge" :size="15" class="origin-badge" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.section-links {
  display: flex;
  flex-direction: column;
}
/* item 容器：间距 8 */
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 两行卡（§2）：C4 / padding 8 8 8 12 / gap 8 / 圆角 6 / 1px 暗-悬停；hover C5 */
.link-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.link-item:hover {
  background: var(--c5);
}
/* 行1（2026-08-15 定案两段化）：图标 18 + 区块名 14 常态 / 所属目录 14 标签色 + ×（高 20，间距 4） */
.row1 {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: 20px;
}
.deco {
  color: var(--text-normal);
  flex: none;
}
.seg,
.sep {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
}
.seg.block,
.sep {
  color: var(--text-normal);
}
.seg.dir {
  color: var(--text-label);
}
.seg {
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.seg.block {
  flex: none;
}
.spacer {
  flex: 1;
  min-width: var(--space-4);
}
/* 行2（稿：名称 + 链接状态徽章）：justify-between / padding 0 4 0 0 / gap 10；
   文件显示名 12/17 标签色单行省略；原始文件徽章 15px（品牌色，SVG 自带） */
.row2 {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding-right: var(--space-4);
}
.file-name {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.origin-badge {
  flex: none;
}
/* ×：常态不占位隐藏（宽度 0），悬停 item 展开 24px 露出（DetailNotes .del 同款机制）；
   选择器带 .link-item 提权，覆盖 IconButton.small 的固定 width 24 */
.link-item .del {
  width: 0;
  padding-left: 0;
  padding-right: 0;
  overflow: hidden;
  opacity: 0;
  color: var(--text-label);
  flex: none;
  transition:
    width 0.12s ease,
    opacity 0.12s ease,
    color 0.12s ease;
}
.link-item:hover .del {
  width: 24px;
  opacity: 1;
}
</style>
