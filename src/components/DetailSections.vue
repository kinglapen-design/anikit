<script setup lang="ts">
// 详情面板壳（7.1 二十三轮三 tab 化重构，稿 = 右侧边栏-详情面板 + 分段控制器）：
// 结构 = pinned 缩略图/封面区块（恒顶，SectionPinned）
//   → 分段控制器（border-top 块 padding 12；icon 型三段：基础信息 basic-info / 链接关系 link-relations /
//     自定义组件 custom-component（2026-10-12 转正：占位期结束，装入提示词 + 逐帧笔记两张真卡片——
//     三 tab 语义分界 = 是什么 / 和谁连着 / 文件级生产批注扩展，PRD 07 扩展点第 1 条同日修订）；
//     buttons/SegmentedControl 滑块 thumb 300ms 形态切换）
//   → tab 内容区（flex 1 自管滚动：kit-scroll + ScrollThumb 悬浮滚轮，速查表 §2）
//   → 底部「信息查看管理」钮（58 高块 padding 12，ActionButton solid md + info-manage 图标；
//     2026-09-26 接业务：开 InfoManageDialog 排序/显隐管理面板，ui.infoManageOpen 宿主 App.vue）。
// tab 口径（2026-09-19 用户改定）：默认「基础信息」；**全局记忆 + 持久化**（ui.detailTab，
// settings.ui 写回通道）——换选中 / 选择模式往返（右栏替换卸载）不重置，链接文件成功后
// 留在当前 tab；旧「会话级记忆、换选中回基础信息、不写设置存储」口径（2026-08-31）废除。
// 旧体系废除：pinned + 7 排序区块堆叠栈 / 注册表双套配置 / 22 稿 SectionEditPanel（摘挂载退役）；
// DetailNotes 笔记区块 2026-10-12 接回换代 = sections/NotesSection（自定义组件 tab 第二张卡片）。
// 形态分流：文件/档案 = pinned + 三 tab + 管理钮；文件夹/快速访问 = FolderDetail；
// 工作空间磁盘实况文件 = WsDetail 简化基础信息（2026-09-26：无 pinned / 分段控制器 / 管理钮，
// 不进 ui.detailBlocks 配置体系，滚动壳与文件夹形态共用）。
import { computed, ref, watch } from "vue";
import SectionPinned from "./sections/SectionPinned.vue";
import TabBasic from "./sections/TabBasic.vue";
import TabLinks from "./sections/TabLinks.vue";
import TabCustom from "./sections/TabCustom.vue";
import FolderDetail from "./sections/FolderDetail.vue";
import WsDetail from "./sections/WsDetail.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import ActionButton from "./buttons/ActionButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import type { DetailTarget } from "../lib/detailSections";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ target: DetailTarget }>();

// 文件 / 档案形态 = 三 tab；文件夹 / 快速访问形态 = FolderDetail（无分段控制器、无信息查看管理钮——稿注）
//（Extract 收窄供子组件类型安全——v-if 分流不参与 TS 收窄）
type TabsTarget = Extract<DetailTarget, { kind: "file" | "archive" }>;
type FolderTarget = Extract<DetailTarget, { kind: "folder" | "quick" }>;
// 工作空间磁盘实况文件形态（2026-09-26）= WsDetail 简化基础信息（同无分段控制器 / 无管理钮，
// 不进 ui.detailBlocks 配置体系；滚动壳与文件夹形态共用）
type WsTarget = Extract<DetailTarget, { kind: "ws" }>;
const tabsTarget = computed<TabsTarget | null>(() =>
  props.target.kind === "file" || props.target.kind === "archive" ? props.target : null,
);
const folderTarget = computed<FolderTarget | null>(() =>
  props.target.kind === "folder" || props.target.kind === "quick" ? props.target : null,
);
const wsTarget = computed<WsTarget | null>(() => (props.target.kind === "ws" ? props.target : null));

// 分段控制器三段（稿序：基础信息 / 链接关系 / 自定义组件——自定义组件 = 占位容器，
// 提示词挪基础信息区块（2026-09-26 提示词二轮）；custom-component 图标用户补源十八批）
const TAB_OPTIONS = [
  { value: "basic", icon: "basic-info" },
  { value: "links", icon: "link-relations" },
  { value: "custom", icon: "custom-component" },
] as const;
// 全局记忆（ui.detailTab 持久化）：卸载重挂 / 换选中不丢当前 tab
const ui = useUiStore();
const tab = computed({
  get: () => ui.detailTab,
  set: (v: string) => {
    ui.detailTab = v as "basic" | "links" | "custom";
  },
});

// tab 切换滑动动画（2026-08-31 用户定：SidebarLeft 分段切换范例同款 + **方向恒定**旧左出 / 新右入——
// 初版按 tab 序双向（fwd/back）被用户走查否掉：「左侧边栏是恒定左出右入，统一一下，不然语义不统一」；
// 时序 = 速查 §20 四调定稿）；切 tab 回顶（全局记忆只记 tab 不记滚动位）
watch(tab, () => {
  if (tabBodyRef.value) tabBodyRef.value.scrollTop = 0;
});

// tab 内容区滚动（kit-scroll 隐藏原生条 + ScrollThumb 悬浮）
const tabBodyRef = ref<HTMLElement>();
</script>

<template>
  <!-- 文件 / 档案形态：pinned + 分段控制器三 tab + 信息查看管理钮 -->
  <div v-if="tabsTarget" class="detail-root">
    <!-- pinned 缩略图 / 档案封面区块（恒顶） -->
    <SectionPinned :target="tabsTarget" />
    <!-- 分段控制器区块（稿：padding 12 / border-top 通用/三级） -->
    <div class="seg-block">
      <SegmentedControl v-model="tab" :options="TAB_OPTIONS" variant="icon" />
    </div>
    <!-- tab 内容区（独立滚动； pinned 与分段控制器不参与）；
         切换滑动动画 = 侧栏分段切换范例同款（速查 §20 时序四调定稿），方向恒定旧左出 / 新右入；
         .tab-pane 包层 = Transition 单子元素要求（TabBasic 等多根组件） -->
    <div class="tab-scroll-wrap">
      <div ref="tabBodyRef" class="tab-body kit-scroll">
        <Transition name="tab-x">
          <div :key="tab" class="tab-pane">
            <TabBasic v-if="tab === 'basic'" :target="tabsTarget" />
            <TabLinks v-else-if="tab === 'links'" :target="tabsTarget" />
            <TabCustom v-else :target="tabsTarget" />
          </div>
        </Transition>
      </div>
      <ScrollThumb :body="tabBodyRef" />
    </div>
    <!-- 信息查看管理（稿：58 高块 padding 12 / 钮 240×34 solid + info-manage）——
         2026-09-26 接业务：开管理弹窗（ui.infoManageOpen，宿主 App.vue） -->
    <footer class="manage-bar">
      <ActionButton label="信息查看管理" icon="info-manage" variant="solid" size="md" @click="ui.infoManageOpen = true" />
    </footer>
  </div>
  <!-- 文件夹 / 快速访问形态（FolderDetail：无分段控制器、无信息查看管理钮——稿注
       「文件夹详情不需要信息查看按钮」；同套滚动壳防内容溢出）；
       工作空间磁盘实况文件形态（WsDetail，2026-09-26）同壳——简化基础信息，无管理钮 -->
  <div v-else-if="folderTarget || wsTarget" class="detail-root">
    <div class="tab-scroll-wrap">
      <div ref="tabBodyRef" class="tab-body kit-scroll">
        <FolderDetail v-if="folderTarget" :target="folderTarget" />
        <WsDetail v-else-if="wsTarget" :target="wsTarget" />
      </div>
      <ScrollThumb :body="tabBodyRef" />
    </div>
  </div>
</template>

<style scoped>
/* 详情面板根（栏内 flex 1；定宽 --sb-w 随栏收窄锚右缘裁剪，同旧子块口径） */
.detail-root {
  display: flex;
  flex-direction: column;
  width: var(--sb-w);
  flex: 1;
  min-height: 0;
}
/* 分段控制器区块（稿：padding 12 / border-top 通用/三级） */
.seg-block {
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
  flex: none;
}
/* tab 内容区滚动壳（relative 供 ScrollThumb 贴右缘） */
.tab-scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.tab-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
}
.tab-pane {
  flex: none;
}
/* tab 切换滑动动画（2026-08-31 用户定，SidebarLeft 分段切换范例同款，速查 §20 时序四调定稿）：
   退场 150ms 缓出（--n-dur-150 + --n-ease-out）先行；入场 250ms 缓入（--n-dur-250 + --n-ease-in）
   delay 100ms 启动（总 350ms，全 token 档）；位移 48px；离场 absolute 不占位。
   **方向恒定 = 旧左出 / 新右入**（与左侧边栏统一——双向版被同日走查否掉：语义不统一） */
.tab-x-enter-active {
  transition:
    transform var(--n-dur-250) var(--n-ease-in) var(--n-dur-100),
    opacity var(--n-dur-250) var(--n-ease-in) var(--n-dur-100);
}
.tab-x-leave-active {
  position: absolute;
  inset: 0;
  transition:
    transform var(--n-dur-150) var(--n-ease-out),
    opacity var(--n-dur-150) var(--n-ease-out);
}
.tab-x-enter-from {
  transform: translateX(48px);
  opacity: 0;
}
.tab-x-leave-to {
  transform: translateX(-48px);
  opacity: 0;
}
/* 信息查看管理按钮块（稿：58 高 = padding 12 + 钮 34；稿无 border-top） */
.manage-bar {
  padding: var(--n-space-12);
  height: 58px;
  box-sizing: border-box;
  flex: none;
}
</style>
