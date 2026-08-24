<script setup lang="ts">
// 区块 · 关系（6.2 选择模式一轮点亮，figma 03_数据 item §4 关系 item；PRD 03_数据链接 02）：
// item = 称谓框（点击 → C2 底输入框二次修改，稿注）+ 朝左箭头 + 对方文件卡（点击 = **跳转定位**，
// 2026-08-24 用户定——导航到对方所在位置并选中，面包屑后退返回；稿注"进选择模式换绑"废除：
// 要换就删了重新添加）+ ×（hover 从右往左滑入，DetailNotes .del 同款）；
// 添加钮（#tail add-small）= 进入选择模式（PRD 03_数据链接 04：主窗口就地选择不弹窗）。
// 双向显示 = 同一条记录两视角：当前目标是 from 显 label、是 to 显 reverseLabel ?? label
//（relationOps.relationsForTarget）；文件与档案同做（DetailSections 共用，档案侧栏同渲染）。
import { computed, nextTick, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppIcon from "../AppIcon.vue";
import IconButton from "../IconButton.vue";
import { jumpToRelationTarget, relationsForTarget, removeRelation, updateRelation, type PickerTarget, type RelationView } from "../../lib/relationOps";
import { currentLibrary } from "../../lib/libraryOps";
import { useContentStore } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useToastStore } from "../../stores/toast";
import type { DetailTarget } from "../../lib/detailSections";

const props = defineProps<{ target: DetailTarget; title: string }>();
const content = useContentStore();
const picker = usePickerStore();
const toast = useToastStore();
const open = ref(true);

// 当前目标端点（关系匹配键：kind + id）
const targetRef = computed<{ kind: "asset" | "archive"; id: string }>(() =>
  props.target.kind === "file"
    ? { kind: "asset", id: props.target.placement.assetId }
    : { kind: "archive", id: props.target.node.id },
);

// 关系视图清单（relations 在 content store，undo / 增删后自动响应）
const views = computed(() => relationsForTarget(content.relations, currentLibrary.name ?? "", targetRef.value));

// 发起文件（选择模式 origin）：文件 = 当前 Placement；档案 = 当前档案节点（封面快照供备选区左标）
function originOf(): PickerTarget {
  if (props.target.kind === "file") {
    const p = props.target.placement;
    return { kind: "asset", id: p.assetId, name: p.name, ext: p.ext, thumb: p.thumbPath, folderId: p.folderId };
  }
  const n = props.target.node;
  return {
    kind: "archive",
    id: n.id,
    name: n.name,
    thumb: content.archives[n.id]?.coverPath ?? content.folderThumbMap.get(n.id) ?? null,
  };
}

// 添加 = 进入选择模式（空态与列表同钮）
function addRelation(): void {
  picker.enterRelation(originOf());
}

// —— 称谓二次修改（稿注：点击关系变成 C2 底色输入框）——
// 写字段按当前目标端位：目标是 from → 改 label；是 to → 改 reverseLabel（精化反向称谓）
const editingId = ref("");
const draft = ref("");
const editInputRef = ref<HTMLInputElement[]>([]); // v-for 内模板 ref = 数组收集
function startEdit(v: RelationView): void {
  editingId.value = v.relation.id;
  draft.value = v.label;
  void nextTick(() => editInputRef.value[0]?.select());
}
function cancelEdit(): void {
  editingId.value = "";
}
async function commitEdit(v: RelationView): Promise<void> {
  if (editingId.value !== v.relation.id) return; // blur 晚到双触发护栏
  editingId.value = "";
  const next = draft.value.trim();
  if (!next || next === v.label) return;
  try {
    if (v.role === "from") await updateRelation(v.relation.id, next, v.relation.reverseLabel ?? "");
    else await updateRelation(v.relation.id, v.relation.label, next);
    // 就地更新 + 重拉对齐（就地更新即时反馈；重拉整表替换强制视图重算——
    // 探针实证桩共享数组下元素级写入不触发 computed 重算，整表替换最稳）
    const cur = content.relations.find((r) => r.id === v.relation.id);
    if (cur) {
      if (v.role === "from") cur.label = next;
      else cur.reverseLabel = next;
    }
    await content.refreshRelations();
    toast.notify({ message: "已更新关系" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// —— 跳转定位（2026-08-24 用户定：点文件卡 = 跳过去——"光显示在那是没用的"）——
// 实现 = relationOps.jumpToRelationTarget（音频区块共用；openView 入面包屑历史，后退即返回）
function jumpTo(v: RelationView): void {
  jumpToRelationTarget(v.other);
}

// 删除（×，hover 滑入）
async function remove(v: RelationView): Promise<void> {
  try {
    await removeRelation(v.relation.id);
    const i = content.relations.findIndex((r) => r.id === v.relation.id);
    if (i >= 0) content.relations.splice(i, 1);
    toast.notify({ message: "已删除关系" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}
</script>

<template>
  <section class="section-relations detail-block">
    <SectionHeader :title="title" grip v-model:open="open">
      <template #tail>
        <IconButton name="add-small" small title="添加关系" @click="addRelation" />
      </template>
    </SectionHeader>
    <!-- 空数据不渲染容器（空区块展开/折叠零高度变化，2026-08-15 走查口径） -->
    <div v-if="open && views.length > 0" class="body">
      <!-- 关系 item（§4 稿：行 36 高 gap 16 = 称谓框 + 朝左箭头 + 对方文件卡 + ×） -->
      <div v-for="v in views" :key="v.relation.id" class="rel-item">
        <!-- 称谓框（C4；点击进编辑态 = C2 底输入框，稿注） -->
        <div class="label-chip" :class="{ editing: editingId === v.relation.id }" @click="startEdit(v)">
          <input
            v-if="editingId === v.relation.id"
            ref="editInputRef"
            v-model="draft"
            class="label-edit"
            maxlength="20"
            @keydown.enter.prevent="commitEdit(v)"
            @keydown.esc.prevent="cancelEdit"
            @blur="commitEdit(v)"
            @click.stop
          />
          <span v-else class="label-text" v-tip="v.label">{{ v.label }}</span>
        </div>
        <!-- 箭头朝左（对方 → 称谓：对方是我的 label）：chevron-large 朝右（速查表 §6）rotate(180) -->
        <AppIcon name="chevron-large" :size="13" class="arrow" />
        <!-- 对方文件卡（点击 = 跳转定位；档案带 archive 图标，文件名称自带扩展名可辨） -->
        <div class="file-chip" v-tip="{ text: '跳转到文件' }" @click="jumpTo(v)">
          <AppIcon v-if="v.other.kind === 'archive'" name="archive" :size="15" class="deco" />
          <span class="name" v-tip="v.other.name">{{ v.other.name }}</span>
          <IconButton name="clear" small title="删除关系" class="del" @click.stop="remove(v)" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.section-relations {
  display: flex;
  flex-direction: column;
}
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 关系 item（§4 稿：36 高 / gap 16 / radius 8 容器虚位） */
.rel-item {
  display: flex;
  align-items: center;
  gap: var(--space-16);
  height: 36px;
}
/* 称谓框（§4 稿：C4 / padding 8 12 / 圆角 6 / 1px 暗-悬停；宽自适应内容，max 96） */
.label-chip {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  max-width: 96px;
  height: 36px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  cursor: pointer;
}
/* 编辑态（稿注：点击关系变成 C2 底色输入框） */
.label-chip.editing {
  background: var(--c2);
  cursor: text;
}
.label-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.label-edit {
  width: 72px;
  background: transparent;
  border: none;
  outline: none;
  padding: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
/* 箭头 13px 标签档（稿 svg 文字/标签；朝左 = chevron-large rotate 180） */
.arrow {
  flex: none;
  color: var(--text-label);
  transform: rotate(180deg);
}
/* 对方文件卡（§4 稿：C4 / padding 8 12 / 圆角 6 / 1px 暗-悬停 / space-between；点击可换绑） */
.file-chip {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--space-6);
  height: 36px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  cursor: pointer;
}
.file-chip:hover {
  background: var(--c5);
}
.deco {
  flex: none;
  color: var(--text-normal);
}
.name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* ×：常态不占位（宽 0），悬停 item 从右往左滑入（速查表 §9，DetailNotes .del 同款；
   选择器带 .rel-item 提权压 IconButton.small 定宽） */
.rel-item .del {
  width: 0;
  padding-left: 0;
  padding-right: 0;
  overflow: hidden;
  opacity: 0;
  color: var(--text-label);
  flex: none;
  transition:
    width 0.12s ease,
    opacity 0.12s ease;
}
.rel-item:hover .del {
  width: 24px;
  opacity: 1;
}
</style>
