<script setup lang="ts">
// 扫描文件/档案链接弹窗（35 稿复用口径 + PRD 03_查找文件-04，2026-10-16 立项，待办 2026-08-13 兑现）：
// 单屏 714 = 组列表（行 = 首列原始文件卡 + 链接卡横排；链接卡复选 = 删除标记，选中压暗 30%，
// 默认全不选）+ 底部「已选择 N 个链接文件」与 取消 / 删除链接文件（次左主右）。
// 无第二屏——删链接是记录级操作（recycle 语义 + undo 一步），不需要重复扫描那种合并前确认。
// 取消 / 关闭 = 全部不处理（纯前端清状态）；壳与组行样式 = scanDialog.css 扫描弹窗族共享件。
import { computed, ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import LinkScanGroupRow from "./LinkScanGroupRow.vue";
import ScrollThumb from "./ScrollThumb.vue";
import {
  linkScanState,
  removeLinkGroup,
  selectedLinkCount,
  closeLinkScan,
  confirmDeleteLinks,
} from "../lib/linkScanOps";
import "./scanDialog.css";

const selectedCount = computed(() => selectedLinkCount());

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const groupsBody = ref<HTMLElement>();
</script>

<template>
  <BaseModal title="扫描文件/档案链接" :width="714" framed class="scan-dialog" @close="closeLinkScan">
    <!-- 组列表（35 稿）；悬浮滚轮悬停显隐（主窗口统一交互） -->
    <div class="scd-scroll">
      <div ref="groupsBody" class="scd-body kit-scroll">
        <LinkScanGroupRow
          v-for="g in linkScanState.groups"
          :key="g.asset.assetId"
          :group="g"
          @remove="removeLinkGroup(g.asset.assetId)"
        />
      </div>
      <ScrollThumb :body="groupsBody" />
    </div>

    <template #actions>
      <!-- 底部（35 稿：左计数右按钮；次左主右） -->
      <div class="scd-bottom">
        <div class="scd-summary">
          <span class="scd-sel">已选择 <b>{{ selectedCount }}</b> 个链接文件</span>
        </div>
        <div class="scd-btns">
          <ActionButton variant="cancel" size="lg" label="取消" :disabled="linkScanState.busy" @click="closeLinkScan" />
          <ActionButton
            variant="primary"
            size="lg"
            label="删除链接文件"
            :disabled="selectedCount === 0 || linkScanState.busy"
            @click="confirmDeleteLinks"
          />
        </div>
      </div>
    </template>
  </BaseModal>
</template>
