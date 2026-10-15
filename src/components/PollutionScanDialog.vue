<script setup lang="ts">
// 扫描污染文件弹窗（2026-10-16 立项，PRD 03_查找文件-05）：全库资产 info 目录异物总览。
// 行 = PollutionScanRow（点击 = 定位：跳转资产视口 + 详情切链接关系 tab；hover 滑入 = 打开
// 资源管理器 / 全部丢到回收站）。底部 = 计数 + 关闭（无跨资产一键清——处置前至少过一眼，PRD 口径）。
// 壳与滚动 = scanDialog.css 扫描弹窗族共享件。
import { computed, ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import PollutionScanRow from "./PollutionScanRow.vue";
import ScrollThumb from "./ScrollThumb.vue";
import {
  pollutionScanState,
  closePollutionScan,
  locatePollution,
  revealPollutionInfo,
  trashPollutionAsset,
} from "../lib/pollutionScanOps";
import "./scanDialog.css";

const assetCount = computed(() => pollutionScanState.groups?.length ?? 0);
const itemCount = computed(() => (pollutionScanState.groups ?? []).reduce((sum, g) => sum + g.items.length, 0));

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const listBody = ref<HTMLElement>();
</script>

<template>
  <BaseModal title="扫描污染文件" :width="714" framed class="scan-dialog" @close="closePollutionScan">
    <div class="scd-scroll">
      <div ref="listBody" class="scd-body kit-scroll">
        <div class="ps-list">
          <PollutionScanRow
            v-for="row in pollutionScanState.groups"
            :key="row.asset.assetId"
            :row="row"
            :busy="pollutionScanState.busy"
            @locate="locatePollution(row)"
            @reveal="revealPollutionInfo(row)"
            @trash="trashPollutionAsset(row)"
          />
        </div>
      </div>
      <ScrollThumb :body="listBody" />
    </div>

    <template #actions>
      <!-- 底部：左计数右关闭（单钮无主次——即时处置在行内，底栏只收口） -->
      <div class="scd-bottom">
        <div class="scd-summary">
          <span class="scd-sel">共 <b>{{ assetCount }}</b> 个资产有污染文件</span>
          <span class="scd-saved">合计 {{ itemCount }} 个条目</span>
        </div>
        <div class="scd-btns">
          <ActionButton variant="cancel" size="lg" label="关闭" :disabled="pollutionScanState.busy" @click="closePollutionScan" />
        </div>
      </div>
    </template>
  </BaseModal>
</template>

<style scoped>
/* 行列表：容器 padding 12 + gap 8（弹窗内列表族口径） */
.ps-list {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
}
</style>
