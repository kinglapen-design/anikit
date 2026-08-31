<script setup lang="ts">
// 链接关系 item——docs/UI 重构/Design Token/组件/item/详情面板 item/链接关系（2026-08-28 用户补稿）
// 卡片 240×50 / padding 12 / gap 6：关系标签 chip（通用六级底 + 立体感-通用 + 圆角 6，
// 正文-加粗 亮色/三级；**仅悬停 chip 本身** = 亮色/半透明-2 + 输入指针，卡 hover 不变——
// 2026-08-28 走查定，不然 chip 没反馈；输入态 = 去立体感 + 通用二级底 +
// 亮色/半透明-1 描边 + 展开宽度）+ 目标文件名（正文 二级，右对齐顶到右缘，hover 被滑入的关闭钮推左）。
import "./items.css";
import { nextTick, ref, watch } from "vue";
import AppIcon from "../AppIcon.vue";

const props = withDefaults(
  defineProps<{
    relation: string; // 关系称谓（如「父亲」）
    target: string; // 目标文件显示名
    editing?: boolean; // 关系标签输入态（真实业务中由点击 chip 进入）
    forceState?: "" | "hover" | "chip-hover"; // 预览页强制态
  }>(),
  { editing: false, forceState: "" },
);

const emit = defineEmits<{ click: []; close: []; editRelation: []; commit: [value: string]; cancel: [] }>();

// 称谓输入态接线（7.1 二十三轮补，详情面板关系区块业务）：进入 editing = 本地 draft 取现值 +
// 聚焦全选（nextTick 等 input 挂载）；Enter / blur = 提交，Esc = 取消；blur 晚到双触发用 done 闸
const draft = ref("");
const inputRef = ref<HTMLInputElement>();
let done = true; // 非编辑态恒 true（防 mount 首帧 blur 误提交）
watch(
  () => props.editing,
  (v) => {
    if (!v) return;
    draft.value = props.relation;
    done = false;
    void nextTick(() => inputRef.value?.select());
  },
);
function commit(): void {
  if (done) return;
  done = true;
  emit("commit", draft.value.trim());
}
function cancel(): void {
  if (done) return;
  done = true;
  emit("cancel");
}
</script>

<template>
  <div
    class="lr ni-card ni-hover"
    :class="[forceState ? `force-${forceState}` : '']"
    @click="emit('click')"
  >
    <span
      class="chip"
      :class="{ editing }"
      @click.stop="emit('editRelation')"
    >
      <input
        v-if="editing"
        ref="inputRef"
        v-model="draft"
        class="chip-input"
        maxlength="20"
        @keydown.enter.prevent="commit"
        @keydown.esc.prevent="cancel"
        @blur="commit"
        @click.stop
      />
      <template v-else>{{ relation }}</template>
    </span>
    <span class="tgt">{{ target }}</span>
    <span class="ni-slide">
      <span class="ni-slide-inner">
        <button class="ni-close" @click.stop="emit('close')">
          <AppIcon name="close-small" :size="14" />
        </button>
      </span>
    </span>
  </div>
</template>

<style scoped>
.lr {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  width: 240px;
  height: 50px;
  padding: var(--n-space-12);
  cursor: pointer;
}
.chip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-g6);
  box-shadow: var(--n-effect-raised);
  border: 1px solid transparent;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
  white-space: nowrap;
  flex: none;
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
.chip:hover:not(.editing),
.lr.force-chip-hover .chip:not(.editing) {
  background: var(--n-l-a2);
  cursor: text;
}
/* 输入态：去立体感 + 通用二级底 + 亮色/半透明-1 描边 + 展开宽度 */
.chip.editing {
  flex: 1;
  min-width: 0;
  background: var(--n-g2);
  border-color: var(--n-l-a1);
  box-shadow: none;
  cursor: text;
}
.chip-input {
  width: 100%;
  min-width: 0;
  padding: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
  background: transparent;
}
.tgt {
  flex: 1;
  min-width: 0;
  /* 名称右对齐顶到右缘（2026-08-28 走查定）；hover 时滑入的关闭钮把它往左推（推挤机制） */
  text-align: right;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
