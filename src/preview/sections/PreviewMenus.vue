<script setup lang="ts">
// 上下文菜单展区——docs/UI 重构/Design Token/组件/上下文菜单/ 六件稿 + 用户容器定案（2026-09-01）。
// AppMenu/ContextMenu 是 fixed 弹出层不便静态展示，预览壳 .pv-menu-shell 复刻容器口径
//（宽度内容自适应——最长 item 决定 / padding 6 / --n-g3 底 / 1px --n-g5 inset 描边 / --n-shadow-sm / 圆角 8），
// 分组 = .pv-menu-group 复刻 ContextMenu .menu-group 口径；item = 真 MenuItem 组件（名称→36px→右件）。
import MenuItem from "../../components/MenuItem.vue";
import { FOLDER_SWATCHES } from "../../lib/customColors";
</script>

<template>
  <section id="design-system-menus" class="pv-section">
    <h2 class="pv-sec-title">组件 · 上下文菜单</h2>
    <p class="pv-sec-desc">
      Design Token/组件/上下文菜单/ 六件稿（分割线 / 对勾 / 展开箭头 / 快捷键 / 更改颜色 / 带图标）+ 用户容器定案（2026-09-01）；
      壳为预览复刻（AppMenu 口径），item 为真组件——常态列可直接 hover 交互。
      「打开文件链接所在位置」二级 = linkList 面板（LinkListRows，与卡片徽章 hover 弹窗同构；flyout 用 tip 壳——
      AppMenu tone="tip" + pad 0 行贴边，最小宽 180，2026-09-05 走查定），弹出层形态不陈列。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">整菜单形态（宽度自适应 / 名称→36px→右件 / 分组 / 组间 1px 五级线）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <div class="pv-menu-group">
              <MenuItem label="打开文件夹" shortcut="Enter" />
              <MenuItem label="新标签页打开" shortcut="Ctrl+Enter" />
            </div>
            <div class="pv-menu-group">
              <MenuItem label="排序依据" arrow />
              <MenuItem label="更改颜色" :swatches="{ colors: FOLDER_SWATCHES, current: 'green' }" />
            </div>
            <div class="pv-menu-group">
              <MenuItem label="删除文件夹" shortcut="Delete" />
            </div>
          </div>
          <span class="pv-cap">三组 + 分割线在组盒内底（组间距恒 8）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">item 状态</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <MenuItem label="复制链接" />
          </div>
          <span class="pv-cap">常态（文字 亮色/二级）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <MenuItem label="复制链接" force-state="hover" />
          </div>
          <span class="pv-cap">hover（底 亮色/半透明-3）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <MenuItem label="粘贴文件夹" disabled />
          </div>
          <span class="pv-cap">禁用（亮色/禁用，不响应 hover）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">右侧件（快捷键 / 展开箭头 / 尾图标；对勾 2026-09-05 起在左侧，见下组）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <MenuItem label="打开文件夹" shortcut="Enter" />
            <MenuItem label="新标签页打开" shortcut="Ctrl+Enter" />
          </div>
          <span class="pv-cap">快捷键（Fira Mono 12 三级，原串直显）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <MenuItem label="排序依据" arrow />
            <MenuItem label="上移" trailing-icon="chevron-small" />
            <MenuItem label="下移" trailing-icon="chevron-small" trailing-icon-class="rot-180" />
          </div>
          <span class="pv-cap">展开箭头（三级）/ trailingIcon 尾图标</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">更改颜色 · 内嵌色块行（点色不关菜单，当前色 1.5px 一级外环）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <MenuItem label="更改颜色" :swatches="{ colors: FOLDER_SWATCHES, current: null }" />
          </div>
          <span class="pv-cap">默认位选中（常态-1 灰 = 清色）；hover 色块放大 125% 可直试</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <MenuItem label="更改颜色" :swatches="{ colors: FOLDER_SWATCHES, current: 'cyan' }" />
          </div>
          <span class="pv-cap">青色选中（九色 -1 档按 token 声明序）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">左侧标记槽 + 占位对齐（对勾稿 / 带图标稿，2026-09-05 走查）</h3>
      <p class="pv-sec-desc">
        对勾在名称<strong>左侧</strong>（对勾稿本意；与快捷键可同存，同 Chrome 原生样式）；图标 18 盒内 14（05_文档图标品牌彩色原值）。
        同菜单内有任一项带对勾/图标 → 无标记项左侧补 18 占位盒（markPad），名称纵向对齐。
        真实菜单里占位由 ContextMenu scanMark 自动扫描，此处手动陈列。
      </p>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <div class="pv-menu-group">
              <MenuItem label="Animate 文档" icon="animate" />
              <MenuItem label="Photoshop 文档" icon="photoshop" />
              <MenuItem label="Illustrator 文档" icon="illustrator" />
            </div>
            <div class="pv-menu-group">
              <MenuItem label="MarkDown 文档" icon="markdown" />
              <MenuItem label="文本文档" icon="txt" />
              <MenuItem label="Word 文档" icon="word" />
              <MenuItem label="Excel 文档" icon="excel" />
            </div>
          </div>
          <span class="pv-cap">新建文档七项（addMenu.newDocGroupEntries 实接）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <div class="pv-menu-group">
              <MenuItem label="打开预览" mark-pad shortcut="Enter" />
              <MenuItem label="打开文件链接所在位置" mark-pad arrow />
            </div>
            <div class="pv-menu-group">
              <MenuItem label="MarkDown 文档" icon="markdown" />
              <MenuItem label="文本文档" icon="txt" />
            </div>
          </div>
          <span class="pv-cap">混排：无图标项 mark-pad 左占位，名称对齐</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <div class="pv-menu-group">
              <MenuItem label="自定义" checkable checked />
              <MenuItem label="当前层级 - 标题 A → Z" checkable />
              <MenuItem label="全部 - 标题 A → Z" checkable force-state="hover" />
            </div>
          </div>
          <span class="pv-cap">对勾左侧：选中恒显一级 / 常态隐 / hover 显三级</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-menu-shell">
            <div class="pv-menu-group">
              <MenuItem label="自定义" checkable checked shortcut="Alt+1" />
              <MenuItem label="文件大小" mark-pad shortcut="Alt+2" />
            </div>
          </div>
          <span class="pv-cap">对勾与快捷键同存（左勾右键）+ 占位对齐</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 预览壳复刻 AppMenu 容器口径（fixed 弹出层不便静态展示；口径变更时与 AppMenu 同步） */
.pv-menu-shell {
  display: flex;
  flex-direction: column;
  gap: 0;
  width: fit-content; /* 内容自适应（最长 item 决定，2026-09-01 二十五轮废 300 定宽） */
  box-sizing: border-box;
  padding: var(--n-space-6);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow:
    inset 0 0 0 1px var(--n-g5),
    var(--n-shadow-sm);
}
/* 分组复刻 ContextMenu .menu-group 口径 */
.pv-menu-group {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  padding: var(--n-space-4) 0;
}
.pv-menu-group:first-child {
  padding-top: 0;
}
.pv-menu-group:last-child {
  padding-bottom: 0;
}
.pv-menu-group:not(:last-child) {
  box-shadow: inset 0 -1px 0 var(--n-g5);
}
</style>
