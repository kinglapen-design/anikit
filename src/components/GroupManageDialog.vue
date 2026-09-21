<script setup lang="ts">
// 分组管理弹窗（稿 = docs/06_UI重构/Design Token/布局/工具栏展开窗口/分组管理.md + 同目录
// 02_样式标注 figma CSS，2026-09-20 R4 未落地稿补齐——内脏按标注稿重写，旧归档 33 稿口径全废）：
// 壳 = modals/BaseModal framed（603 宽沿 R2 定案，稿 640 未随）；标题栏 54 带底线 + 关闭 X 由 framed 提供。
// 组卡片（稿「分组item」）：通用/六级底 + 立体感-通用 + 圆角 8 + padding 12，卡片间 gap 12（旧组块组底线废）；
// 组标题行（稿「输入分组名称」42 高 / gap 8 / 底线 亮色/禁用）：上移 + 下移箭头（稿序 上→下；
//   chevron-small bare sm，可用 = 亮色/二级——旧 2026-08-14 高亮一级档随旧稿废，禁用 = 控件族 30% 透明度）
//   + 名称输入框（TextInput sm 衍生：30 高 / 通用/四级底 / 左对齐 / 文字亮色/一级——sm 半透明-3 描边
//   与圆角 8 已合稿，稿无「分组名称:」文字标签）+ clear X 删除（bare sm 亮色/三级）；
// 规则区 = GroupRulesEditor（规则行×N + 添加新规则——编辑器内部版式不在本稿落地范围，稿差记台账）；
// 底栏（稿「按钮」62 高 / padding 12 四边 / space-between / 稿无顶线——旧 33 稿顶线通栏废）：
//   左 = 添加分组（+ 添加空组，稿外件 2026-09-17 实机走查增）ActionButton cancel lg——稿 通用/五级底
//   padding 8 48 与 BaseModal 按钮罩同值、稿无图标；右 = 取消 + 确认（lg 宽自适应、次左主右）；
// 动画（稿 prose 点名两件）：新建分组卡片从下至上入场（enter 淡入 + 24 上移，--n-dur-250 小位移档）
//   + 排序 TransitionGroup 交换动画（150 通用档沿速查 §9，不做拖拽）。
// 草稿式：打开时快照 FolderNode.groups 进本地草稿，确认 → set_folder_groups 一次提交单撤销步；
// 单规则组互斥转移（PRD 02-04 条款 2）：选值时从其他同 kind 单规则组摘除该值 + toast 告知；
// 确认前预检（完全相同 / 空规则 / 空范围）→ ConfirmModal danger（错误提示 + 提示音沿；后端同校验兜底）。
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
              <!-- 组标题行（稿：上移 + 下移箭头 / 名称输入框 / clear X 删除，底线 亮色/禁用） -->
              <div class="group-title">
                <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow" :disabled="i === 0" title="上移" @click="moveGroup(i, -1)" />
                <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow arrow-down" :disabled="i === draft.length - 1" title="下移" @click="moveGroup(i, 1)" />
                <TextInput v-model="g.name" size="sm" class="name-input" placeholder="未命名分组" />
                <IconButton name="clear" variant="bare" size="sm" title="删除分组" @click="removeGroup(i)" />
              </div>
              <!-- 规则区（稿「规则列表」padding-top 12 自承）：自由组 = 静态说明行；规则组 = 规则编辑器 -->
              <div class="group-content">
                <div v-if="g.free" class="free-hint">无规则（手动归组）——将文件拖入分组，或右键「移动至分组」</div>
                <GroupRulesEditor v-else v-model="g.rules" :items="items" @pick-value="onPickValue" />
              </div>
            </div>
          </TransitionGroup>
        </div>
        <ScrollThumb :body="rowsEl" />
      </div>
    </div>
    <!-- 底栏（稿「按钮」space-between）：左 = 添加分组/添加空组，右 = 取消 + 确认（次左主右） -->
    <template #actions>
      <div class="gm-actions-left">
        <ActionButton variant="cancel" size="lg" label="添加分组" @click="addGroup" />
        <ActionButton variant="cancel" size="lg" label="添加空组" @click="addFreeGroup" />
      </div>
      <div class="gm-actions-right">
        <ActionButton variant="cancel" label="取消" size="lg" :disabled="busy" @click="emit('close')" />
        <ActionButton variant="primary" label="确认" size="lg" :disabled="busy" @click="submit" />
      </div>
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
/* 壳：603 宽沿 R2（width prop；稿 640 未随），framed padding 0 贴边、gap 归零（相邻区紧贴）；
   视口封顶沿旧 AppModal 口径（BaseModal 不带） */
.group-manage :deep(.bm-modal) {
  max-height: calc(100vh - 96px);
  gap: 0;
}
/* 底栏（稿「按钮」：62 高 / padding 12 四边 / space-between——左添加右确认；稿无顶线，旧 33 稿顶线通栏废） */
.group-manage :deep(.bm-actions) {
  height: 62px;
  align-items: center;
  justify-content: space-between;
  padding: var(--n-space-12);
  flex: none;
}
/* 底栏左右组（稿 gap 8）：左 = 添加分组/添加空组（cancel lg = 稿 通用/五级底，padding 8 48 由
   BaseModal 按钮罩统一给）；右 = 取消 + 确认 */
.gm-actions-left,
.gm-actions-right {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
}
.gm-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 滚动包裹层：relative 锚 ScrollThumb；flex 列让行列表可收缩（视口封顶时自滚）。
   包裹层天然通栏（framed 壳 padding 0），thumb 贴弹窗缘 */
.scroll-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
/* 分组列表（稿：714 高 = 窗 830 - 标题栏 54 - 底栏 62 / padding 12 24；超出自滚） */
.gm-rows {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 714px;
  padding: var(--n-space-12) var(--n-space-24);
}
/* 卡片列（稿 分组列表 gap 12） */
.gm-rows-inner {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
/* 排序交换动画（稿 prose「排序时要有交换动画」；速查 §9 口径沿：150 通用档，不做拖拽） */
.gm-swap-move {
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
/* 新建分组卡片入场（稿 prose「新的分组卡片从下至上」）：淡入 + 24 上移，250 小位移档 */
.gm-swap-enter-from {
  opacity: 0;
  transform: translateY(var(--n-space-24));
}
.gm-swap-enter-active {
  transition:
    opacity var(--n-dur-250) var(--n-ease-in-out),
    transform var(--n-dur-250) var(--n-ease-in-out);
}
/* 组卡片（稿「分组item」：通用/六级底 / 立体感-通用 / 圆角 8 / padding 12；旧组块组底线废——
   稿组间靠卡片间隔 12，无线） */
.group-block {
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12);
  border-radius: var(--n-radius-8);
  background: var(--n-g6);
  box-shadow: var(--n-effect-raised);
}
/* 组标题行（稿「输入分组名称」：gap 8 / padding-bottom 12 / 底线 亮色/禁用；
   行高由 30 输入框撑起 = 稿 42） */
.group-title {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  padding-bottom: var(--n-space-12);
  border-bottom: 1px solid var(--n-l-disabled);
}
/* 排序箭头（稿：可用 = 亮色/二级——旧 2026-08-14 高亮一级档随旧稿废，提权压组件自带 .ic-btn.bare；
   禁用 = 控件族 30% 透明度） */
.group-title .ic-btn.sort-arrow {
  color: var(--n-l2);
}
/* chevron-small 源图形朝上——下移旋转 180（BlockManageRow 同口径） */
.arrow-down :deep(svg) {
  transform: rotate(180deg);
}
.name-input {
  flex: 1;
  min-width: 0;
}
/* 名称输入框衍生（稿：30 高 / padding 6 12 / 通用/四级底 / 文字亮色/一级；
   基件 TextInput sm 的 半透明-3 描边与圆角 8 已合稿——提权压 .ti-root.sm；
   左对齐 2026-09-21 起归基件统一口径，此件不再覆写） */
.group-title .ti-root.name-input {
  height: 30px;
  padding: var(--n-space-6) var(--n-space-12);
  background: var(--n-g4);
}
.name-input :deep(.ti-inner) {
  color: var(--n-l1);
}
/* 规则区（稿「规则列表」padding-top 12：标题行底线与首条规则间隔 12 自承） */
.group-content {
  padding-top: var(--n-space-12);
}
/* 自由组静态说明行（2026-09-17 无规则空组，稿外件）：通用/三级容器与规则行同形，说明文字亮色/三级 */
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
