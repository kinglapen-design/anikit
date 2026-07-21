<script setup lang="ts">
// 查看方式菜单（03_UI 布局/05_查看方式）：
// 一级：单选框"使用全局设置" + 布局方式 / 排序方式 / 分组依据（hover 展开二级）；
// 二级：对勾项（选中显示对勾，未选中隐藏但占位）。
// 排序 / 分组阶段 1 仅样式占位（阶段 4 接功能）。
import { ref } from "vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import MenuDivider from "./MenuDivider.vue";
import AppCheckbox from "./AppCheckbox.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
defineProps<{ anchor: { x: number; y: number } }>();
const emit = defineEmits<{ close: [] }>();

const sub = ref<"layout" | "sort" | "group" | null>(null);

const layouts = [
  { value: "justified", label: "自适应" },
  { value: "waterfall", label: "瀑布流" },
  { value: "grid", label: "网格" },
  { value: "list", label: "列表" },
] as const;

const sorts = ["标题", "扩展名", "文件大小", "尺寸", "评分", "时长", "添加日期", "创建日期", "修改日期"];

function pickLayout(v: (typeof layouts)[number]["value"]): void {
  ui.layoutMode = v;
  emit("close");
}
</script>

<template>
  <AppMenu :x="anchor.x" :y="anchor.y" @close="emit('close')">
    <div class="check-wrap">
      <AppCheckbox v-model="ui.useGlobalView" label="使用全局设置" />
    </div>
    <MenuDivider />
    <MenuItem icon="view-mode" label="布局方式" arrow @pointerenter="sub = 'layout'" @select="sub = 'layout'" />
    <MenuItem icon="sort-order" label="排序方式" arrow @pointerenter="sub = 'sort'" @select="sub = 'sort'" />
    <MenuItem icon="group-by" label="分组依据" arrow @pointerenter="sub = 'group'" @select="sub = 'group'" />
  </AppMenu>

  <AppMenu v-if="sub === 'layout'" :x="anchor.x + 177" :y="anchor.y + 32" @close="emit('close')">
    <MenuItem
      v-for="l in layouts"
      :key="l.value"
      :label="l.label"
      checkable
      :checked="ui.layoutMode === l.value"
      @select="pickLayout(l.value)"
    />
  </AppMenu>

  <AppMenu v-if="sub === 'sort'" :x="anchor.x + 177" :y="anchor.y + 60" @close="emit('close')">
    <MenuItem
      v-for="s in sorts"
      :key="s"
      :label="s"
      checkable
      :checked="ui.sortBy === s"
      @select="ui.sortBy = s"
    />
    <MenuDivider />
    <MenuItem label="升序" checkable :checked="ui.sortAsc" @select="ui.sortAsc = true" />
    <MenuItem label="降序" checkable :checked="!ui.sortAsc" @select="ui.sortAsc = false" />
  </AppMenu>
</template>

<style scoped>
/* 单选框菜单项：padding 4 6 4 6（05_查看方式） */
.check-wrap {
  padding: var(--space-4) var(--space-6);
}
</style>
