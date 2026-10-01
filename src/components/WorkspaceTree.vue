<script setup lang="ts">
// 工作空间树（PRD 02 + 步骤 3）：快速入口容器（2026-10-01 用户定——库快速访问同口径无标题总容器，
// 行 gap 2，位于 常用/文件夹 区块之上，后续入口行直接加进容器；首行 = 回收站，TreeRowItem quick +
// trash 图标，点击 = 资源管理器打开系统回收站——ws 删除走系统回收站，PRD 06）+ 常用固定区块 +
// 文件夹区块，区块标题（items/SectionTitle block「常用 / N」「文件夹 / N」）分割——2026-09-17 用户
// 截图定；区块写法与库 LibraryView 完全同口径：段容器内 标题↔内容 gap 8、行 items gap 2、
// 段间距 8（容器 padding 8 12 共享），hover 出展开钮点击折叠区块（FolderStrip / ArchiveStrip /
// LibraryView 区块同先例）。
// 行列表（含递归展开）在 WsTreeRows（根/子层共用）；步骤 3 废除根行「工作空间」（磁盘上无此
// 目录，合成父级与子级同缩进视觉撞兄弟）——2026-10-02 用户实机报「根目录无法加链接：侧栏无根入口、
// 树空白无菜单」后**根入口行回籍快速入口容器**（入口行 ≠ 合成父级，不撞旧坑：点击 = 根视口
// openView("ws:")，右键 = 根空白菜单 wsBlankSpec("")；名称与根标签标题同 = 「工作空间」，
// content.viewTitle 口径；回收站行居次位——10-01「回收站最上方」是对区块而言，根入口先之）；
// 树容器空白（非 .tree-item 落点）右键同出根空白菜单——根目录写操作（新建 / 添加链接文件夹…）
// 由此有处可触发。workspace/ 目录不存在（exists=false）= 空态文案（正文档 + 亮色/四级，
// SidebarLeft 同风格；root=null 是拉取瞬态不渲染误标；不随区块折叠隐藏）。
import { computed, onMounted, ref } from "vue";
import SectionTitle from "./items/SectionTitle.vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import WsTreeRows from "./WsTreeRows.vue";
import { openContextMenu } from "../lib/contextMenus";
import { wsBlankSpec, wsTreeRowSpec } from "../lib/menuSpecs/workspace";
import { openSystemRecycleBin } from "../lib/wsOps";
import { useContentStore } from "../stores/content";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { useWsStore } from "../stores/ws";

// rel = 本层目录（"" = 根）；depth = 本层内容缩进档（子层经 WsTreeRows 传给递归实例）
const props = withDefaults(defineProps<{ rel?: string; depth?: number }>(), { rel: "", depth: 0 });

const ws = useWsStore();
const content = useContentStore();
const tabs = useTabsStore();
const isRoot = computed(() => props.rel === "");

// 区块折叠（FolderStrip / ArchiveStrip 同先例：区块标题点击折叠，组件级 ref 默认开）
const pinsOpen = ref(true);
const treeOpen = ref(true);

// 懒加载：层级挂载即确保本目录 listing 已加载（根层随分段切换挂载；子层随父级展开挂载）——
// 已加载即跳过（2026-10-01 性能优化③：加载后由 fs-changed 精准重扫维持新鲜，挂载不再全量
// 重读，ContentArea 视口同口径）；根层另取固定清单（ensureRoot 已拉一次，此处兜底会话内
// pin 变化）+ 确保各 pin 目录已加载（计数显示用）
onMounted(() => {
  void ws.ensureDir(props.rel);
  if (isRoot.value) {
    void ws.loadPinned();
    for (const rel of ws.pinned) void ws.ensureDir(rel);
  }
});

const subDirs = computed(() => ws.subDirs(props.rel));

// pin 行显示名 = rel 最后一段
const pinName = (rel: string): string => rel.slice(rel.lastIndexOf("/") + 1);

// pin 行链接门（junction 方向 b，2026-10-02，树行同口径）：挂链 = linked-file-badge；断链 =
// warning-small + dimmed 置灰；联接态存父目录条目列表——父目录未加载（null）按非链接无徽章（降级）
const pinBadge = (rel: string): string => {
  const s = ws.linkState(rel);
  return s?.isLink ? (s.linkOk ? "linked-file-badge" : "warning-small") : "";
};
const pinDimmed = (rel: string): boolean => {
  const s = ws.linkState(rel);
  return !!s?.isLink && !s.linkOk;
};

function openDir(rel: string): void {
  tabs.openView(`ws:${rel}`);
}

// pin 行右键（取消固定 / 打开 / 资源管理器，wsTreeRowSpec 按清单状态分流）；
// spec async（build 前 refreshSystemClip，2026-10-01），@contextmenu.prevent 已同步拦默认
function onPinMenu(e: MouseEvent, rel: string): void {
  void wsTreeRowSpec(rel).then((spec) => openContextMenu(e, spec, undefined));
}

// 回收站行（2026-10-01 用户定）：点击 = 资源管理器打开系统回收站（ws 删除走系统回收站，PRD 06）
function onOpenRecycleBin(): void {
  void openSystemRecycleBin().catch((e: unknown) => useToastStore().notify({ message: String(e), icon: "fail" }));
}

// 根入口行右键 = 根空白菜单（wsBlankSpec("")：新建文件夹 / 添加链接文件夹… / 粘贴 / 资源管理器 /
// 系统菜单 / 刷新——根目录写操作入口，2026-10-02 用户实机报后补）；spec async 同 pin 行口径
function onRootMenu(e: MouseEvent): void {
  void wsBlankSpec("").then((spec) => openContextMenu(e, spec, undefined));
}

// 树容器空白右键（非 .tree-item 落点 = 行与标题之外的空位）= 同根空白菜单；行自身右键已 .stop 拦截；
// 递归子层（v-else 分支共用本容器）不响应——根目录菜单只归根层
function onTreeBlankMenu(e: MouseEvent): void {
  if (!isRoot.value) return;
  if ((e.target as HTMLElement).closest(".tree-item")) return;
  void wsBlankSpec("").then((spec) => openContextMenu(e, spec, undefined));
}
</script>

<template>
  <div class="ws-tree" :class="{ root: isRoot }" @contextmenu.prevent="onTreeBlankMenu">
    <template v-if="isRoot">
      <!-- 快速入口容器（2026-10-01 用户定，库快速访问区块同口径：无区块标题的总容器、
           行 gap 2，位于 常用/文件夹 区块之上——后续入口行（不止回收站）直接加进本容器，
           不动区块嵌套）：首行 = 根入口「工作空间」（2026-10-02 用户实机报后回籍——点击 = 根视口，
           右键 = 根空白菜单 wsBlankSpec("")；数量 = 根目录直接文件数同 pin 行口径）；
           回收站行居次 = 库快速访问回收站同款（TreeRowItem quick + trash 图标），
           点击 = 资源管理器打开系统回收站；无数量（磁盘实况不跟踪系统回收站内容） -->
      <section class="ws-section ws-quick">
        <TreeRowItem
          kind="quick"
          icon="workspace"
          name="工作空间"
          :count="ws.loadedOk('') ? ws.fileCount('') : undefined"
          :selected="content.sidebarSelected === 'ws:'"
          @click="openDir('')"
          @contextmenu.prevent.stop="onRootMenu"
        />
        <TreeRowItem kind="quick" icon="trash" name="回收站" @click="onOpenRecycleBin" />
      </section>
      <!-- 常用固定区块（步骤 3：行 = 目录快捷入口，右键可取消固定；
           数量 = 该目录直接文件数，未加载不显示） -->
      <section v-if="ws.pinned.length" class="ws-section">
        <SectionTitle
          variant="block"
          title="常用"
          :count="ws.pinned.length"
          :expanded="pinsOpen"
          :actions="false"
          @toggle="pinsOpen = !pinsOpen"
        />
        <div v-if="pinsOpen" class="ws-rows">
          <div v-for="rel in ws.pinned" :key="`pin:${rel}`" class="ws-node" @contextmenu.prevent.stop="onPinMenu($event, rel)">
            <TreeRowItem
              kind="folder"
              :name="pinName(rel)"
              :badge="pinBadge(rel)"
              :dimmed="pinDimmed(rel)"
              :count="ws.loadedOk(rel) ? ws.fileCount(rel) : undefined"
              :expandable="false"
              :selected="content.sidebarSelected === `ws:${rel}`"
              :depth="0"
              @click="openDir(rel)"
            />
          </div>
        </div>
      </section>
      <!-- 文件夹区块（标题↔树 gap 8 / 行 gap 2 / 段距 8，LibraryView 树容器同口径） -->
      <section class="ws-section">
        <SectionTitle
          variant="block"
          title="文件夹"
          :count="subDirs.length"
          :expanded="treeOpen"
          :actions="false"
          @toggle="treeOpen = !treeOpen"
        />
        <WsTreeRows v-if="treeOpen" :rel="props.rel" :depth="0" />
      </section>
      <!-- 空态：workspace 目录不存在（exists=false）。root=null 是拉取中 / 未开空间的瞬态
          （ensureRoot null 不缓存、下次调用即重试），不渲染「未开启」误标（2026-09-17 实机报）；
          不随区块折叠隐藏 -->
      <div v-if="ws.root !== null && !ws.root.exists" class="ws-empty">
        工作空间目录不存在
      </div>
    </template>
    <!-- 递归子层：无区块标题，直接行列表 -->
    <WsTreeRows v-else :rel="props.rel" :depth="props.depth" />
  </div>
</template>

<style scoped>
/* 树容器：非根层单个子容器（gap 无意义）；根层 = 段堆叠，段间距 8 + 容器 padding 8 12
   （LibraryView 树容器同口径：padding 由容器统一给，区块不再各自 padding） */
.ws-tree {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
.ws-tree.root {
  padding: var(--n-space-8) var(--n-space-12);
}
/* 区块（LibraryView .block.tree 同口径）：标题↔内容 gap 8 */
.ws-section {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 快速入口容器（库 .block.quick 同口径）：无区块标题，行 gap 2；容器 padding 由 .ws-tree.root 统一给 */
.ws-quick {
  gap: var(--n-space-2);
}
.ws-rows {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
.ws-node {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
/* 空态（未开空间 / 目录不存在）：正文档 + 亮色/四级居中（SidebarLeft .empty-state 同档） */
.ws-empty {
  padding: var(--n-space-12) 0;
  text-align: center;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
  user-select: none;
}
</style>
