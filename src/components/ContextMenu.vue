<script setup lang="ts">
// 全局右键菜单渲染器（6.3）：宿主 App.vue 唯一实例，读 lib/contextMenus 的 menuState。
// 渲染复用 AppMenu / MenuItem / MenuDivider（视觉口径不变）；新增一级级联子菜单——
// hover 带 submenu 的项时右侧飞出（原 AppMenu 不支持级联，2026-07-29）。
// 组外关闭关系：根菜单的 group = flyout 根元素（flyout 内点击不关根菜单）；
// flyout 自身无 group，点其外部经根菜单统一关闭（closeAll 幂等）。
import { computed, ref } from "vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import MenuDivider from "./MenuDivider.vue";
import { closeContextMenu, menuState, type CtxEntry } from "../lib/contextMenus";

const entries = computed(() => menuState.entries ?? []);

// 级联 flyout：当前展开的子菜单（项 label 作 key）与位置
const flyout = ref<{ key: string; x: number; y: number; entries: CtxEntry[] } | null>(null);
const flyoutRootRef = ref<HTMLElement>();

function onItemHover(entries_: CtxEntry[] | undefined, e: MouseEvent): void {
  if (!entries_) {
    flyout.value = null;
    return;
  }
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  flyout.value = { key: r.top, x: r.right + 2, y: r.top - 8, entries: entries_ };
}

function onItemClick(entry: CtxEntry): void {
  if (entry === "divider" || entry.disabled || entry.submenu) return;
  entry.action?.();
  closeAll(true);
}

function closeAll(executed = false): void {
  flyout.value = null;
  closeContextMenu(executed);
}
</script>

<template>
  <AppMenu v-if="menuState.entries" :x="menuState.x" :y="menuState.y" :group="flyoutRootRef ?? null" @close="closeAll(false)">
    <template v-for="(entry, i) in entries" :key="i">
      <MenuDivider v-if="entry === 'divider'" />
      <MenuItem
        v-else
        :label="entry.label"
        :icon="entry.icon"
        :icon-class="entry.iconClass"
        :shortcut="entry.shortcut"
        :disabled="entry.disabled"
        :arrow="!!entry.submenu"
        :checkable="entry.checkable"
        :checked="entry.checked"
        @select="onItemClick(entry)"
        @mouseenter="onItemHover(entry.submenu, $event)"
      />
    </template>
  </AppMenu>
  <!-- 级联 flyout：内容包一层 div 供根菜单 group 判定（点击 flyout 内 = 组内） -->
  <AppMenu v-if="flyout && menuState.entries" :x="flyout.x" :y="flyout.y" @close="closeAll(false)">
    <div ref="flyoutRootRef" class="flyout-root">
      <template v-for="(entry, i) in flyout.entries" :key="i">
        <MenuDivider v-if="entry === 'divider'" />
        <MenuItem
          v-else
          :label="entry.label"
          :icon="entry.icon"
          :icon-class="entry.iconClass"
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
</style>
