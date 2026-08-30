<script setup lang="ts">
// 档案卡片区（2026-08-15 档案子系统一轮，PRD 02-03 三区：文件夹 / 档案 / 文件；
// 7.1 内容容器轮贴稿——中间面板-内容容器-列表容器-档案列表）：
// 卡片 = cards/ArchiveCard（正方形 aspect-ratio 1 稿注；grid auto-fill minmax 250 弹性列宽，列表 gap 12）；
// 区块标题 = items/SectionTitle block（稿无动作钮 → actions=false）。
// 卡面数据：封面 = 自定义封面优先（covers/）回退子项首图（folderThumbMap 既有口径）；
// 左上徽标 = ArchiveData.badge 文字（统一 FormatBadge 铬，旧 badge.color 底色芯片口径作废）；
// 属性行 = ArchiveData.props（" | " 分隔）；数量行 = 「N 个文件 / 容量」（容量 = list_archives
// totalSize 去重合计，7.1 扩展）；右下链接徽章业务口径未定 → 恒 false（09 待办）。
// 无档案时整块不渲染（AssetGrid v-if，同"子项无文件夹隐藏区块"既有口径）；
// 交互口径同 FolderStrip（单击选中 / 双击进入 / 右键 04 稿档案菜单），随网格统一滚动。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import SectionTitle from "./items/SectionTitle.vue";
import ArchiveCard from "./cards/ArchiveCard.vue";
import { dragView } from "../lib/dragOperation";
import { formatSize } from "../lib/formatSize";
import { openArchiveCardMenu } from "../lib/menuSpecs/gridDispatch";
import { renameFolder } from "../lib/placementOps";
import { useContentStore, type FolderNode } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";

const props = defineProps<{ viewKey: string; nodes?: FolderNode[] }>();
const content = useContentStore();
const picker = usePickerStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const archiveOpen = ref(true);
// 折叠切换：SectionTitle 守卫在调用方（§20——dragView.suppressClick 吞拖拽后紧随 click）
function onFold(): void {
  if (dragView.suppressClick) return;
  archiveOpen.value = !archiveOpen.value;
}

// 本标签页视口的直接子级档案（1.2 per-view 参数化）；
// nodes 传入时取而代之（3.6 搜索态：命中档案列表由 ContentArea 按 search store 装配）
const stripArchives = computed(() => props.nodes ?? content.stripArchivesFor(props.viewKey));

// 卡面数据装配：封面（自定义封面优先，回退子项首图）/ 徽标文字 / 属性行 / 数量行
function coverOf(id: string): string {
  const p = content.archives[id]?.coverPath ?? content.folderThumbMap.get(id);
  return p ? convertFileSrc(p) : "";
}
function badgeOf(id: string): string {
  return content.archives[id]?.badge?.text ?? "";
}
function attrsOf(id: string): string[] {
  return content.archives[id]?.props ?? [];
}
function statsOf(f: FolderNode): string[] {
  return [`${f.count.toLocaleString()} 个文件`, formatSize(content.archives[f.id]?.totalSize ?? 0)];
}
// 选择模式（6.2 一轮）：已选压暗 + 大勾；发起档案自身（2026-08-24 用户定）= 半透明 + 禁选
function isPickerSelf(id: string): boolean {
  return picker.active && picker.origin?.kind === "archive" && picker.origin.id === id;
}

// 选中口径同 FolderStrip（文件夹选择域；单击单选 / Ctrl 加选 / Shift 按条带顺序范围选）
// 选择模式（6.2 一轮）：单击档案卡 = 加入/移出备选（档案可作关系目标），不再选中
function onSelect(id: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
  if (picker.active) {
    const node = stripArchives.value.find((f) => f.id === id);
    if (node) {
      picker.togglePick({
        kind: "archive",
        id,
        name: node.name,
        thumb: content.archives[id]?.coverPath ?? content.folderThumbMap.get(id) ?? null,
      });
    }
    return;
  }
  selection.clickFolder(
    id,
    { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey },
    stripArchives.value.map((f) => f.id),
  );
}

// 双击卡片（非名称区域）= 进入该档案（同 FolderStrip 口径）
function onEnter(id: string): void {
  tabs.openView(`folder:${id}`);
}

// 右键（3.2 一轮，04 稿档案菜单 320 宽 + 搜索框）：未选中项先单选（与树 item 同规则），全局菜单宿主；
// 选择模式（6.2）：不入选择域，直接出菜单（spec 内已分流精简集）
function onMenu(f: { id: string; name: string }, e: MouseEvent): void {
  if (!picker.active && !selection.isFolderSelected(f.id)) selection.clickFolder(f.id);
  openArchiveCardMenu(e, f);
}

// —— 行内重命名（7.1 上移自旧 ArchiveCard，TreeRowItem 同口径：空/不变判定在调用方）——
const namingId = ref<string | null>(null);
// 右键菜单"重命名"（04 稿）：经 selection.namingFolderCardId 触发（与名称双击同入口）
watch(
  () => selection.namingFolderCardId,
  (id) => {
    if (id && stripArchives.value.some((f) => f.id === id)) {
      selection.namingFolderCardId = null;
      namingId.value = id;
    }
  },
);
async function onRenameCommit(f: FolderNode, name: string): Promise<void> {
  namingId.value = null;
  if (!name || name === f.name) return;
  await renameFolder(f.id, name);
}
</script>

<template>
  <section class="archive-block">
    <SectionTitle
      variant="block"
      title="档案"
      :count="stripArchives.length"
      :expanded="archiveOpen"
      :actions="false"
      @toggle="onFold"
    />
    <div v-if="archiveOpen" class="flow">
      <ArchiveCard
        v-for="f in stripArchives"
        :key="f.id"
        :name="f.name"
        :cover-url="coverOf(f.id)"
        :badge="badgeOf(f.id)"
        :attrs="attrsOf(f.id)"
        :stats="statsOf(f)"
        class="cell"
        :selected="selection.isFolderSelected(f.id)"
        :dimmed="dragView.folderIds?.has(f.id) ?? false"
        :cut="selection.isFolderCut(f.id)"
        :naming="namingId === f.id"
        :drop-into="dragView.folderCardId === f.id"
        :picked="picker.active && picker.isPicked('archive', f.id)"
        :picker-self="isPickerSelf(f.id)"
        :data-folder-id="f.id"
        :data-drag-name="f.name"
        @click="onSelect(f.id, $event)"
        @dblclick="onEnter(f.id)"
        @contextmenu.prevent.stop="onMenu(f, $event)"
        @name-edit="namingId = f.id"
        @name-commit="onRenameCommit(f, $event)"
      />
    </div>
  </section>
</template>

<style scoped>
/* 档案卡片区块：容器口径同 FolderStrip（四缘 padding 0，边距归列表容器与 PAD.right 12；标题↔卡片 gap 12） */
.archive-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
/* 卡片列表（稿）：grid auto-fill 弹性列宽（min 250 = 稿卡片宽）；间距 12（稿 档案列表 gap 12，
   旧 24 随贴稿改）；卡片正方形比例由 ArchiveCard aspect-ratio 保持 */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  column-gap: var(--n-space-12);
  row-gap: var(--n-space-12);
}
.cell {
  min-width: 0;
}
</style>
