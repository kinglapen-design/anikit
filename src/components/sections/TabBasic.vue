<script setup lang="ts">
// 基础信息 tab（7.1 二十三轮贴稿，稿 = 右侧边栏-详情面板-基础信息 + 输入框/标签/详情/基本数据 四子稿）：
// 区块链（全带 border-top 通用/三级，padding 12）：
//   输入框区块（gap 8）：名称 TextInput lg 只读 + 链接 TextInput lg 只读（占位 http:// + 右置 link 图标）——
//     名称/链接输入框自旧 pinned 区块移入（旧 36 高 C2 口径作废），只读业务口径不变（一轮只读沿用）；
//   标签区块（gap 12，垂直居中）：属性 chips（高 25 / 通用四级底 + 通用六级 1px / 圆角 6 / 次要文字 12/16 二级
//     ——figma「标签」档 12/17 未收录 Token.md，2026-08-31 用户定就近用档）
//     + add-small 添加钮（6.1 业务：开 32 稿属性弹窗 ui.propsEdit）；
//   详情区块（gap 12）：描述条目（标题行 = 正文-加粗 四级 + expand 折叠钮；正文 13/18 二级，
//     超 3 行截断 + 展开/收起小字钮 6.1 口径保留）+ 编辑描述 ActionButton（6.1 业务：ui.descEdit）；
//   基本数据区块（gap 12）：BaseDataItem 信息卡（文件 = 尺寸/文件大小/格式/添加日期/修改日期——
//     创建日期 metadata 无字段不加（旧口径沿）；档案 = 文件数/占用空间/创建日期）。
// 文件与档案同构（2026-08-31 用户定）：档案 = props / descriptions / 统计数据渲染同一结构。
import { computed, nextTick, onMounted, ref, watch } from "vue";
import TextInput from "../inputs/TextInput.vue";
import ActionButton from "../buttons/ActionButton.vue";
import IconButton from "../buttons/IconButton.vue";
import BaseDataItem from "../items/BaseDataItem.vue";
import AppIcon from "../AppIcon.vue";
import { displayName } from "../../lib/formatBadge";
import { formatDetailDate, type DetailTarget } from "../../lib/detailSections";
import { formatSize } from "../../lib/formatSize";
import { useUiStore } from "../../stores/ui";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
const ui = useUiStore();

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));

// 名称 / 链接（只读展示；链接 = detail.links / archives.json links 首条，空 = placeholder http://）
const name = computed(() => {
  if (file.value) return displayName(file.value.placement, ui.showExt);
  return archive.value?.node.name ?? "";
});
const firstLink = computed(() => {
  if (file.value) return file.value.detail.links[0] ?? "";
  return archive.value?.data.links[0] ?? "";
});

// 属性（文件 = tags 语义属性词；档案 = props）与描述（两侧统一标题 + 内容列表）——SectionCustom 旧业务
const propWords = computed(() => (file.value ? file.value.detail.tags : (archive.value?.data.props ?? [])));
const descriptions = computed(() =>
  file.value ? file.value.detail.descriptions : (archive.value?.data.descriptions ?? []),
);

// —— 描述折叠（稿：标题行 expand 钮折叠正文，默认展开）+ 3 行截断（6.1 口径：超 3 行出展开/收起小字钮）——
const bodyCollapsed = ref<Set<number>>(new Set());
function toggleBody(i: number): void {
  const next = new Set(bodyCollapsed.value);
  if (next.has(i)) next.delete(i);
  else next.add(i);
  bodyCollapsed.value = next;
}
const descEls = ref<HTMLElement[]>([]);
const needsClamp = ref<boolean[]>([]);
const expanded = ref<Set<number>>(new Set());
async function measure(): Promise<void> {
  await nextTick();
  needsClamp.value = descriptions.value.map((_, i) => {
    if (expanded.value.has(i)) return needsClamp.value[i] ?? false; // 展开态保持判定（展开后量不出截断）
    const el = descEls.value[i];
    return el ? el.scrollHeight > el.clientHeight + 1 : false;
  });
}
onMounted(measure);
watch(descriptions, () => {
  expanded.value = new Set();
  bodyCollapsed.value = new Set();
  needsClamp.value = [];
  void measure();
});
function toggleDesc(i: number): void {
  const next = new Set(expanded.value);
  if (next.has(i)) next.delete(i);
  else next.add(i);
  expanded.value = next;
}

// —— 编辑入口（6.1）：快照目标 + 当前值，弹窗宿主 App.vue ——
function openPropsDialog(): void {
  const t = props.target;
  ui.propsEdit =
    t.kind === "file"
      ? { kind: "file", id: t.detail.assetId, name: t.placement.name, initial: [...t.detail.tags] }
      : { kind: "archive", id: t.node.id, name: t.node.name, initial: [...t.data.props] };
}
function openDescDialog(): void {
  const t = props.target;
  ui.descEdit =
    t.kind === "file"
      ? { kind: "file", id: t.detail.assetId, name: t.placement.name, initial: t.detail.descriptions.map((d) => ({ ...d })) }
      : { kind: "archive", id: t.node.id, name: t.node.name, initial: t.data.descriptions.map((d) => ({ ...d })) };
}

// 基本数据行（BaseDataItem rows 形态；SectionInfo 旧口径：文件无尺寸不显示该行；档案 createdAt 有则显示）
const dataRows = computed<{ label: string; values: string[] }[]>(() => {
  if (props.target.kind === "file") {
    const d = props.target.detail;
    const out: { label: string; values: string[] }[] = [];
    if (d.width != null && d.height != null) out.push({ label: "尺寸", values: [`${d.width} x ${d.height}`] });
    out.push({ label: "文件大小", values: [formatSize(d.size)] });
    out.push({ label: "格式", values: [d.ext.toUpperCase()] });
    out.push({ label: "添加日期", values: formatDetailDate(d.importedAt).split(" ") });
    out.push({ label: "修改日期", values: formatDetailDate(d.modifiedAt).split(" ") });
    return out;
  }
  const d = props.target.data;
  const out: { label: string; values: string[] }[] = [
    { label: "文件数", values: [d.fileCount.toLocaleString()] },
    { label: "占用空间", values: [formatSize(d.totalSize)] },
  ];
  if (d.createdAt) out.push({ label: "创建日期", values: formatDetailDate(d.createdAt).split(" ") });
  return out;
});
</script>

<template>
  <!-- 输入框区块（稿：padding 12 / gap 8 / border-top 通用/三级） -->
  <section class="db-block inputs">
    <TextInput :model-value="name" readonly placeholder="名称" />
    <TextInput :model-value="firstLink" readonly placeholder="http://" trailing-icon="link" />
  </section>

  <!-- 属性标签区块（稿：padding 12 / gap 12 / border-top；chips 行 gap 6） -->
  <section class="db-block tags">
    <span v-for="t in propWords" :key="t" class="prop-chip">{{ t }}</span>
    <IconButton name="add-small" variant="bare" size="sm" title="添加属性" @click="openPropsDialog" />
  </section>

  <!-- 描述详情区块（稿：条目列表 gap 12；标题行 space-between = 标题 正文-加粗 四级 + expand 折叠钮；
       正文 13/18 二级，超 3 行截断 + 展开/收起小字钮（6.1 口径）） -->
  <section class="db-block descs">
    <div v-for="(d, i) in descriptions" :key="i" class="desc-item">
      <div class="desc-head" @click="toggleBody(i)">
        <span class="desc-title">{{ d.title }}</span>
        <AppIcon
          name="expand"
          :size="14"
          class="desc-chev"
          :class="{ collapsed: bodyCollapsed.has(i) }"
        />
      </div>
      <template v-if="!bodyCollapsed.has(i)">
        <span
          :ref="(el) => { if (el) descEls[i] = el as HTMLElement; }"
          class="desc-content"
          :class="{ clamp: !expanded.has(i) }"
        >{{ d.content }}</span>
        <button v-if="needsClamp[i]" class="desc-toggle" @click="toggleDesc(i)">
          {{ expanded.has(i) ? "收起" : "展开" }}
        </button>
      </template>
    </div>
    <ActionButton label="编辑描述" icon="pencil" variant="solid" size="md" @click="openDescDialog" />
  </section>

  <!-- 基本数据区块（稿：信息列表卡 = BaseDataItem） -->
  <section class="db-block data">
    <BaseDataItem :rows="dataRows" />
  </section>
</template>

<style scoped>
/* 详情面板区块公共壳（稿）：padding 12 / border-top 1px 通用/三级；区块内 gap 按稿分档 */
.db-block {
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
}
.db-block.inputs {
  gap: var(--n-space-8);
}
.db-block.tags,
.db-block.descs,
.db-block.data {
  gap: var(--n-space-12);
}
/* 属性标签区块（稿：水平排、垂直居中；chips gap 6） */
.db-block.tags {
  flex-direction: row;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--n-space-6);
}
/* 属性 chip（稿：高 25 / padding 4 8 / 通用四级底 + 通用六级 1px / 圆角 6 / 文字二级；
   字档 = 次要文字 12/16——figma「标签」档 12/17 Token.md 未收录，2026-08-31 用户定就近用档不补 token） */
.prop-chip {
  display: inline-flex;
  align-items: center;
  height: 25px;
  box-sizing: border-box;
  padding: var(--n-space-4) var(--n-space-8);
  background: var(--n-g4);
  border: 1px solid var(--n-g6);
  border-radius: var(--n-radius-6);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l2);
  white-space: nowrap;
}
/* 描述条目（稿：条目内 标题行—正文 gap 4；列表 gap 12） */
.desc-item {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
.desc-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
  cursor: default;
}
/* 标题 = 正文-加粗 13/18 亮色/四级 */
.desc-title {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l4);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* 折叠钮 = expand 14 四级（ SectionTitle 同款朝向口径：展开原样 ∨ / 收起 rotate -90） */
.desc-chev {
  flex: none;
  color: var(--n-l4);
  transition: transform var(--n-dur-100) var(--n-ease-in-out);
}
.desc-chev.collapsed {
  transform: rotate(-90deg);
}
/* 正文 = 13/18 亮色/二级；超 3 行截断（6.1 口径） */
.desc-content {
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
.desc-content.clamp {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
  overflow: hidden;
}
/* 展开 / 收起钮（2026-08-23 用户定：标签色小字钮，hover 提亮常态色；字档就近 12/16 次要文字） */
.desc-toggle {
  align-self: flex-start;
  color: var(--n-l4);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.desc-toggle:hover {
  color: var(--n-l1);
}
</style>
