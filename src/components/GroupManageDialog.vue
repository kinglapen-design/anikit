<script setup lang="ts">
// 分组管理弹窗（33 稿，3.5 二轮）：
// 2026-09-20 R2 旧件烧减：壳 = modals/BaseModal framed（603 宽挪 prop；标题栏 54 带底线 +
// 关闭 X 由 framed 提供）；输入框 = inputs/TextInput sm；排序箭头 = buttons/IconButton bare sm
//（禁用 = 新件自带 30% 透明度，旧「--line-active 原样呈现」覆盖废）；删除钮 = clear X bare sm
//（2026-09-20 用户定「红色删除改成 X」，delete-red 变体退役）；按钮 = ActionButton
//（按钮栏 lg 宽自适应 + 左右 48、次左主右；添加钮 solid md——旧 88 / 104 定宽皆废）。
// 组块 = 组标题行（下移/上移箭头 chevron-small + 「分组名称：」输入框 + clear X 删除）
//        + GroupRulesEditor（规则行×N + 添加新规则）+ 组块底线；
// 排序 = 箭头 + TransitionGroup 交换动画（速查表 §9 口径，不做拖拽）。
// 草稿式：打开时快照 FolderNode.groups 进本地草稿，确认 → set_folder_groups 一次提交单撤销步；
// 单规则组互斥转移（PRD 02-04 条款 2）：选值时从其他同 kind 单规则组摘除该值 + toast 告知；
// 确认前预检（完全相同 / 空规则 / 空范围）→ ConfirmModal danger（23 稿错误提示 + 提示音沿；后端同校验兜底）。
import { nextTick, ref, watch } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ConfirmModal from "./modals/ConfirmModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import TextInput from "./inputs/TextInput.vue";
import IconButton from "./buttons/IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import GroupRulesEditor from "./GroupRulesEditor.vue";
import { playAlert } from "../lib/sounds";
import { setFolderGroups } from "../lib/groupOps";
import { RULE_KIND_LABELS, type GroupDef, type GroupRule } from "../lib/grouping";
import { findFolder, useContentStore } from "../stores/content";
import { useToastStore } from "../stores/toast";

const props = defineProps<{ folderId: string }>();
const emit = defineEmits<{ close: [] }>();
const content = useContentStore();
const toast = useToastStore();

// 打开快照（v-if 宿主，setup 即"打开时"）：深拷贝（草稿编辑不污染 store）
interface GroupDraft {
  key: string;
  id: string;
  name: string;
  rules: GroupRule[];
  free: boolean; // 自由组（2026-09-17 无规则空组）：无规则编辑、手动归组；加载 = rules 为空即自由组
  members?: string[]; // 自由组成员快照（保存时原样带回）
}
let seq = 0;
const draft = ref<GroupDraft[]>(
  // 递归查（2026-08-19 修复：平铺 find 只命中顶层，子文件夹分组配置读不回）
  (findFolder(content.folders, props.folderId)?.groups ?? []).map((g) => ({
    key: g.id,
    id: g.id,
    name: g.name,
    rules: g.rules.map((r) => ({ kind: r.kind, values: [...r.values] })),
    free: g.rules.length === 0,
    members: g.members ? [...g.members] : undefined,
  })),
);
const rowsEl = ref<HTMLElement>();
const busy = ref(false);
// 23 稿错误提示（ConfirmModal danger）：标题 + 描述；非 null = 打开
const saveError = ref<{ title: string; desc: string } | null>(null);

// 错误提示音（旧 AppModal alert onMounted 口径沿——弹开即播）
watch(saveError, (v) => {
  if (v) playAlert();
});

// 当前视口内容（规则范围候选：扩展名 / 类型按实际存在出，33 稿注释）
const items = content.placementsFor(`folder:${props.folderId}`);

function addGroup(): void {
  draft.value.push({ key: `new-${++seq}`, id: `g${Date.now().toString(36)}-${seq}`, name: "", rules: [{ kind: "ext", values: [] }], free: false });
  void nextTick(() => {
    const input = rowsEl.value?.querySelector<HTMLElement>(".group-block:last-child input");
    input?.focus();
  });
}
// 添加无规则空组（自由组，2026-09-17 实机走查）：创建即为空、完全无规则、手动归组——
// 内容区恒渲染给「将文件拖入分组」落点；保存时 rules = [] + members 快照带回
function addFreeGroup(): void {
  draft.value.push({ key: `new-${++seq}`, id: `g${Date.now().toString(36)}-${seq}`, name: "", rules: [], free: true, members: [] });
  void nextTick(() => {
    const input = rowsEl.value?.querySelector<HTMLElement>(".group-block:last-child input");
    input?.focus();
  });
}
function moveGroup(index: number, dir: -1 | 1): void {
  const rows = draft.value;
  const j = index + dir;
  if (j < 0 || j >= rows.length) return;
  const [row] = rows.splice(index, 1);
  rows.splice(j, 0, row);
}
function removeGroup(index: number): void {
  draft.value.splice(index, 1);
}

// 单规则组互斥转移（02-04 条款 2）：组二选了组一已占用的扩展名/类型/日期 → 该项自动从组一
// 转移（仅对"单规则组"生效；多规则组合允许重叠）+ toast 告知；确认前取消全部恢复（草稿丢弃）
function onPickValue(rule: GroupRule, value: string): void {
  for (const g of draft.value) {
    if (g.rules.length !== 1) continue;
    const r = g.rules[0];
    if (r === rule || r.kind !== rule.kind) continue;
    const i = r.values.indexOf(value);
    if (i < 0) continue;
    r.values.splice(i, 1);
    toast.notify({
      message: `「${value}」已从「${g.name || "未命名分组"}」转移（${RULE_KIND_LABELS[r.kind]}）`,
      duration: 3000,
    });
  }
}

// 预检（后端同口径兜底）：空范围 / 完全相同（名称不查重，规则集合规范化比较）；
// 自由组（2026-09-17 无规则空组）放开空规则、查重只管规则组
function precheck(): string | null {
  for (const g of draft.value) {
    if (g.free) continue;
    if (g.rules.length === 0) return `分组「${g.name || "未命名分组"}」至少需要一条规则`;
    for (const r of g.rules) {
      if (r.values.length === 0) return `分组「${g.name || "未命名分组"}」存在未选择范围的规则`;
    }
  }
  const norm = (rules: GroupRule[]): string =>
    JSON.stringify(rules.map((r) => [r.kind, [...r.values].sort()] as const).sort());
  for (let i = 0; i < draft.value.length; i++) {
    if (draft.value[i].free) continue;
    for (let j = i + 1; j < draft.value.length; j++) {
      if (draft.value[j].free) continue;
      if (norm(draft.value[i].rules) === norm(draft.value[j].rules)) return "存在完全相同的分组";
    }
  }
  return null;
}

async function submit(): Promise<void> {
  if (busy.value) return;
  const bad = precheck();
  if (bad) {
    saveError.value =
      bad === "存在完全相同的分组"
        ? { title: "无法保存，存在完全相同的分组！", desc: "不能出现两个或多个一模一样的分组，请修改规则和范围，区分不同分组的功能。" }
        : { title: "无法保存，分组规则不完整", desc: `${bad}。请补全或删除后再保存。` };
    return;
  }
  busy.value = true;
  try {
    const groups: GroupDef[] = draft.value.map((g) =>
      g.free
        ? { id: g.id, name: g.name.trim() || "未命名分组", rules: [], members: g.members ?? [] }
        : { id: g.id, name: g.name.trim() || "未命名分组", rules: g.rules },
    );
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
  <BaseModal title="分组管理" :width="603" framed class="group-manage" @close="emit('close')">
    <div class="gm-body">
      <div class="scroll-wrap">
        <div ref="rowsEl" class="gm-rows kit-scroll">
          <TransitionGroup name="gm-swap" tag="div" class="gm-rows-inner">
            <div v-for="(g, i) in draft" :key="g.key" class="group-block">
              <!-- 组标题行（33 稿：下移 + 上移箭头 / 分组名称输入 / clear X 删除） -->
              <div class="group-title">
                <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow arrow-down" :disabled="i === draft.length - 1" title="下移" @click="moveGroup(i, 1)" />
                <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow" :disabled="i === 0" title="上移" @click="moveGroup(i, -1)" />
                <span class="row-label">分组名称:</span>
                <TextInput v-model="g.name" size="sm" class="name-input" placeholder="未命名分组" />
                <IconButton name="clear" variant="bare" size="sm" title="删除分组" @click="removeGroup(i)" />
              </div>
              <!-- 自由组（2026-09-17 无规则空组）：无规则编辑——静态说明行；规则组 = 规则编辑器 -->
              <div v-if="g.free" class="free-hint">无规则（手动归组）——将文件拖入分组，或右键「移动至分组」</div>
              <GroupRulesEditor v-else v-model="g.rules" :items="items" @pick-value="onPickValue" />
            </div>
          </TransitionGroup>
        </div>
        <ScrollThumb :body="rowsEl" />
      </div>
      <!-- 添加分组（33 稿：60 高容器居中）+ 添加空组（自由组，2026-09-17 实机走查） -->
      <div class="add-wrap">
        <ActionButton variant="solid" size="md" icon="add-large" label="添加分组" class="add-btn" @click="addGroup" />
        <ActionButton variant="solid" size="md" icon="add-large" label="添加空组" class="add-btn" @click="addFreeGroup" />
      </div>
    </div>
    <template #actions>
      <ActionButton variant="cancel" label="取消" size="lg" :disabled="busy" @click="emit('close')" />
      <ActionButton variant="primary" label="确认" size="lg" :disabled="busy" @click="submit" />
    </template>
  </BaseModal>
  <!-- 23 稿错误提示（ConfirmModal danger + 提示音；确认「修改分组」回编辑 / 取消放弃） -->
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
/* 33 稿壳：603 宽挪 width prop（旧 = 内容宽 600 + 真 border 3 补偿，速查表 §1）；
   framed padding 0 贴边、gap 归零（相邻区紧贴）；视口封顶沿旧 AppModal 口径（BaseModal 不带） */
.group-manage :deep(.bm-modal) {
  max-height: calc(100vh - 96px);
  gap: 0;
}
/* 底部按钮栏（33 稿：62 高 / padding 12 12 12 24 / 顶线通栏；lg 宽自适应 + 左右 48、次左主右） */
.group-manage :deep(.bm-actions) {
  height: 62px;
  align-items: center;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-top: 1px solid var(--n-g5);
  flex: none;
}
.gm-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 滚动包裹层：relative 锚 ScrollThumb；flex 列让行列表可收缩（视口封顶时自滚）。
   旧 .content padding + 负 margin 对随 AppModal 退役废除——framed 壳 padding 0，
   包裹层天然通栏，thumb 贴弹窗缘 */
.scroll-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 内容区 414 高封顶自滚（稿值；组块纵向排，组间距 0——组块自带 padding 12 0）；
   0 24 内缩自承（framed 壳 padding 0） */
.gm-rows {
  display: flex;
  flex-direction: column;
  min-height: 0;
  max-height: 414px;
  padding: 0 var(--n-space-24);
}
.gm-rows-inner {
  display: flex;
  flex-direction: column;
}
.gm-swap-move {
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
/* 组块（33 稿：padding 12 0 / gap 12 / 组底 1px 通用/五级，末块去线防与添加区叠色） */
.group-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
  border-bottom: 1px solid var(--n-g5);
}
.group-block:last-child {
  border-bottom: none;
}
/* 组标题行（33 稿：28 高 / gap 8） */
.group-title {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 28px;
}
/* 排序箭头：可用态 = 亮色/一级（2026-08-14 用户定高亮档沿，提权压组件自带 .ic-btn.bare）；
   禁用 = 新件自带 30% 透明度 */
.group-title .ic-btn.sort-arrow {
  color: var(--n-l1);
}
/* chevron-small 源图形朝上——下移旋转 180（BlockManageRow 同口径） */
.arrow-down :deep(svg) {
  transform: rotate(180deg);
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
/* 添加分组容器（33 稿：60 高 / 按钮居中；钮 = solid md，旧 104 定宽废——BaseModal 罩宽自适应 + 左右 48） */
.add-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--n-space-12); /* 双钮（添加分组 / 添加空组）间距 */
  height: 60px;
  padding: var(--n-space-12) var(--n-space-24);
  flex: none;
}
/* 自由组静态说明行（2026-09-17 无规则空组）：通用/三级容器与规则行同形，说明文字亮色/三级 */
.free-hint {
  display: flex;
  align-items: center;
  min-height: 44px;
  padding: var(--n-space-8) var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g3);
  box-shadow: inset 0 0 0 1px var(--n-g5);
  color: var(--n-l3);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  flex: none;
}
</style>
