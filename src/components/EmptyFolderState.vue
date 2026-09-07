<script setup lang="ts">
// 空文件夹空态——docs/06_UI重构/Design Token/布局/主窗口/中间面板/中间面板-内容容器-空文件夹.md：
// 列表容器中央 = 空文件夹插画（200×181.4，opacity 0.6 + 小阴影 drop-shadow 0 4 20 40%）
// + 标题「暂无文件」（标题档 二级）+ 提示（正文 三级）+ 按钮组（gap 8：
// 导入文件 / 导入文件夹 = ActionButton solid md + 前置图标 18，各 137 宽稿值）；
// 块 padding-bottom 24（稿「图标+提示+按钮」）。
// 业务 = 既有导入口径（menuOps.pickImportPaths：导入文件 → 当前视口目录 / 导入文件夹 → 根目录）。
// 根层 pointer-events none 穿透（不挡空白右键 / 框选 / 拖入导入），仅按钮可点。
import AppIcon from "./AppIcon.vue";
import ActionButton from "./buttons/ActionButton.vue";
import { pickImportPaths } from "../lib/menuOps";
</script>

<template>
  <div class="ef-root">
    <div class="ef-body">
      <AppIcon name="folder-empty" :size="200" class="ef-icon" />
      <div class="ef-texts">
        <span class="ef-title">暂无文件</span>
        <span class="ef-hint">将文件拖拽至此处导入</span>
      </div>
      <div class="ef-actions">
        <ActionButton label="导入文件" icon="import-file" @click="pickImportPaths('files')" />
        <ActionButton label="导入文件夹" icon="import-folder" @click="pickImportPaths('folder')" />
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 根：铺满列表容器，内容居中；pointer-events none 穿透（按钮恢复 auto） */
.ef-root {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}
/* 图标+提示+按钮块：column 居中 gap 24，padding-bottom 24（稿） */
.ef-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-24);
  padding-bottom: var(--n-space-24);
}
/* 空文件夹插画：opacity 0.6 由 CSS 管（2026-08-30 用户定：SVG 节点不透明导出）——
   filter 先于 opacity 应用：drop-shadow 以不透明本体为源、被本体挡住不透出（SVG 内嵌
   0.6 时阴影透过半透明本体 = "中间深色方块"根因），随后图标+投影整体乘 0.6 = 稿观感；
   外阴影 = filter drop-shadow（figma 效果不随 SVG 导出，稿「小阴影」在此补） */
.ef-icon {
  opacity: 0.6;
  filter: drop-shadow(0 4px 20px rgba(0, 0, 0, 0.4));
}
/* 标题+提示：column 居中 gap 8 */
.ef-texts {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-8);
}
.ef-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.ef-hint {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
/* 按钮组：grid 双列均分各 137（稿值）+ gap 8、宽 282（稿值）——ActionButton md/lg 宽 100%
   由父容器定格（稿 flex-grow 拉伸语境）；flex 行内 width:100% + 组件 flex:none 不收缩会
   双双撑满 282 向右溢出，视觉组中心偏右 = 2026-08-30 用户实机"空态没居中"根因，故用 grid 定格 */
.ef-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--n-space-8);
  width: 282px;
  pointer-events: auto;
}
</style>
