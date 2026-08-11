<script setup lang="ts">
// 全局右键菜单渲染器（6.3；1.3 一轮 2026-08-11 升级）：宿主 App.vue 唯一实例，读 lib/contextMenus 的 menuState。
// 渲染复用 AppMenu / MenuItem / MenuDivider（视觉口径不变）；一级级联 flyout——
// hover 带子菜单的项时右侧飞出，右缘溢出改向左飞出（03 稿注"菜单不可超出窗口边缘"）；
// flyout 按 key 随父项重建（2026-08-11 走查：复用实例位置滞留越偏越远）+ pop=false 不回放弹出动画。
// 菜单结构（03/04/05 稿）：容器 padding 0——搜索区（高 44 / padding 12 / 搜索图标 18 盒 svg 15 标签色 /
// 占位符"搜索..." 极淡 / 右侧清除钮 20 圆角 6 / border-bottom 暗-悬停 随容器满宽）与 item 组
//（独立容器 padding 12 / gap 2；divider = 组内自然宽 296，稿值，不通栏，2026-08-11 二轮走查）是两个容器。
// 搜索：打开自动聚焦，输入即过滤——一级+二级项扁平匹配 label（命中二级平铺为一级直接执行），
// 过滤时隐藏 divider 且不出 flyout；清空恢复层级；Esc 先清空、空时再按关闭（AppMenu 豁免输入框 Esc）。
// 组外关闭关系：根菜单的 group = flyout 根元素（flyout 内点击不关根菜单）；
// flyout 自身无 group，点其外部经根菜单统一关闭（closeAll 幂等）。
import { computed, nextTick, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import MenuDivider from "./MenuDivider.vue";
import { closeContextMenu, menuState, type CtxEntry, type CtxItem } from "../lib/contextMenus";

const query = ref("");
const searchInputRef = ref<HTMLInputElement>();

// 级联 flyout：当前展开的子菜单（父项 top 作 key 随项重建）与位置、宽度
const flyout = ref<{ key: number; x: number; y: number; width: number; entries: CtxEntry[] } | null>(null);
const flyoutRootRef = ref<HTMLElement>();

// 每次打开重置搜索并自动聚焦
watch(
  () => menuState.entries,
  async (v) => {
    query.value = "";
    flyout.value = null;
    if (v && menuState.searchable) {
      await nextTick();
      searchInputRef.value?.focus();
    }
  },
);

// 搜索过滤：扁平匹配（大小写不敏感）；命中二级的项平铺为一级展示
const entries = computed<CtxEntry[]>(() => {
  const all = menuState.entries ?? [];
  const q = query.value.trim().toLowerCase();
  if (!q) return all;
  const out: CtxItem[] = [];
  for (const e of all) {
    if (e === "divider") continue;
    if (e.label.toLowerCase().includes(q)) out.push(e.submenu ? { ...e, submenu: undefined } : e);
    else if (e.submenu) {
      for (const s of e.submenu) {
        if (s !== "divider" && s.label.toLowerCase().includes(q)) out.push(s);
      }
    }
  }
  return out;
});

function onItemHover(entry: CtxItem, e: MouseEvent): void {
  if (!entry.submenu || entry.disabled || query.value.trim()) {
    flyout.value = null;
    return;
  }
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const width = entry.submenuWidth ?? 175;
  // 右缘溢出 → 向左飞出（父项左缘 - 二级宽 - 2）
  const x = r.right + 2 + width > window.innerWidth - 4 ? r.left - width - 2 : r.right + 2;
  flyout.value = { key: r.top, x, y: r.top - 8, width, entries: entry.submenu };
}

function onItemClick(entry: CtxEntry): void {
  if (entry === "divider" || entry.disabled || entry.submenu) return;
  entry.action?.();
  closeAll(true);
}

function onSearchKeydown(e: KeyboardEvent): void {
  // Esc：有内容先清空，空则关闭菜单（AppMenu 已豁免输入框 Esc）
  if (e.key === "Escape") {
    e.stopPropagation();
    if (query.value) query.value = "";
    else closeAll(false);
  }
}

function closeAll(executed = false): void {
  flyout.value = null;
  closeContextMenu(executed);
}
</script>

<template>
  <AppMenu
    v-if="menuState.entries"
    :x="menuState.x"
    :y="menuState.y"
    :width="menuState.width ?? 175"
    pad="0"
    gap="0"
    :group="flyoutRootRef ?? null"
    @close="closeAll(false)"
  >
    <!-- 搜索区（03/04/05 稿独立容器）：搜索图标 + 输入框 + 清除钮，border-bottom 通栏分隔 -->
    <div v-if="menuState.searchable" class="menu-search">
      <span class="lead">
        <span class="search-icon"><AppIcon name="search" :size="15" /></span>
        <input
          ref="searchInputRef"
          v-model="query"
          type="text"
          placeholder="搜索..."
          spellcheck="false"
          @keydown="onSearchKeydown"
        />
      </span>
      <button v-if="query" class="clear" @click="query = ''">
        <AppIcon name="clear" :size="12" />
      </button>
    </div>
    <!-- item 组（独立容器 padding 12 / gap 2） -->
    <div class="menu-items">
      <template v-for="(entry, i) in entries" :key="i">
        <MenuDivider v-if="entry === 'divider'" />
        <MenuItem
          v-else
          :label="entry.label"
          :icon="entry.icon"
          :icon-class="entry.iconClass"
          :icon-data="entry.iconData"
          :shortcut="entry.shortcut"
          :disabled="entry.disabled"
          :arrow="!!entry.submenu"
          :checkable="entry.checkable"
          :checked="entry.checked"
          @select="onItemClick(entry)"
          @mouseenter="onItemHover(entry, $event)"
        />
      </template>
      <div v-if="query && entries.length === 0" class="menu-empty">无匹配项</div>
    </div>
  </AppMenu>
  <!-- 级联 flyout：key 随父项重建（防实例复用位置滞留）；pop=false 不回放动画；
       内容包一层 div 供根菜单 group 判定（点击 flyout 内 = 组内） -->
  <AppMenu v-if="flyout && menuState.entries" :key="flyout.key" :x="flyout.x" :y="flyout.y" :width="flyout.width" :pop="false" @close="closeAll(false)">
    <div ref="flyoutRootRef" class="flyout-root">
      <template v-for="(entry, i) in flyout.entries" :key="i">
        <MenuDivider v-if="entry === 'divider'" />
        <MenuItem
          v-else
          :label="entry.label"
          :icon="entry.icon"
          :icon-class="entry.iconClass"
          :icon-data="entry.iconData"
          :shortcut="entry.shortcut"
          :disabled="entry.disabled"
          :checkable="entry.checkable"
          :checked="entry.checked"
          @select="onItemClick(entry)"
        />
      </template>
    </div>
  </AppMenu>
</template>

<style scoped>
.flyout-root {
  display: contents;
}
/* 搜索区（03 稿）：高 44 / padding 12 / gap 6 / 下边框 暗-悬停（容器 padding 0 → 通栏） */
.menu-search {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-6);
  height: 44px;
  padding: 0 var(--space-12);
  flex: none;
  border-bottom: 1px solid var(--line-hover);
}
.menu-search .lead {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
}
/* 搜索图标 18 盒（svg 15），标签色 */
.search-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  flex: none;
  color: var(--text-label);
}
.menu-search input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.menu-search input::placeholder {
  color: var(--text-faint);
}
/* 清除钮 20×20 圆角 6（图标 12，标签色） */
.menu-search .clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-6);
  flex: none;
  color: var(--text-label);
}
.menu-search .clear:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover));
  color: var(--text-normal);
}
/* item 组容器（03 稿）：padding 12 / gap 2 */
.menu-items {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-12);
  min-height: 0;
}
.menu-empty {
  padding: var(--space-6) var(--space-8);
  color: var(--text-faint);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
</style>
