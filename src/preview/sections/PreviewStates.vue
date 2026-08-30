<script setup lang="ts">
// 组件 空态展区——docs/UI 重构/Design Token/布局/主窗口/中间面板/中间面板-内容容器-空文件夹.md 的展示与验收面。
// EmptyFolderState 实机根层 pointer-events none 穿透（不挡空白右键 / 框选 / 拖入导入），仅按钮可点；
// 按钮接真实业务（menuOps.pickImportPaths 系统导入对话框）——预览页纯展示口径：演示舞台整层禁指针（2026-08-30 内容容器轮）。
import EmptyFolderState from "../../components/EmptyFolderState.vue";
</script>

<template>
  <section id="design-system-states" class="pv-section">
    <h2 class="pv-sec-title">组件 · 空态</h2>
    <p class="pv-sec-desc">
      布局/主窗口/中间面板/中间面板-内容容器-空文件夹 稿：空文件夹插画 200（opacity 0.6 + drop-shadow 0 4 20 40%）+
      标题「暂无文件」（标题档 二级）+ 提示「将文件拖拽至此处导入」（正文 三级）+ 导入文件 / 导入文件夹双钮
      （ActionButton solid md + 前置图标 18，各 137 稿值，gap 8）；块 padding-bottom 24。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">空文件夹空态（EmptyFolderState 铺满列表容器居中；按钮实机可点，演示舞台禁指针纯展示）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-ef-stage">
            <EmptyFolderState />
          </div>
          <span class="pv-cap">舞台 = 列表容器示意（720×420）；按钮接真实导入业务（系统对话框），此处不响应</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 空态演示舞台：relative 供 EmptyFolderState 绝对铺满；整层禁指针（按钮接真实业务，预览纯展示） */
.pv-ef-stage {
  position: relative;
  width: 720px;
  height: 420px;
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  pointer-events: none;
}
</style>
