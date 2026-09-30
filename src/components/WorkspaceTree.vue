<script setup lang="ts">
// 工作空间树（PRD 02 + 步骤 3）：常用固定区块 + 文件夹区块 + 回收站行（2026-10-01 用户定，
// 库快速访问回收站同款 TreeRowItem quick + trash，点击 = 资源管理器打开系统回收站——
// ws 删除走系统回收站，PRD 06），区块标题（items/SectionTitle block「常用 / N」「文件夹 / N」）
// 分割——2026-09-17 用户截图定；区块写法与库 LibraryView 完全同口径：段容器内 标题↔内容 gap 8、
// 行 items gap 2、段间距 8（容器 padding 8 12 共享），hover 出展开钮点击折叠区块
//（FolderStrip / ArchiveStrip / LibraryView 区块同先例）。
// 行列表（含递归展开）在 WsTreeRows（根/子层共用）；步骤 3 废除根行「工作空间」（磁盘上无此
// 目录，合成父级与子级同缩进视觉撞兄弟），根视口经切 workspace 分段自动 openView("ws:") 与
// 历史后退可达。workspace/ 目录不存在（exists=false）= 空态文案（正文档 + 亮色/四级，
// SidebarLeft 同风格；root=null 是拉取瞬态不渲染误标；不随区块折叠隐藏）。
import { computed, onMounted, ref } from "vue";
import SectionTitle from "./items/SectionTitle.vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import WsTreeRows from "./WsTreeRows.vue";
import { openContextMenu } from "../lib/contextMenus";
import { wsTreeRowSpec } from "../lib/menuSpecs/workspace";
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

// 懒加载：层级挂载即拉本目录 listing（根层随分段切换挂载；子层随父级展开挂载）；
// 根层另取固定清单（ensureRoot 已拉一次，此处兜底会话内 pin 变化）+ 拉各 pin 目录计数
onMounted(() => {
  void ws.loadDir(props.rel);
  if (isRoot.value) {
    void ws.loadPinned();
    for (const rel of ws.pinned) void ws.loadDir(rel);
  }
});

const subDirs = computed(() => ws.subDirs(props.rel));

// pin 行显示名 = rel 最后一段
const pinName = (rel: string): string => rel.slice(rel.lastIndexOf("/") + 1);

function openDir(rel: string): void {
  tabs.openView(`ws:${rel}`);
}

// pin 行右键（取消固定 / 打开 / 资源管理器，wsTreeRowSpec 按清单状态分流）
function onPinMenu(e: MouseEvent, rel: string): void {
  openContextMenu(e, wsTreeRowSpec(rel), undefined);
}

// 回收站行（2026-10-01 用户定）：点击 = 资源管理器打开系统回收站（ws 删除走系统回收站，PRD 06）
function onOpenRecycleBin(): void {
  void openSystemRecycleBin().catch((e: unknown) => useToastStore().notify({ message: String(e), icon: "fail" }));
}
</script>

<template>
  <div class="ws-tree" :class="{ root: isRoot }">
    <template v-if="isRoot">
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
      <!-- 回收站行（2026-10-01 用户定）：库快速访问回收站同款（TreeRowItem quick + trash 图标）；
           点击 = 资源管理器打开系统回收站；无数量（磁盘实况不跟踪系统回收站内容） -->
      <TreeRowItem kind="quick" icon="trash" name="回收站" @click="onOpenRecycleBin" />
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
