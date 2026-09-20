<script setup lang="ts">
// 属性标签弹窗（32_属性标签稿，6.1；文件与档案共用）：
// 壳 = 479 稿宽（482 真 border 补偿）/ 标题栏 54 / 按钮栏 62（主左次右 88×38）；
// 内容 = 搜索框（过滤池）+ 拥有属性行（chips × 移除 + 输入新属性回车添加 + 添加钮）
// + 属性池「已有属性标签：」（全库聚合，点 chip 加入拥有——轻微过冲回弹，稿 prose 定；
// 池内 × = 全局删除：警告二次确认后立即生效，全库文件与档案该词失效，不随本弹窗取消回滚）。
// 确认 / 取消 = 草稿式：只提交本对象属性集（一步 undo），取消全部放弃。
import { computed, onMounted, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppSearchBar from "./AppSearchBar.vue";
import TagButton from "./buttons/TagButton.vue";
import IconButton from "./IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { useUiStore } from "../stores/ui";
import { useSelectionStore } from "../stores/selection";
import { deletePropsGlobally, listPropPool, setArchiveProps, setAssetProps } from "../lib/customDataOps";

const ui = useUiStore();
const selection = useSelectionStore();
const target = computed(() => ui.propsEdit);
const isFile = computed(() => target.value?.kind === "file");

// 草稿：拥有属性（初值 = 打开时快照）+ 池 + 搜索词
const owned = ref<string[]>([...(target.value?.initial ?? [])]);
const pool = ref<string[]>([]);
const keyword = ref("");
const draft = ref(""); // 新属性输入
const busy = ref(false);
const popKey = ref(""); // 新加入 chip（过冲回弹动画挂点）
const poolRef = ref<HTMLElement>(); // 属性池滚动容器（ScrollThumb 锚：包裹层 = .pd-body）

onMounted(async () => {
  pool.value = await listPropPool();
});

// 池过滤（搜索框）；已删除的词从池剔除
const poolShown = computed(() => {
  const k = keyword.value.trim().toLowerCase();
  if (!k) return pool.value;
  return pool.value.filter((w) => w.toLowerCase().includes(k));
});

// 加入拥有（去重；过冲回弹动画触发）
function add(word: string): void {
  const w = word.trim();
  if (!w || owned.value.includes(w)) return;
  owned.value.push(w);
  popKey.value = w;
}
// 输入新属性：回车 / 添加钮（IME 守卫——回车是选词键，不守卫会把中文输入确认误提交）
function onDraftEnter(e: KeyboardEvent): void {
  if (e.isComposing) return;
  add(draft.value);
  draft.value = "";
}

// 池内 × = 全局删除（32 稿 prose：二次确认，删除后所有填写了这个属性的地方都失效）
function removePoolWord(word: string): void {
  selection.confirmPrompt = {
    title: "删除属性标签",
    body: `删除后所有填写了「${word}」的地方都会失效（文件与档案的属性都会移除该词）。`,
    confirmLabel: "删除",
    alert: "warning",
    onConfirm: () => {
      void (async () => {
        if (await deletePropsGlobally([word])) {
          pool.value = pool.value.filter((w) => w !== word);
          owned.value = owned.value.filter((w) => w !== word);
        }
      })();
    },
  };
}

async function onConfirm(): Promise<void> {
  const t = target.value;
  if (!t || busy.value) return;
  busy.value = true;
  try {
    const ok = isFile.value ? await setAssetProps(t.id, owned.value) : await setArchiveProps(t.id, owned.value);
    if (ok) ui.propsEdit = null;
  } finally {
    busy.value = false;
  }
}
function onCancel(): void {
  if (busy.value) return;
  ui.propsEdit = null;
}
</script>

<template>
  <AppModal v-if="target" variant="standard" title="属性标签" class="props-dialog" @close="onCancel">
    <div class="pd-body">
      <!-- 搜索框（32 稿：36 高 C1 底；过滤池显示） -->
      <div class="pd-search">
        <AppSearchBar v-model="keyword" placeholder="搜索..." />
      </div>
      <!-- 拥有属性行（稿：gap 8 / chip 25 高；输入新属性小框 + 添加钮） -->
      <div class="pd-owned">
        <TagButton
          v-for="w in owned"
          :key="w"
          :label="w"
          removable
          :class="{ pop: w === popKey }"
          @remove="owned = owned.filter((x) => x !== w)"
        />
        <input
          v-model="draft"
          class="pd-new"
          placeholder="输入新属性"
          @keydown.enter.prevent="onDraftEnter"
        />
        <IconButton name="add-small" small title="添加" @click="add(draft); draft = ''" />
      </div>
      <!-- 属性池（稿：C1 底 + 顶分隔线，「已有属性标签：」标签色 + 流式 chips gap 8；
           滚轮 = ScrollThumb 悬浮件，2026-09-13 实机走查补——此前有滚动无滚轮） -->
      <div ref="poolRef" class="pd-pool kit-scroll">
        <span class="pool-label">已有属性标签:</span>
        <div class="pool-chips">
          <TagButton v-for="w in poolShown" :key="w" :label="w" removable @click="add(w)" @remove="removePoolWord(w)" />
        </div>
      </div>
      <ScrollThumb :body="poolRef" />
    </div>
    <template #actions>
      <div class="pd-bottom">
        <AppButton variant="primary" label="确认" :disabled="busy" @click="onConfirm" />
        <AppButton variant="panel" label="取消" :disabled="busy" @click="onCancel" />
      </div>
    </template>
  </AppModal>
</template>

<style scoped>
/* 32 稿总容器：479 宽（482 = 内容宽 + 真 border 3 补偿，DuplicateDialog 同口径）；
   高 = 稿 506 档（509 补偿），拥有行多行换行挤压池区（flex 1 min-height 0） */
.props-dialog :deep(.app-modal) {
  width: 482px;
  height: 509px;
  padding: 0;
  gap: 0;
}
/* 标题栏（32 稿）：54 高 / padding 12 12 12 24 / 下分隔线；标题常态（AppModal 默认） */
.props-dialog :deep(.titlebar) {
  height: 54px;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  box-shadow: inset 0 -1px 0 0 var(--line-hover);
  flex: none;
}
.props-dialog :deep(.content) {
  flex: 1;
  min-height: 0;
  padding: 0;
  gap: 0;
  overflow: hidden;
  /* 壳 padding 0：归零 AppModal .content 的负 margin 对（2026-09-13 实机走查贴右缘口径），
     .pd-body 通栏 = 属性池包裹右缘即弹窗缘 */
  margin-right: 0;
  padding-right: 0;
}
.props-dialog :deep(.actions) {
  flex: none;
}
.pd-body {
  position: relative; /* ScrollThumb 锚点（属性池滚轮，2026-09-13 实机走查贴右缘口径） */
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  gap: var(--space-12);
  padding-top: var(--space-12);
}
/* 搜索框容器（稿：padding 0 24） */
.pd-search {
  padding: 0 var(--space-24);
  flex: none;
}
/* 拥有属性行（稿：padding 0 24 / gap 8 / 行高 25，多行向下排自适应） */
.pd-owned {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-8);
  padding: 0 var(--space-24);
  min-height: 25px;
  flex: none;
}
/* 新加入 chip 过冲回弹（32 稿 prose：轻微过冲回弹效果） */
.pop {
  animation: chip-pop 0.28s cubic-bezier(0.34, 1.56, 0.64, 1);
}
@keyframes chip-pop {
  0% {
    transform: scale(0.6);
  }
  60% {
    transform: scale(1.12);
  }
  100% {
    transform: scale(1);
  }
}
/* 输入新属性小框（稿「输出框 - 小」：76×25 / C1 底 / 1px 暗-悬停 / 圆角 6 / padding 4 8 / 12 档极淡占位） */
.pd-new {
  width: 76px;
  height: 25px;
  box-sizing: border-box;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
}
.pd-new::placeholder {
  color: var(--text-faint);
}
/* 属性池（稿：C1 底 / 顶分隔线 / padding 12 24 24 / 标签 14/20 标签色 / 流式 chips gap 8） */
.pd-pool {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  flex: 1;
  min-height: 0;
  padding: var(--space-12) var(--space-24) var(--space-24);
  background: var(--c1);
  box-shadow: inset 0 1px 0 0 var(--line-hover);
  overflow-y: auto;
}
.pool-label {
  color: var(--text-label);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
}
.pool-chips {
  display: flex;
  flex-wrap: wrap;
  align-content: flex-start;
  gap: var(--space-8);
}
/* 池 chip 可点（点击加入拥有） */
.pool-chips .app-tag {
  cursor: pointer;
}
/* 底部按钮栏（62 高 / 上分隔线 / 主左次右 88×38，速查表 §1） */
.pd-bottom {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  height: 62px;
  box-shadow: inset 0 1px 0 0 var(--line-hover);
}
.pd-bottom :deep(.app-btn) {
  width: 88px;
}
</style>
