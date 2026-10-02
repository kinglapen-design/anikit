<script setup lang="ts">
// 标签栏右侧"添加标签页"按钮（1.2，figma 15：30×30 padding 6，图标同新增 add-large）：
// dockview rightHeaderActionsComponent——每个 group 标签栏各一枚，点击在该 group 新建标签页；
// 落点跟随本组停留视图种类（库 → 全部 / 工作空间 → 根视图，2026-10-03 用户定），决策在 tabs 命令层
import type { IDockviewHeaderActionsProps } from "dockview-vue";
import IconButton from "./buttons/IconButton.vue";
import { useTabsStore } from "../stores/tabs";

// props 运行时声明（rolldown 环境 SFC 编译器无法解析 node_modules 类型）；
// dockview-vue 传参约定：header actions 的 framework props 整组包在 params prop 里（含 group / panels 等）
const raw = defineProps<{ params: Record<string, unknown> }>();
const group = (raw.params as unknown as IDockviewHeaderActionsProps).group;
const tabs = useTabsStore();

function onAdd(): void {
  tabs.addTabAtHome(group.id);
}
</script>

<template>
  <IconButton variant="plain" name="add-large" title="新建标签页" @click="onAdd" />
</template>
