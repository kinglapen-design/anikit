<script setup lang="ts">
// 分组管理弹窗（33 稿，3.5 二轮）：
// AppModal standard 603 宽（600 + 真 border 3 补偿），标题栏 54 / 内容区 padding 0 24 自滚 /
// 底部添加分组（60 高 C4 104×36）+ 按钮栏 62 确认 primary · 取消 panel（88×38 主左次右）。
// 组块 = 组标题行（下移/上移箭头 chevron-small + 「分组名称：」AppInput + delete-red 红删）
//        + GroupRulesEditor（规则行×N + 添加新规则）+ 组块底线；
// 排序 = 箭头 + TransitionGroup 交换动画（速查表 §9 口径，不做拖拽）。
// 草稿式：打开时快照 FolderNode.groups 进本地草稿，确认 → set_folder_groups 一次提交单撤销步；
// 单规则组互斥转移（PRD 02-04 条款 2）：选值时从其他同 kind 单规则组摘除该值 + toast 告知；
// 确认前预检（完全相同 / 空规则 / 空范围）→ 23 稿 alert error（后端同校验兜底）。
import { nextTick, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppInput from "./AppInput.vue";
import IconButton from "./IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import GroupRulesEditor from "./GroupRulesEditor.vue";
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
}
let seq = 0;
const draft = ref<GroupDraft[]>(
  // 递归查（2026-08-19 修复：平铺 find 只命中顶层，子文件夹分组配置读不回）
  (findFolder(content.folders, props.folderId)?.groups ?? []).map((g) => ({
    key: g.id,
    id: g.id,
    name: g.name,
    rules: g.rules.map((r) => ({ kind: r.kind, values: [...r.values] })),
  })),
);
const rowsEl = ref<HTMLElement>();
const busy = ref(false);
// 23 稿错误弹窗（alert error）：标题 + 描述；非 null = 打开
const saveError = ref<{ title: string; desc: string } | null>(null);

// 当前视口内容（规则范围候选：扩展名 / 类型按实际存在出，33 稿注释）
const items = content.placementsFor(`folder:${props.folderId}`);

function addGroup(): void {
  draft.value.push({ key: `new-${++seq}`, id: `g${Date.now().toString(36)}-${seq}`, name: "", rules: [{ kind: "ext", values: [] }] });
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

// 预检（后端同口径兜底）：空规则 / 空范围 / 完全相同（名称不查重，规则集合规范化比较）
function precheck(): string | null {
  for (const g of draft.value) {
    if (g.rules.length === 0) return `分组「${g.name || "未命名分组"}」至少需要一条规则`;
    for (const r of g.rules) {
      if (r.values.length === 0) return `分组「${g.name || "未命名分组"}」存在未选择范围的规则`;
    }
  }
  const norm = (rules: GroupRule[]): string =>
    JSON.stringify(rules.map((r) => [r.kind, [...r.values].sort()] as const).sort());
  for (let i = 0; i < draft.value.length; i++) {
    for (let j = i + 1; j < draft.value.length; j++) {
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
    const groups: GroupDef[] = draft.value.map((g) => ({
      id: g.id,
      name: g.name.trim() || "未命名分组",
      rules: g.rules,
    }));
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
  <AppModal variant="standard" title="分组管理" class="group-manage" @close="emit('close')">
    <div class="gm-body">
      <div class="scroll-wrap">
        <div ref="rowsEl" class="gm-rows kit-scroll">
          <TransitionGroup name="gm-swap" tag="div" class="gm-rows-inner">
            <div v-for="(g, i) in draft" :key="g.key" class="group-block">
              <!-- 组标题行（33 稿：下移 + 上移箭头 / 分组名称输入 / delete-red） -->
              <div class="group-title">
                <IconButton small name="chevron-small" class="sort-arrow arrow-down" :disabled="i === draft.length - 1" title="下移" @click="moveGroup(i, 1)" />
                <IconButton small name="chevron-small" class="sort-arrow" :disabled="i === 0" title="上移" @click="moveGroup(i, -1)" />
                <span class="row-label">分组名称:</span>
                <AppInput v-model="g.name" size="small" class="name-input" placeholder="未命名分组" />
                <IconButton name="delete-red" delete-red title="删除分组" @click="removeGroup(i)" />
              </div>
              <GroupRulesEditor v-model="g.rules" :items="items" @pick-value="onPickValue" />
            </div>
          </TransitionGroup>
        </div>
        <ScrollThumb :body="rowsEl" />
      </div>
      <!-- 添加分组（33 稿：60 高容器，C4 104×36 居中） -->
      <div class="add-wrap">
        <AppButton variant="panel" icon="add-large" label="添加分组" class="add-btn" @click="addGroup" />
      </div>
    </div>
    <template #actions>
      <AppButton variant="primary" label="确认" :disabled="busy" @click="submit" />
      <AppButton variant="panel" label="取消" :disabled="busy" @click="emit('close')" />
    </template>
  </AppModal>
  <!-- 23 稿错误（alert error 48 徽章 + 提示音；主「修改分组」回编辑 / 次「取消」放弃） -->
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
/* 33 稿壳：内容宽 600 + 真 border 3 补偿 = 603（速查表 §1：定宽弹窗写内容宽 +3） */
.group-manage :deep(.app-modal) {
  width: 603px;
  padding: 0;
  gap: 0;
}
.group-manage :deep(.titlebar) {
  height: 54px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-bottom: 1px solid var(--line-hover);
  flex: none;
}
/* 内容区（33 稿）：padding 0 24；
   壳 padding 0：归零 AppModal .content 的负 margin（2026-09-13 实机走查贴右缘口径）——
   padding-right 保留 24（组块靠它内缩），贴右缘由内层 .scroll-wrap/.gm-rows 负 margin 对承担 */
.group-manage :deep(.content) {
  padding: 0 var(--space-24);
  gap: 0;
  overflow: hidden;
  margin-right: 0;
}
.group-manage :deep(.actions) {
  height: 62px;
  box-sizing: border-box;
  align-items: center;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-top: 1px solid var(--line-hover);
  flex: none;
}
.group-manage :deep(.actions) .app-btn {
  width: 88px;
}
.gm-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 滚动包裹层：relative 锚 ScrollThumb；
   负 margin 对（2026-09-13 实机走查贴右缘口径）——margin-right 负扩 24 吃进 .content 的
   0 24 padding 贴到弹窗缘（恰抵 .content 裁剪线内，overflow hidden 不裁），.gm-rows 补回 */
.scroll-wrap {
  position: relative;
  min-height: 0;
  margin-right: calc(-1 * var(--space-24));
}
/* 内容区 414 高自滚（稿值；组块纵向排，组间距 0——组块自带 padding 12 0）；
   padding-right 补回包裹层负扩，组块原位不动（2026-09-13 贴右缘口径） */
.gm-rows {
  display: flex;
  flex-direction: column;
  max-height: 414px;
  padding-right: var(--space-24);
}
.gm-rows-inner {
  display: flex;
  flex-direction: column;
}
.gm-swap-move {
  transition: transform 0.18s ease;
}
/* 组块（33 稿：padding 12 0 / gap 12 / 组底 border-bottom 暗-悬停，末块去线防与添加区叠色） */
.group-block {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding: var(--space-12) 0;
  border-bottom: 1px solid var(--line-hover);
}
.group-block:last-child {
  border-bottom: none;
}
/* 组标题行（33 稿：28 高 / gap 8） */
.group-title {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  height: 28px;
}
.group-title .icon-btn.sort-arrow {
  color: var(--text-highlight);
}
.group-title .icon-btn.sort-arrow:disabled {
  color: var(--line-active);
  opacity: 1;
}
/* chevron-small 源图形朝上——下移旋转 180（BlockManageRow 同口径） */
.arrow-down :deep(svg) {
  transform: rotate(180deg);
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
/* 弹窗内输入框 token = C1（30 稿口径；AppInput 壳化后 class 落壳——底色在壳上覆盖） */
.name-input {
  background: var(--c1);
}
/* 添加分组容器（33 稿：60 高 / 按钮居中 104×36） */
.add-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 60px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-24);
  flex: none;
}
.add-btn {
  width: 104px;
}
.err-desc {
  margin: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
</style>
