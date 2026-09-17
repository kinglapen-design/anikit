<script setup lang="ts">
// 新建分组弹窗（34 稿，3.5 二轮）：分组菜单「新建分组」触发，单次只添加一个（34 稿注释）。
// AppModal standard 603 宽，标题栏 54「添加分组」；单组编辑块（名称行 无箭头无删除 + 规则编辑器）+
// 按钮栏 62（确认 primary / 取消 panel 88×38 主左次右）。
// 确认 = 追加到该文件夹现有组列表尾部一次提交（set_folder_groups 单撤销步）；
// 与现有组规则完全相同 → 23 稿 alert error（前端预检 + 后端兜底，与分组管理同口径）。
import { ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppInput from "./AppInput.vue";
import GroupRulesEditor from "./GroupRulesEditor.vue";
import { setFolderGroups } from "../lib/groupOps";
import type { GroupDef, GroupRule } from "../lib/grouping";
import { findFolder, useContentStore } from "../stores/content";

const props = defineProps<{ folderId: string }>();
const emit = defineEmits<{ close: [] }>();
const content = useContentStore();

// 新组草稿（单组，无排序/删除件）：默认一条扩展名规则待选范围
const name = ref("");
const rules = ref<GroupRule[]>([{ kind: "ext", values: [] }]);
const busy = ref(false);
const saveError = ref<{ title: string; desc: string } | null>(null);

const items = content.placementsFor(`folder:${props.folderId}`);

async function submit(): Promise<void> {
  if (busy.value) return;
  if (rules.value.length === 0 || rules.value.some((r) => r.values.length === 0)) {
    saveError.value = { title: "无法保存，分组规则不完整", desc: "至少需要一条规则并选择范围。请补全或删除后再保存。" };
    return;
  }
  // 递归查（2026-08-19 修复：平铺 find 只命中顶层——子文件夹上 existing 恒空，
  // 查重失效且提交会把已有组整体覆盖丢光）
  const existing = findFolder(content.folders, props.folderId)?.groups ?? [];
  const norm = (rs: GroupRule[]): string => JSON.stringify(rs.map((r) => [r.kind, [...r.values].sort()] as const).sort());
  const mine = norm(rules.value);
  if (existing.some((g) => norm(g.rules) === mine)) {
    saveError.value = { title: "无法保存，存在完全相同的分组！", desc: "不能出现两个或多个一模一样的分组，请修改规则和范围，区分不同分组的功能。" };
    return;
  }
  busy.value = true;
  try {
    const groups: GroupDef[] = [
      ...existing,
      { id: `g${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`, name: name.value.trim() || "未命名分组", rules: rules.value },
    ];
    await setFolderGroups(props.folderId, groups);
    emit("close");
  } catch (e) {
    saveError.value = { title: "无法保存分组", desc: String(e) };
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <AppModal variant="standard" title="添加分组" class="group-edit" @close="emit('close')">
    <div class="ge-body">
      <div class="group-block">
        <div class="group-title">
          <span class="row-label">分组名称:</span>
          <AppInput v-model="name" size="small" class="name-input" placeholder="未命名分组" />
        </div>
        <GroupRulesEditor v-model="rules" :items="items" />
      </div>
    </div>
    <template #actions>
      <AppButton variant="primary" label="确认" :disabled="busy" @click="submit" />
      <AppButton variant="panel" label="取消" :disabled="busy" @click="emit('close')" />
    </template>
  </AppModal>
  <!-- 23 稿错误（alert error；主「修改分组」回编辑 / 次「取消」放弃） -->
  <AppModal
    v-if="saveError"
    variant="alert"
    alert-icon="error"
    :title="saveError.title"
    @close="saveError = null"
  >
    <p class="err-desc">{{ saveError.desc }}</p>
    <template #actions>
      <AppButton variant="primary" label="修改分组" @click="saveError = null" />
      <AppButton variant="panel" label="取消" @click="emit('close')" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 34 稿壳：内容宽 600 + 真 border 3 补偿 = 603 */
.group-edit :deep(.app-modal) {
  width: 603px;
  padding: 0;
  gap: 0;
}
.group-edit :deep(.titlebar) {
  height: 54px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-bottom: 1px solid var(--line-hover);
  flex: none;
}
/* 内容区（34 稿：padding 12 24；组块 padding 12 0 自带底分隔线）；
   壳 padding 0：归零 AppModal .content 的负 margin（2026-09-13 实机走查贴右缘口径）——
   padding-right 保留 24（组块靠它内缩）；.content 自身即滚动容器，
   AppModal 兜底 ScrollThumb 贴弹窗右缘 */
.group-edit :deep(.content) {
  padding: 0 var(--space-24);
  gap: 0;
  margin-right: 0;
}
.group-edit :deep(.actions) {
  height: 62px;
  box-sizing: border-box;
  align-items: center;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-top: 1px solid var(--line-hover);
  flex: none;
}
.group-edit :deep(.actions) .app-btn {
  width: 88px;
}
.ge-body {
  display: flex;
  flex-direction: column;
}
.group-block {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding: var(--space-12) 0;
  border-bottom: 1px solid var(--line-hover);
}
.group-title {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  height: 28px;
}
.row-label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
  white-space: nowrap;
  flex: none;
}
.name-input {
  flex: 1;
  min-width: 0;
}
.name-input {
  background: var(--c1);
}
.err-desc {
  margin: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
</style>
