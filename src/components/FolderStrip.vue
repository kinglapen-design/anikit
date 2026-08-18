<script setup lang="ts">
// 文件夹卡片区（figma-css 02_布局/42_中间面板，2026-08-10 芯片样式重做）：
// 区块标题行右侧带"递归显示"复选框（SectionHeader tail 插槽，绑 ui.showSubfolders——
// PRD 04-1 显示子文件夹内容开关，2026-08-10 接入 placementsFor：开 = 视口含全部后代文件夹内容）；
// 卡片横条芯片定高 65：grid auto-fill 弹性列宽（min 185 = figma 稿宽）换行，不再跟随滑块缩放
//（新稿定尺寸，2026-07-23 滑块联动定案作废）；随网格统一滚动（在 AssetGrid canvas 内）。
// 1.2 多标签页：子文件夹按本标签页 viewKey 取；双击进入经 tabs.openView（锁定标签页会新开）。
import { computed, ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import AppCheckbox from "./AppCheckbox.vue";
import FolderCard from "./FolderCard.vue";
import { dragView } from "../lib/dragOperation";
import { openFolderCardMenu } from "../lib/menuSpecs/gridDispatch";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ viewKey: string }>();
const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const ui = useUiStore();
const folderOpen = ref(true);

// 本标签页视口的直接子文件夹（1.2 per-view 参数化）
const stripFolders = computed(() => content.stripFoldersFor(props.viewKey));

// 文件夹卡片选中（2026-07-28 复验反馈 5：迁 selection store 文件夹选择域——单击 = 单选 /
// Ctrl+单击 = 加选切换 / Shift+单击 = 按条带顺序锚点范围选；与素材 / 收件箱条目互斥，
// 互斥清理由 clickFolder 内部统一做）；有效拖拽结束后的紧随 click 不触发（C4 引擎置位）
function onSelect(id: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
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
function onMenu(f: { id: string; name: string }, e: MouseEvent): void {
  if (!selection.isFolderSelected(f.id)) selection.clickFolder(f.id);
  openFolderCardMenu(e, f);
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
    <!-- 区块标题（figma 42）：右侧递归显示复选框（label 按 42 稿标签档 12/17 覆盖组件默认 14/20） -->
    <SectionHeader title="文件夹" :count="stripFolders.length" v-model:open="folderOpen">
      <template #tail>
        <AppCheckbox v-model="ui.showSubfolders" label="递归显示" class="recursive-cb" />
      </template>
    </SectionHeader>
    <div v-if="folderOpen" class="flow">
      <FolderCard
        v-for="f in stripFolders"
        :key="f.id"
        :folder="f"
        class="cell"
        :selected="selection.isFolderSelected(f.id)"
        :dimmed="dragView.folderIds?.has(f.id) ?? false"
        :cut="selection.isFolderCut(f.id)"
        @select="onSelect(f.id, $event)"
        @dblclick="onEnter(f.id)"
        @contextmenu.prevent.stop="onMenu(f, $event)"
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
/* 文件夹卡片区块（figma 42）：左/上/下 padding 0（区块容器 asset-section 承担 16 边距）；
   右 16 对齐网格 PAD.right（卡片右缘距中栏外边框 16，不与贴边滚动条重叠）；标题↔卡片 gap 12 */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding-right: var(--space-16);
}
/* 递归显示复选框：label 按 42 稿标签档 12/17（组件 AppCheckbox 默认正文 14/20） */
.recursive-cb :deep(.label) {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
}
/* 卡片列表：grid 均分同宽（auto-fill 空轨道同宽，末行不被拉宽），满了换行；
   min 185 = figma 42 稿卡片宽；列/行间距 12（figma 42 文件夹卡片列表 gap 12，2026-08-10）；
   查看方式的自适应 / 瀑布 / 网格只影响素材卡片，与文件夹卡片无关 */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(185px, 1fr));
  column-gap: var(--space-12);
  row-gap: var(--space-12);
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
