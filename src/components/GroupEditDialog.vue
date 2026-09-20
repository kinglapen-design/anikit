<script setup lang="ts">
// 新建分组弹窗（34 稿，3.5 二轮）：分组菜单「新建分组」触发，单次只添加一个（34 稿注释）。
// 2026-09-20 R2 旧件烧减：壳 = modals/BaseModal framed（603 宽挪 prop；标题栏 54 带底线 +
// 关闭 X 由 framed 提供）；单组编辑块（名称行 无箭头无删除 + 规则编辑器）+ 按钮栏 62
//（ActionButton lg 宽自适应 + 左右 48、次左主右——旧 88 定宽与主左次右序废）；
// 输入框 = inputs/TextInput sm；滚动自管（BaseModal 无内建滚轮，旧靠 AppModal .content 兜底）。
// 确认 = 追加到该文件夹现有组列表尾部一次提交（set_folder_groups 单撤销步）；
// 与现有组规则完全相同 → 23 稿错误提示（ConfirmModal danger + 提示音；前端预检 + 后端兜底，与分组管理同口径）。
import { ref, watch } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ConfirmModal from "./modals/ConfirmModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import TextInput from "./inputs/TextInput.vue";
import ScrollThumb from "./ScrollThumb.vue";
import GroupRulesEditor from "./GroupRulesEditor.vue";
import { playAlert } from "../lib/sounds";
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

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const scrollEl = ref<HTMLElement>();

// 错误提示音（旧 AppModal alert onMounted 口径沿——弹开即播）
watch(saveError, (v) => {
  if (v) playAlert();
});

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
  <BaseModal title="添加分组" :width="603" framed class="group-edit" @close="emit('close')">
    <div class="ge-body">
      <div class="scroll-wrap">
        <div ref="scrollEl" class="ge-scroll kit-scroll">
          <div class="group-block">
            <div class="group-title">
              <span class="row-label">分组名称:</span>
              <TextInput v-model="name" size="sm" class="name-input" placeholder="未命名分组" />
            </div>
            <GroupRulesEditor v-model="rules" :items="items" />
          </div>
        </div>
        <ScrollThumb :body="scrollEl" />
      </div>
    </div>
    <template #actions>
      <ActionButton variant="cancel" label="取消" size="lg" :disabled="busy" @click="emit('close')" />
      <ActionButton variant="primary" label="确认" size="lg" :disabled="busy" @click="submit" />
    </template>
  </BaseModal>
  <!-- 23 稿错误提示（ConfirmModal danger；确认「修改分组」回编辑 / 取消放弃） -->
  <ConfirmModal
    v-if="saveError"
    kind="danger"
    :title="saveError.title"
    confirm-label="修改分组"
    @confirm="saveError = null"
    @cancel="emit('close')"
  >
    {{ saveError.desc }}
  </ConfirmModal>
</template>

<style scoped>
/* 34 稿壳：603 宽挪 width prop（旧 = 内容宽 600 + 真 border 3 补偿）；framed padding 0 贴边、
   gap 归零（相邻区紧贴：标题栏线 / 组块线 / 按钮栏线各自承担）；视口封顶沿旧 AppModal 口径（BaseModal 不带） */
.group-edit :deep(.bm-modal) {
  max-height: calc(100vh - 96px);
  gap: 0;
}
/* 底部按钮栏（34 稿：62 高 / padding 12 12 12 24 / 顶线通栏；lg 宽自适应 + 左右 48、次左主右） */
.group-edit :deep(.bm-actions) {
  height: 62px;
  align-items: center;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-top: 1px solid var(--n-g5);
  flex: none;
}
.ge-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 滚动包裹层：relative 锚 ScrollThumb；flex 列让内容区可收缩（视口封顶时自滚） */
.scroll-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 内容区（34 稿：padding 0 24 内缩自承——framed 壳 padding 0；组块 padding 12 0 自带底分隔线） */
.ge-scroll {
  flex: 1;
  min-height: 0;
  padding: 0 var(--n-space-24);
}
.group-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
  border-bottom: 1px solid var(--n-g5);
}
.group-title {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 28px;
}
.row-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
  flex: none;
}
.name-input {
  flex: 1;
  min-width: 0;
}
</style>
