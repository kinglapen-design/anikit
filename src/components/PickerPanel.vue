<script setup lang="ts">
// 备选区（7.1 二十三轮贴稿重构，稿 = 右侧边栏-选择模式；业务 = 6.2 选择模式旧件不变）：
// 结构 = 备选区域（padding 12 / gap 12，kit-scroll + ScrollThumb 悬浮滚轮）+ 按钮栏 62（padding 12 / gap 8）。
// 模板一 relation（关系模式，多选累积——选一个 = 追加一条，2026-08-24 用户定）：
//   组卡 = items/PickRelationItem（选中文件框 + 关系双框（左 ↓ 选中→发起 / 右 ↑ 反向可空）+ 原始文件框
//   只读 + 关闭钮移出该组）；未选 = 占位组（选中框占位「点击卡片选择」+ 双框禁用 + 无关闭钮，旧口径沿）。
// 模板二 file（文件选择模式）：行卡 = items/PickFileItem 恒输入态（名称框本身就是输入框——默认源名可改，
//   确认按行写入）+ 关闭钮移出备选；用途 = link 添加文件链接 / voice 链接音频。
//   旧稿 15px 缩略图左标 / 链接状态徽章随新稿移除（稿 item = 名称 + 关闭钮）。
// 按钮栏 = 取消（solid 左）/ 确认（primary 右，带数量「确认 (N)」）flex 均分——
//   2026-08-31 用户全局定案「取消左确认右」（旧「确认主左」口径废除）；确认防重入在 picker.confirm 内。
import { computed, ref } from "vue";
import PickFileItem from "./items/PickFileItem.vue";
import PickRelationItem from "./items/PickRelationItem.vue";
import ActionButton from "./buttons/ActionButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { usePickerStore } from "../stores/picker";

const picker = usePickerStore();
const bodyRef = ref<HTMLElement>();

// 确认文案：带数量（PRD 多选口径：确认 (3)）
const confirmLabel = computed(() => (picker.picked.length > 0 ? `确认 (${picker.picked.length})` : "确认"));
// 确认禁用（空选不可确认；禁用态视觉稿未画不做——按钮族口径，仅 handler 拦截）
function onConfirm(): void {
  if (!picker.canConfirm) return;
  void picker.confirm();
}
</script>

<template>
  <div class="picker-panel">
    <!-- 备选区域（稿：padding 12 / gap 12；悬浮滚轮 ScrollThumb 同详情 tab 口径） -->
    <div class="pick-scroll-wrap">
      <div ref="bodyRef" class="pick-body kit-scroll">
        <!-- 模板一：关系模式（多选整组竖排） -->
        <template v-if="picker.template === 'relation'">
          <!-- 空态占位组（未选：引导占位 + 双框禁用 + 发起文件只读框；无关闭钮） -->
          <PickRelationItem
            v-if="picker.picked.length === 0"
            picked=""
            picked-placeholder="点击卡片选择"
            from-label=""
            to-label=""
            :origin="picker.origin?.name ?? ''"
            disabled
            :closable="false"
          />
          <!-- 已选组 × N（整组竖排复制，2026-08-24 用户定；发起文件框每组同显） -->
          <PickRelationItem
            v-for="(p, i) in picker.picked"
            :key="`${p.target.kind}|${p.target.id}`"
            :picked="p.target.name"
            v-model:from-label="p.label"
            v-model:to-label="p.reverseLabel"
            :origin="picker.origin?.name ?? ''"
            @close="picker.picked.splice(i, 1)"
          />
        </template>
        <!-- 模板二：文件选择模式（多选累积；行 = 名称输入框恒编辑态） -->
        <template v-if="picker.template === 'file'">
          <!-- 空态占位行（稿无空态，实现定：占位文案 + 输入框只读，无关闭钮） -->
          <PickFileItem
            v-if="picker.picked.length === 0"
            name=""
            editing
            placeholder="点击卡片选择"
            readonly
            :closable="false"
          />
          <PickFileItem
            v-for="p in picker.picked"
            :key="`${p.target.kind}|${p.target.id}`"
            :name="p.name ?? ''"
            editing
            @update:name="p.name = $event"
            @close="picker.togglePick(p.target)"
          />
        </template>
      </div>
      <ScrollThumb :body="bodyRef" />
    </div>
    <!-- 按钮栏 62（稿：padding 12 / gap 8；取消左 solid / 确认右 primary，flex 均分） -->
    <footer class="pick-actions">
      <ActionButton label="取消" variant="solid" size="lg" @click="picker.exit()" />
      <ActionButton :label="confirmLabel" variant="primary" size="lg" @click="onConfirm" />
    </footer>
  </div>
</template>

<style scoped>
/* 备选区（稿 = 总容器同款：随右栏 g1 底，无自身底色/边线——栏容器 border-left 统一承担） */
.picker-panel {
  display: flex;
  flex-direction: column;
  width: var(--sb-w); /* 跟随右栏宽（SidebarRight 定宽子块口径） */
  flex: 1;
  min-height: 0;
}
/* 备选区域（可滚；包裹层 relative 供悬浮滚轮贴右缘） */
.pick-scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.pick-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  overflow-y: auto;
  overflow-x: hidden;
}
/* item 宽随父容器（稿值 240 = 栏宽 264 - 左右 padding 12×2 画布语境，卡片族尺寸模型口径） */
.pick-body :deep(.ni-card) {
  width: 100%;
}
/* 按钮栏（稿：62 高 / padding 12 / gap 8；两钮 flex 均分——ActionButton lg 宽 100% 由父定 +
   flex:none 覆盖为均分，同提示词按钮组口径） */
.pick-actions {
  flex: none;
  display: flex;
  gap: var(--n-space-8);
  height: 62px;
  box-sizing: border-box;
  padding: var(--n-space-12);
}
.pick-actions :deep(.act-btn) {
  flex: 1;
  min-width: 0;
}
</style>
