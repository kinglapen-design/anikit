<script setup lang="ts">
// 上下文菜单展区——docs/UI 重构/Design Token/组件/上下文菜单/ 五件稿 + 用户容器定案（2026-09-01）。
// AppMenu/ContextMenu 是 fixed 弹出层不便静态展示，预览壳 .pv-menu-shell 复刻容器口径
//（300 宽 / padding 6 / --n-g3 底 / 1px --n-g5 inset 描边 / --n-shadow-sm / 圆角 8），
// 分组 = .pv-menu-group 复刻 ContextMenu .menu-group 口径；item = 真 MenuItem 组件。
import MenuItem from "../../components/MenuItem.vue";
import { FOLDER_SWATCHES } from "../../lib/customColors";
</script>

<template>
  <section id="design-system-menus" class="pv-section">
    <h2 class="pv-sec-title">组件 · 上下文菜单</h2>
    <p class="pv-sec-desc">
      Design Token/组件/上下文菜单/ 五件稿（分割线 / 对勾 / 展开箭头 / 快捷键 / 更改颜色）+ 用户容器定案（2026-09-01）；
      壳为预览复刻（AppMenu 口径），item 为真组件——常态列可直接 hover 交互。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">整菜单形态（容器 300 / 分组 / 组间 1px 五级线）</h3>
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
      <h3 class="pv-group-title">右侧件（快捷键 / 对勾 / 展开箭头 / 尾图标）</h3>
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
            <MenuItem label="名称" checkable checked />
            <MenuItem label="文件大小" checkable />
            <MenuItem label="时长" checkable force-state="hover" />
          </div>
          <span class="pv-cap">对勾：选中恒显一级 / 常态隐 / hover 显三级（第三个 = 强制 hover）</span>
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
  </section>
</template>

<style scoped>
/* 预览壳复刻 AppMenu 容器口径（fixed 弹出层不便静态展示；口径变更时与 AppMenu 同步） */
.pv-menu-shell {
  display: flex;
  flex-direction: column;
  gap: 0;
  width: 300px;
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
