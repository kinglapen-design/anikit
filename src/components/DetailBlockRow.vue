<script setup lang="ts">
// 信息查看管理行（2026-09-26 三 tab 化重立，旧 22 稿「编辑数据区块」行后继）：
// 高 52 / padding 12 0 / gap 8 / 底线 通用/五级（末行去线）——下移+上移箭头（buttons/IconButton
// bare sm chevron-small：源图形朝上，下移 rot-180；首行禁上 / 末行禁下，禁用 = 新件自带 30%
// 透明度）+ 区块标题（隐藏行 = 禁用色档——22 稿注「眼睛切换标题要禁用」）+ 显隐眼睛
//（view / view-off，2026-09-26 用户直投入库；bare lg 常态三级 hover 二级 = 新件自带档）。
// 排序交互 = 箭头 + TransitionGroup 交换动画（速查 §9 第 18 条定案：面板内列表行排序不做拖拽）。
import IconButton from "./buttons/IconButton.vue";

defineProps<{
  title: string; // 区块标题（注册表 DETAIL_TAB_BLOCK_TITLES）
  hidden: boolean; // 隐藏行：标题禁用色 + 眼睛 view-off
  isFirst: boolean; // 首行：上箭头禁用
  isLast: boolean; // 末行：下箭头禁用
}>();
const emit = defineEmits<{ moveUp: []; moveDown: []; toggle: [] }>();
</script>

<template>
  <div class="im-row">
    <!-- 排序箭头（chevron-small 源图形朝上——上移不旋转、下移旋转 180；可用态亮色/一级提权） -->
    <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow arrow-down" :disabled="isLast" title="下移" @click="emit('moveDown')" />
    <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow arrow-up" :disabled="isFirst" title="上移" @click="emit('moveUp')" />
    <span class="row-title" :class="{ off: hidden }">{{ title }}</span>
    <IconButton variant="bare" size="lg" :name="hidden ? 'view-off' : 'view'" :title="hidden ? '显示区块' : '隐藏区块'" @click="emit('toggle')" />
  </div>
</template>

<style scoped>
/* 行（22 稿口径）：52 高（border-box 含 padding）/ padding 12 0 / gap 8 / 底线 通用/五级；末行去线 */
.im-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 52px;
  padding: var(--n-space-12) 0;
  box-sizing: border-box;
  border-bottom: 1px solid var(--n-g5);
  flex: none;
}
.im-row:last-child {
  border-bottom: none;
}
/* 下箭头：chevron-small 源图形朝上，旋转 180 朝下 */
.arrow-down :deep(svg) {
  transform: rotate(180deg);
}
/* 排序箭头：可用态 = 亮色/一级（提权压组件自带 .ic-btn.bare——BlockManageRow 同款写法） */
.im-row .ic-btn.sort-arrow {
  color: var(--n-l1);
}
/* 区块标题：正文 13/18 亮色/二级（22 稿常态色就近档）；隐藏行 = 禁用色（22 稿注「眼睛切换标题要禁用」） */
.row-title {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.row-title.off {
  color: var(--n-l-disabled);
}
</style>
