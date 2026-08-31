<script setup lang="ts">
// 右侧面板（7.1 二十三轮贴稿重构，稿 = 布局/主窗口/右侧边栏/）：
// 结构 = 顶行 46（窗口拖拽区；窗口控制组 = App.vue 右上角浮块恒在，不参与收展，2026-08-08 定案不变）
//   → 详情面板 DetailPanel（flex 1：pinned 缩略图/封面区块 + 分段控制器三 tab + tab 内容自管滚动
//     + 底部「信息查看管理」钮——点亮不接业务，排序/显隐管理面板后续轮接，2026-08-31 用户定）。
// 旧 54 工具栏（收起详情面板 + 编辑数据区块）废除：收展入口移中间面板工具栏恒显钮（同日用户定），
// 编辑数据区块入口由底部钮接替（SectionEditPanel 摘挂载，随管理面板轮重做或退役）。
// 选择模式（picker.active）= 详情面板与底部钮整体替换为 PickerPanel 备选区（PRD 03_数据链接 04）。
// 收展 = 常驻挂载 + 宽度由 App.vue rAF 驱动逐帧写入（v6，同 SidebarLeft）。
import DetailPanel from "./DetailPanel.vue";
import PickerPanel from "./PickerPanel.vue";
import { usePickerStore } from "../stores/picker";
import { useUiStore } from "../stores/ui";
import { sidebarShowWidths } from "../lib/useSidebarAnim";

const ui = useUiStore();
const picker = usePickerStore();
</script>

<template>
  <aside
    class="sidebar-right"
    :style="{ width: `${sidebarShowWidths.right}px`, '--sb-w': `${ui.rightWidth}px` }"
  >
    <!-- 顶行 46（稿 窗口控制行）：窗口控制组由 App.vue 右上角浮块呈现，本行仅作窗口拖拽区；
         稿无下边框（旧 border-bottom 暗-悬停废除） -->
    <header class="bar-title" data-tauri-drag-region />
    <!-- 选择模式（6.2 一轮）：详情面板整体替换为备选区（编辑类入口本模式不渲染，
         与"右键仅查看类"同口径） -->
    <PickerPanel v-if="picker.active" />
    <!-- 详情面板（含底部「信息查看管理」钮——文件夹/快速访问形态不渲染该钮，归 DetailPanel 内分流） -->
    <DetailPanel v-else />
  </aside>
</template>

<style scoped>
/* 右侧面板（7.1 十七轮贴稿 布局/主窗口/右侧边栏-总容器）：column，g1 实底 +
   左缘 1px 通用/三级通栏分隔线（稿口径；子块原各自 border-left 收编容器统一）——
   2026-08-31 用户实机修订：底色与分隔线改 ::before 只画顶行以下，顶行 46 透窗口云母；
   宽度可拖（SplitBar；默认 250→264 迁移见 ui store loadSettings）。
   收展 = 常驻挂载 + 宽度由 App.vue rAF 驱动逐帧写入（同 SidebarLeft 的 v6 定案；
   本文件禁挂 width 的 CSS transition）；子块定宽 --sb-w + align-end =
   收窄时内容锚右缘、左缘裁剪（向右滑出观感），不挤压 */
.sidebar-right {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  flex: none;
  min-height: 0;
  overflow: hidden;
  /* 自建层叠上下文：让 ::before 背景层 z-index -1 只在本栏内沉底（不依赖外层上下文状态） */
  isolation: isolate;
}
/* g1 栏底 + 左缘 1px 通用/三级通栏分隔线下移伪元素、只画顶行以下（2026-08-31 用户实机定）：
   顶行 46（窗口控制行）不要底色——透主窗口云母/实色-1，与菜单栏、标签栏同一行连成一片；
   aside 根不再挂 background/border-left（根挂 = 顶行也被盖成 g1 色块）。
   **z-index: -1 必须**（同日实机 BUG：无 z-index 的 absolute 伪元素列于层叠第 6 层 = 定位元素档，
   会盖住正常流内容（ActionButton / cs-slider 等无定位元素「能盲点但看不到」——pointer-events none
   让 hit 穿透所致）；负 z-index + aside isolation 沉到正常流之下） */
.sidebar-right::before {
  content: "";
  position: absolute;
  top: 46px;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: -1;
  background: var(--n-g1);
  border-left: 1px solid var(--n-g3);
  pointer-events: none;
}
/* 顶行 46（稿 窗口控制行）：窗口拖拽区；稿无底色无下边框（透窗口底——云母/实色-1） */
.bar-title {
  width: var(--sb-w); /* 定宽：aside 收窄时裁剪不挤压 */
  height: 46px;
  flex: none;
}
</style>
