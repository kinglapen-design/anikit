<script setup lang="ts">
// 文件夹卡片区（7.1 内容容器轮贴稿——中间面板-内容容器-列表容器-文件夹列表）：
// 卡片 = cards/FolderCard（122 定高稿比例：grid auto-fill minmax 180 弹性列宽 + auto-rows 122 定高，
// 列表 gap 12；2026-09-13 实机走查列宽 min 220→180——一排能放四个放三个显太宽；不跟随滑块缩放，
// 查看方式的自适应/瀑布/网格只影响素材卡片）；
// 区块标题 = items/SectionTitle block（稿无动作钮 → actions=false）；递归显示开关撤出本行
//（2026-08-30 用户定：查看方式菜单里有就行——ViewOptionsMenu AppSwitch 仍在）；
// 简介两行（稿）：「N 个文件 / 容量」+「M 个档案 / K 个子文件夹」——容量 = 后端 totalSize 递归去重合计
//（7.1 内容容器轮扩展，与 count 同径）；档案/子文件夹 = 直接子级按 kind 分流计数。
// 行内重命名（上移自旧 FolderCard）：入口 = 名称双击（卡片 nameEdit）/ 右键菜单
//（selection.namingFolderCardId）；提交 = 非空且变了才 renameFolder。
// 1.2 多标签页：子文件夹按本标签页 viewKey 取；双击进入经 tabs.openView（锁定标签页会新开）。
import { computed, ref, watch } from "vue";
import SectionTitle from "./items/SectionTitle.vue";
import FolderCard from "./cards/FolderCard.vue";
import { dragView } from "../lib/dragOperation";
import { formatSize } from "../lib/formatSize";
import { openContextMenu } from "../lib/contextMenus";
import { openFolderCardMenu } from "../lib/menuSpecs/gridDispatch";
import { wsFolderSpec } from "../lib/menuSpecs/workspace";
import { renameFolder } from "../lib/placementOps";
import { useContentStore, type FolderNode } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { useWsStore } from "../stores/ws";

const props = withDefaults(defineProps<{ viewKey: string; panelActive?: boolean }>(), { panelActive: false });
const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const ws = useWsStore();
const folderOpen = ref(true);
// 工作空间视口（PRD 02，2026-09-17 实机定：文件夹同库口径进网格）：导航 / 右键菜单 /
// 简介行全部分流；库侧专属域（多选选择 / 剪切态 / 排序并入 data-folder-id）不适用——
// 步骤 2 起 ws 文件夹卡片改挂 data-ws-folder（= rel）作素材移动拖拽落点，落点视觉 drop-into 同位
const isWs = computed(() => props.viewKey.startsWith("ws:"));
// 折叠切换：SectionTitle 守卫在调用方（§20——dragView.suppressClick 吞拖拽后紧随 click）
function onFold(): void {
  if (dragView.suppressClick) return;
  folderOpen.value = !folderOpen.value;
}

// 本标签页视口的直接子文件夹（1.2 per-view 参数化）
const stripFolders = computed(() => content.stripFoldersFor(props.viewKey));

// 文件夹卡简介两行（稿值结构恒定）：行1 = 文件数 / 容量；行2 = 档案数 / 子文件夹数（直接子级按 kind 分流）
function folderLines(f: FolderNode): string[][] {
  if (isWs.value) {
    // ws：磁盘实况无容量 / 档案口径——直接文件数 / 直接子文件夹数
    return [[`${f.count.toLocaleString()} 个文件`, `${ws.subDirs(f.id).length.toLocaleString()} 个子文件夹`]];
  }
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
  if (isWs.value) return; // ws：文件夹选择域不接合成 id（写操作归步骤 2）
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
  tabs.openView(isWs.value ? `ws:${id}` : `folder:${id}`);
}

// 右键（6.3；1.3 一轮按 05 稿 320 宽 + 搜索框）：未选中项先单选（与树 item 同规则），全局菜单宿主
// 选择模式（6.2）：不入选择域，直接出菜单（spec 内精简为查看类）
// ws：选择域不适用，直接出 ws 菜单（打开 / 资源管理器定位 / 刷新——无库写操作项）
function onMenu(f: { id: string; name: string }, e: MouseEvent): void {
  if (isWs.value) {
    openContextMenu(e, wsFolderSpec(f.id, f.name), undefined);
    return;
  }
  if (usePickerStore().active) {
    openFolderCardMenu(e, f);
    return;
  }
  if (!selection.isFolderSelected(f.id)) selection.clickFolder(f.id);
  openFolderCardMenu(e, f);
}

// 名称双击进重命名：ws 同样适用（PRD 02 步骤 2，提交走 ws_rename 按路径分支）
function onNameEdit(id: string): void {
  namingId.value = id;
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
  // ws（PRD 02 步骤 2）：f.id = rel，走 ws_rename；库走 renameFolder
  if (isWs.value) {
    try {
      await useWsStore().rename(f.id, name);
    } catch (e) {
      useToastStore().notify({ message: String(e), icon: "fail" });
    }
    return;
  }
  await renameFolder(f.id, name);
}

// 同级排序间隙竖块（2026-08-18 修复"首列前插提示线不可见"：卡片内 left:-12 的竖块会被
// grid-viewport overflow 整条裁掉——改由 FolderStrip 单件 Teleport 到 body 按 client 坐标画；
// 拖拽中 folderGapId 随指针 move 变更 → 本 computed 逐次重取 rect）。
// 2026-09-13 修复"提示线画到窗口左上"：查询限定组件根内 + 非激活 panel 不渲染——
// 原 document.querySelector 全局首匹配会命中隐藏 panel（display:none）的同名卡片 → 零 rect 错位；
// 同日二轮再钳制到所属 grid-viewport 可见矩形（Teleport 无裁剪，滚出视口会画到工具栏上）
const rootRef = ref<HTMLElement>();
const gapBar = computed(() => {
  const id = dragView.folderGapId;
  if (!id || !props.panelActive) return null;
  const el = rootRef.value?.querySelector<HTMLElement>(`.folder-card[data-folder-id="${CSS.escape(id)}"]`);
  const vr = rootRef.value?.closest(".grid-viewport")?.getBoundingClientRect();
  if (!el || !vr) return null;
  const r = el.getBoundingClientRect();
  // 钳制口径同 AssetGrid gapClient（2026-09-13 四轮）：纵向求交（条带随 canvas 滚出视口不画到
  // 工具栏上）；横向不钳——首卡前插落在宿主列表容器 padding-left 12 留白带是设计口径
  const top = Math.max(r.top, vr.top);
  const bottom = Math.min(r.bottom, vr.bottom);
  if (bottom <= top) return null;
  return { left: dragView.folderGapAfter ? r.right : r.left - 12, top, height: bottom - top };
});

</script>

<template>
  <section ref="rootRef" class="folder-block">
    <!-- 区块标题（7.1 内容容器轮 = items/SectionTitle block 贴稿；稿无动作钮 → actions=false）：
         递归显示开关撤出本行（2026-08-30 用户定：查看方式菜单里有就行） -->
    <SectionTitle
      variant="block"
      title="文件夹"
      :count="stripFolders.length"
      :expanded="folderOpen"
      :actions="false"
      @toggle="onFold"
    />
    <div v-if="folderOpen" class="flow">
      <FolderCard
        v-for="f in stripFolders"
        :key="f.id"
        :name="f.name"
        :lines="folderLines(f)"
        class="cell"
        :selected="!isWs && selection.isFolderSelected(f.id)"
        :dimmed="!isWs && (dragView.folderIds?.has(f.id) ?? false)"
        :cut="!isWs && selection.isFolderCut(f.id)"
        :naming="namingId === f.id"
        :drop-into="dragView.folderCardId === (isWs ? `ws:${f.id}` : f.id)"
        :data-folder-id="isWs ? undefined : f.id"
        :data-ws-folder="isWs ? f.id : undefined"
        :data-drag-name="isWs ? undefined : f.name"
        @click="onSelect(f.id, $event)"
        @dblclick="onEnter(f.id)"
        @contextmenu.prevent.stop="onMenu(f, $event)"
        @name-edit="onNameEdit(f.id)"
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
/* 文件夹卡片区块（7.1 内容容器轮）：右 12 对齐网格 PAD.right——卡片右缘距容器 12 不被裁
   （同日修订：PAD.right 只管素材网格布局，strip grid 满宽须自收，旧 padding-right 16 改 12 归位）；
   标题↔卡片 gap 12 */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding-right: var(--n-space-12);
}
/* 卡片列表（稿）：minmax(180, 1fr) 自适应吃满（2026-08-30 用户实机二轮定——换行时余量由本行
   卡片分摊拉伸，不留近卡宽空区；2026-09-13 实机走查 220→180——一排能放四个放三个显太宽，走查待复验）；
   行高 122 定高（122 高链 = padding 24 + 名称 26 + 间距 36 + 简介 36）；列/行间距 12（稿 gap 12） */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  grid-auto-rows: 122px;
  column-gap: var(--n-space-12);
  row-gap: var(--n-space-12);
}
/* 唯一卡片限宽 220 不拉伸（同日用户定：单卡没有排满概念，一张卡被来回拉伸重排很傻；
   2026-09-17 实机走查回宽 180→220——单卡 180 太窄太挤；2 张起 = 上方 minmax(180,1fr) 自适应吃满） */
.flow > .cell:only-child {
  width: 220px;
}
.cell {
  min-width: 0;
}
/* 同级排序间隙竖块（Teleport 到 body，fixed 定位）：宽 12 = 卡片间距档 / 高随目标卡片，
   圆角竖向矩形色块；**品牌色-常态描边 + 品牌色-半透明填充**（2026-08-30 用户实机统一口径，
   旧 --brand-1-30/--brand-1 青绿系随新品牌蓝色 #0F89EC 换代）——原卡片内 left:-12 方案被 viewport
   裁剪废除（2026-08-18，首列前插提示线不可见的根因） */
.folder-drop-gap {
  position: fixed;
  z-index: 5;
  width: 12px;
  border-radius: var(--radius-4);
  background: var(--n-brand-50);
  box-shadow: inset 0 0 0 1px var(--n-brand);
  pointer-events: none;
}
</style>
