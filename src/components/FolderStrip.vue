<script setup lang="ts">
// 文件夹卡片区（7.1 内容容器轮贴稿——中间面板-内容容器-列表容器-文件夹列表）：
// 卡片 = cards/FolderCard（220×122 稿比例：grid auto-fill minmax 220 弹性列宽 + auto-rows 122 定高，
// 列表 gap 12；不跟随滑块缩放，查看方式的自适应/瀑布/网格只影响素材卡片）；
// 区块标题 = items/SectionTitle block（稿无动作钮 → actions=false）+ tail「递归显示」复选框
//（ui.showSubfolders——PRD 04-1 显示子文件夹内容开关）；
// 简介两行（稿）：「N 个文件 / 容量」+「M 个档案 / K 个子文件夹」——容量 = 后端 totalSize 递归去重合计
//（7.1 内容容器轮扩展，与 count 同径）；档案/子文件夹 = 直接子级按 kind 分流计数。
// 行内重命名（上移自旧 FolderCard）：入口 = 名称双击（卡片 nameEdit）/ 右键菜单
//（selection.namingFolderCardId）；提交 = 非空且变了才 renameFolder。
// 1.2 多标签页：子文件夹按本标签页 viewKey 取；双击进入经 tabs.openView（锁定标签页会新开）。
import { computed, ref, watch } from "vue";
import SectionTitle from "./items/SectionTitle.vue";
import AppCheckbox from "./AppCheckbox.vue";
import FolderCard from "./cards/FolderCard.vue";
import { dragView } from "../lib/dragOperation";
import { formatSize } from "../lib/formatSize";
import { openFolderCardMenu } from "../lib/menuSpecs/gridDispatch";
import { renameFolder } from "../lib/placementOps";
import { useContentStore, type FolderNode } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ viewKey: string }>();
const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const ui = useUiStore();
const folderOpen = ref(true);
// 折叠切换：SectionTitle 守卫在调用方（§20——dragView.suppressClick 吞拖拽后紧随 click）
function onFold(): void {
  if (dragView.suppressClick) return;
  folderOpen.value = !folderOpen.value;
}

// 本标签页视口的直接子文件夹（1.2 per-view 参数化）
const stripFolders = computed(() => content.stripFoldersFor(props.viewKey));

// 文件夹卡简介两行（稿值结构恒定）：行1 = 文件数 / 容量；行2 = 档案数 / 子文件夹数（直接子级按 kind 分流）
function folderLines(f: FolderNode): string[][] {
  const archives = f.children.filter((c) => c.kind === "archive").length;
  const subs = f.children.length - archives;
  return [
    [`${f.count.toLocaleString()} 个文件`, formatSize(f.totalSize)],
    [`${archives.toLocaleString()} 个档案`, `${subs.toLocaleString()} 个子文件夹`],
  ];
}

// 文件夹卡片选中（2026-07-28 复验反馈 5：迁 selection store 文件夹选择域——单击 = 单选 /
// Ctrl+单击 = 加选切换 / Shift+单击 = 按条带顺序锚点范围选；与素材 / 收件箱条目互斥，
// 互斥清理由 clickFolder 内部统一做）；有效拖拽结束后的紧随 click 不触发（C4 引擎置位）
function onSelect(id: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
  if (usePickerStore().active) return; // 选择模式（6.2）：文件夹不是可选目标，单击无操作（双击导航保留）
  selection.clickFolder(
    id,
    { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey },
    stripFolders.value.map((f) => f.id),
  );
}

// 双击卡片（非名称区域）= 进入该文件夹（2026-07-29 复验三轮；1.2 经 tabs.openView——
// 当前标签页锁定则新开标签页）；sidebarSelected watch 会清选择，无需手动；名称双击在卡片内 stop 不会到此
function onEnter(id: string): void {
  tabs.openView(`folder:${id}`);
}

// 右键（6.3；1.3 一轮按 05 稿 320 宽 + 搜索框）：未选中项先单选（与树 item 同规则），全局菜单宿主
// 选择模式（6.2）：不入选择域，直接出菜单（spec 内精简为查看类）
function onMenu(f: { id: string; name: string }, e: MouseEvent): void {
  if (usePickerStore().active) {
    openFolderCardMenu(e, f);
    return;
  }
  if (!selection.isFolderSelected(f.id)) selection.clickFolder(f.id);
  openFolderCardMenu(e, f);
}

// —— 行内重命名（7.1 上移自旧 FolderCard，TreeRowItem 同口径：空/不变判定在调用方）——
const namingId = ref<string | null>(null);
// 右键菜单"重命名"（6.3）：经 selection.namingFolderCardId 触发（与名称双击同入口）
watch(
  () => selection.namingFolderCardId,
  (id) => {
    if (id && stripFolders.value.some((f) => f.id === id)) {
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

// 同级排序间隙竖块（2026-08-18 修复"首列前插提示线不可见"：卡片内 left:-12 的竖块会被
// grid-viewport overflow 整条裁掉——改由 FolderStrip 单件 Teleport 到 body 按 client 坐标画；
// 拖拽中 folderGapId 随指针 move 变更 → 本 computed 逐次重取 rect）
const gapBar = computed(() => {
  const id = dragView.folderGapId;
  if (!id) return null;
  const el = document.querySelector<HTMLElement>(`.folder-card[data-folder-id="${CSS.escape(id)}"]`);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return { left: dragView.folderGapAfter ? r.right : r.left - 12, top: r.top, height: r.height };
});

</script>

<template>
  <section class="folder-block">
    <!-- 区块标题（7.1 内容容器轮 = items/SectionTitle block 贴稿；稿无动作钮 → actions=false）：
         右侧递归显示复选框（tail 插槽常驻，绑 ui.showSubfolders——PRD 04-1 显示子文件夹内容开关） -->
    <SectionTitle
      variant="block"
      title="文件夹"
      :count="stripFolders.length"
      :expanded="folderOpen"
      :actions="false"
      @toggle="onFold"
    >
      <template #tail>
        <AppCheckbox v-model="ui.showSubfolders" label="递归显示" class="recursive-cb" />
      </template>
    </SectionTitle>
    <div v-if="folderOpen" class="flow">
      <FolderCard
        v-for="f in stripFolders"
        :key="f.id"
        :name="f.name"
        :lines="folderLines(f)"
        class="cell"
        :selected="selection.isFolderSelected(f.id)"
        :dimmed="dragView.folderIds?.has(f.id) ?? false"
        :cut="selection.isFolderCut(f.id)"
        :naming="namingId === f.id"
        :drop-into="dragView.folderCardId === f.id"
        :data-folder-id="f.id"
        :data-drag-name="f.name"
        @click="onSelect(f.id, $event)"
        @dblclick="onEnter(f.id)"
        @contextmenu.prevent.stop="onMenu(f, $event)"
        @name-edit="namingId = f.id"
        @name-commit="onRenameCommit(f, $event)"
      />
    </div>
    <!-- 同级排序间隙竖块（Teleport body / client 坐标，2026-08-18——首列前插不被 viewport 裁剪） -->
    <Teleport to="body">
      <span
        v-if="gapBar"
        class="folder-drop-gap"
        :style="{ left: `${gapBar.left}px`, top: `${gapBar.top}px`, height: `${gapBar.height}px` }"
        aria-hidden="true"
      />
    </Teleport>
  </section>
</template>

<style scoped>
/* 文件夹卡片区块（7.1 内容容器轮）：四缘 padding 0——区块边距由列表容器（asset-section 12）
   与网格 PAD.right 12 承担（旧 padding-right 16 随骨架贴稿撤除）；标题↔卡片 gap 12 */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
/* 递归显示复选框：label 按新档次要文字 12/16（组件 AppCheckbox 默认正文档 13/18） */
.recursive-cb :deep(.label) {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
/* 卡片列表（稿）：grid 均分同宽（auto-fill 空轨道同宽，末行不被拉宽），满了换行；
   min 220 = 稿卡片宽，行高 122 定高（122 高链 = padding 24 + 名称 26 + 间距 36 + 简介 36）；
   列/行间距 12（稿 文件夹列表 gap 12） */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  grid-auto-rows: 122px;
  column-gap: var(--n-space-12);
  row-gap: var(--n-space-12);
}
.cell {
  min-width: 0;
}
/* 同级排序间隙竖块（Teleport 到 body，fixed 定位）：宽 12 = 卡片间距档 / 高随目标卡片，
   圆角竖向矩形色块；填充 30%（2026-08-10 用户定档）——原卡片内 left:-12 方案被 viewport
   裁剪废除（2026-08-18，首列前插提示线不可见的根因） */
.folder-drop-gap {
  position: fixed;
  z-index: 5;
  width: 12px;
  border-radius: var(--radius-4);
  background: var(--brand-1-30);
  box-shadow: inset 0 0 0 1px var(--brand-1);
  pointer-events: none;
}
</style>
