<script setup lang="ts">
// 信息查看管理弹窗（2026-09-26 三 tab 化重立，旧 22 稿 SectionEditPanel 后继）：
// 壳 = modals/BaseModal framed 460（管理弹窗族：区块管理 559 / 分组管理 640 同族先例）；
// 内容 = SegmentedControl text 型三段（基础信息 / 链接关系 / 自定义组件——与详情面板三 tab 一一对应，
// 2026-09-26 提示词二轮：原「提示词」段改自定义组件（占位容器无区块行），提示词挪基础信息区块，
// g3 壳语境 tone g2，速查 §48）+ 当前 tab 区块行列表（DetailBlockRow：箭头排序 + view 眼睛显隐；
// 行 = 注册表全量序含隐藏行）。
// 生效 = 即时（旧 22 稿口径：纯视图配置直写 ui.detailBlocks，settings.ui 防抖 500ms 写回，
// 详情面板实时跟随）——无确认/取消按钮行；打开默认落详情面板当前 tab（ui.detailTab 快照，
// 面板内切换仅查看管理不反写）。
// tab 切换动画 = 详情面板同款（旧左出 / 新右入 48px，速查 §23 时序四调）；
// 排序动画 = TransitionGroup 交换 150ms（区块管理 bm-swap 同款）。
// 入口 = 右栏底部「信息查看管理」钮 / 视图菜单同名项（ui.infoManageOpen），宿主 App.vue。
import { computed, ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import ScrollThumb from "./ScrollThumb.vue";
import DetailBlockRow from "./DetailBlockRow.vue";
import { DETAIL_TAB_BLOCK_TITLES, type DetailTabKey } from "../lib/detailSections";
import { useUiStore } from "../stores/ui";

const emit = defineEmits<{ close: [] }>();
const ui = useUiStore();

// 分段三段（与详情面板三 tab 一一对应）；打开落详情当前 tab（快照，不反写 ui.detailTab）
const TAB_OPTIONS = [
  { value: "basic", label: "基础信息" },
  { value: "links", label: "链接关系" },
  { value: "custom", label: "自定义组件" },
] as const;
const tabKey = ref<DetailTabKey>(ui.detailTab);
// SegmentedControl model 为 string——computed 包装收窄（DetailSections tab 同款写法）
const tab = computed<string>({
  get: () => tabKey.value,
  set: (v: string) => {
    tabKey.value = v as DetailTabKey;
  },
});

// 行 = 注册表全量序（含隐藏行；隐藏行标题禁用色，眼睛可切回）
const cfg = computed(() => ui.detailBlocks[tabKey.value]);
const rows = computed(() => cfg.value.order);
function titleOf(id: string): string {
  return DETAIL_TAB_BLOCK_TITLES[tabKey.value][id] ?? id;
}
function isHidden(id: string): boolean {
  return cfg.value.hidden.includes(id);
}

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const rowsEl = ref<HTMLElement>();
</script>

<template>
  <BaseModal title="信息查看管理" :width="460" framed class="info-manage" @close="emit('close')">
    <!-- 分段控制器（text 型 tone g2——g3 壳语境，速查 §48） -->
    <div class="im-seg">
      <SegmentedControl v-model="tab" :options="TAB_OPTIONS" variant="text" tone="g2" />
    </div>
    <!-- 行列表（定高 5 行 = 260：切 tab 弹窗不跳高，超出滚动——区块管理行封顶先例）；
         tab 切换滑动动画 = 详情面板同款（方向恒定旧左出 / 新右入） -->
    <div class="im-wrap">
      <div ref="rowsEl" class="im-rows kit-scroll">
        <Transition name="im-x">
          <div :key="tabKey" class="im-pane">
            <TransitionGroup name="im-swap" tag="div" class="im-pane-inner">
              <DetailBlockRow
                v-for="(id, i) in rows"
                :key="id"
                :title="titleOf(id)"
                :hidden="isHidden(id)"
                :is-first="i === 0"
                :is-last="i === rows.length - 1"
                @move-up="ui.moveDetailBlock(tabKey, id, i - 1)"
                @move-down="ui.moveDetailBlock(tabKey, id, i + 1)"
                @toggle="ui.toggleDetailBlock(tabKey, id)"
              />
            </TransitionGroup>
          </div>
        </Transition>
      </div>
      <ScrollThumb :body="rowsEl" />
    </div>
  </BaseModal>
</template>

<style scoped>
/* 分段控制器块（framed 内容段自承 padding 0 24） */
.im-seg {
  padding: 0 var(--n-space-24);
  flex: none;
}
/* 行列表壳（relative 供 ScrollThumb 贴右缘 + im-x 离场 absolute 定位锚） */
.im-wrap {
  position: relative;
  padding: 0 var(--n-space-24) var(--n-space-24);
  flex: none;
}
/* 定高 5 行（52×5 = 260）：切 tab 弹窗不跳高；超出滚动 */
.im-rows {
  height: 260px;
  overflow-y: auto;
  overflow-x: hidden;
}
.im-pane {
  min-height: 100%;
}
/* tab 切换滑动动画（详情面板同款，速查 §23 时序四调：退 150 缓出先行 / 入 250 缓入 delay 100 /
   位移 48px / 离场 absolute 不占位；方向恒定 = 旧左出 / 新右入） */
.im-x-enter-active {
  transition:
    transform var(--n-dur-250) var(--n-ease-in) var(--n-dur-100),
    opacity var(--n-dur-250) var(--n-ease-in) var(--n-dur-100);
}
.im-x-leave-active {
  position: absolute;
  inset: 0 var(--n-space-24) var(--n-space-24);
  transition:
    transform var(--n-dur-150) var(--n-ease-out),
    opacity var(--n-dur-150) var(--n-ease-out);
}
.im-x-enter-from {
  transform: translateX(48px);
  opacity: 0;
}
.im-x-leave-to {
  transform: translateX(-48px);
  opacity: 0;
}
/* 箭头排序交换动画（区块管理 bm-swap 同款 150ms） */
.im-swap-move {
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
</style>
