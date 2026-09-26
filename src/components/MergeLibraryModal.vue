<script setup lang="ts">
// 合并资源库弹窗（壳/列表/底行 = 2026-09-08 布局/模态窗口/合并资源库 稿；2026-09-26 功能轮接通 +
// 二轮交互修订：列表 = 当前空间**全部**资源库（libraryOrder 序，含当前库），勾选 ≥2 才可合并，
// 目标 = 勾选集中最上方（libraryOrder 最前）的库，其余并入它——合并完成即打开目标库
//（目标非当前库时表现为切库，afterOpen 收尾））：壳 = modals/BaseModal（498 宽稿值）。
// 列表行 = 34 高 padding 8/12 圆角 6 space-between，未选 通用/四级底 / 选中 通用/六级底 +
// 1px 亮色/半透明-3 + 尾对勾（check 18 亮色/一级，未选 opacity 0 占位——稿口径）；
// 容量列 = 稿 320GB 位（数字-大 13/18 三级；引用资产 size 求和，list_merge_candidates 加载）。
// 底行 = 「找到 N 个资源库」（正文 13/18 三级 + N 数字-大品牌色）；勾选够两个 → 换显
// 「并入「目标库名」」（目标名 正文-加粗 一级——稿未画此态，功能必需就近正文档加粗档）；
// 确认钮 disabled = 控件族通用口径（30% 透明度 + 不响应，§39②）。
// 列表超量滚动稿未画——库量级小（2026-09-07 用户：资源库不至于多到需要排序），自然撑高不预设。
import { computed, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import { confirmMergeLibraries, type MergeCandidate } from "../lib/libraryOps";
import { formatSize } from "../lib/formatSize";

const props = defineProps<{ items: MergeCandidate[] }>();
const emit = defineEmits<{ close: [] }>();

const selected = ref<Set<string>>(new Set());
function toggle(name: string): void {
  const next = new Set(selected.value);
  if (next.has(name)) next.delete(name);
  else next.add(name);
  selected.value = next;
}

// 目标库 = 勾选集中最上方（libraryOrder 最前）者；勾选 ≥2 才可确认
const targetName = computed(() => props.items.find((it) => selected.value.has(it.name))?.name ?? null);
const confirmable = computed(() => selected.value.size >= 2);

const busy = ref(false);
async function onConfirm(): Promise<void> {
  if (busy.value || !confirmable.value) return;
  busy.value = true;
  try {
    await confirmMergeLibraries([...selected.value]);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseModal title="合并资源库" :width="498">
    <div class="lib-list">
      <button
        v-for="it in props.items"
        :key="it.name"
        class="lib-item"
        :class="{ on: selected.has(it.name) }"
        @click="toggle(it.name)"
      >
        <span class="lib-main">
          <span class="lib-name">{{ it.name }}</span>
          <span class="lib-size">{{ formatSize(it.sizeBytes) }}</span>
        </span>
        <AppIcon name="check" :size="18" class="lib-check" />
      </button>
    </div>
    <div class="foot">
      <span class="hint">
        <template v-if="confirmable && targetName">并入「<span class="target">{{ targetName }}</span>」</template>
        <template v-else>找到 <span class="num">{{ props.items.length }}</span> 个资源库</template>
      </span>
      <span class="btns">
        <ActionButton label="取消" variant="cancel" size="lg" @click="emit('close')" />
        <ActionButton label="确认" variant="primary" size="lg" :disabled="!confirmable" @click="onConfirm" />
      </span>
    </div>
  </BaseModal>
</template>

<style scoped>
/* 资源库列表（稿）：列 gap 6；行 34 高 padding 8/12 圆角 6 space-between */
.lib-list {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-6);
}
.lib-item {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
  box-sizing: border-box;
  height: 34px;
  padding: var(--n-space-8) var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g4);
  cursor: default;
}
/* 选中 = 通用/六级底 + 1px 亮色/半透明-3 真 border（稿 border 口径）+ 对勾现身 */
.lib-item.on {
  background: var(--n-g6);
  border: 1px solid var(--n-l-a3);
}
/* 名称+容量组（稿 gap 12）；容量 = 数字-大 13/18 三级 */
.lib-main {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-12);
  min-width: 0;
}
.lib-name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.lib-size {
  flex: none;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
}
.lib-check {
  flex: none;
  color: var(--n-l1);
  opacity: 0; /* 未选对勾占位（稿 opacity 0 口径） */
}
.lib-item.on .lib-check {
  opacity: 1;
}
/* 底行（稿「提示文字+按钮」）：space-between，提示 正文 13/18 三级 + N 数字-大品牌色 */
.foot {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-24);
}
.hint {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 并入目标库名（勾选够两个时换显）：正文-加粗 一级 */
.hint .target {
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.hint .num {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-brand);
}
.btns {
  display: flex;
  flex-direction: row;
  gap: var(--n-space-8);
  flex: none;
}
</style>
