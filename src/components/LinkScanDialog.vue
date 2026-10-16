<script setup lang="ts">
// 扫描文件/档案链接弹窗（PRD 03_查找文件-04，2026-10-16 立项 + 同日两段式改版——与扫描重复文件同款两步）：
// 屏 1（groups，714）= 组列表：组级复选默认全选（选中 = 本组进归并）+ 行尾红 X 纯前端摘组，
// 卡片横排只看不动；底部「已选择 N 个文件 · 将清除 M 个位置」与 取消 / 合并。
// 屏 2（confirm，970，合并前确认）= 左组列表 + 右保留者 radio 单选：**每组选一个位置保留当原始文件，
// 其余位置（链接 + 被替换的旧 origin）全部清除进回收站**；默认保留者 = 当前 origin（行 0）。
// 逐条删链接归链接管理，本弹窗只做整组归并；归并 = linkScanOps.confirmMergeLinks（consolidate_asset_links 单命令）。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import Radio from "./buttons/Radio.vue";
import AppIcon from "./AppIcon.vue";
import LinkScanGroupRow from "./LinkScanGroupRow.vue";
import ScrollThumb from "./ScrollThumb.vue";
import {
  linkScanState,
  keeperRows,
  removeLinkGroup,
  selectedGroups,
  enterConfirm,
  backToGroups,
  closeLinkScan,
  confirmMergeLinks,
  type LinkGroup,
} from "../lib/linkScanOps";
import "./scanDialog.css";

// —— 屏 1 计数：已选择 = 选中组数；将清除 = Σ(组内位置数 - 1)（保留者恒存 1 个位置）——
const selectedFileCount = computed(() => selectedGroups().length);
const clearCount = computed(() => selectedGroups().reduce((sum, g) => sum + g.asset.placements.length - 1, 0));

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const groupsBody = ref<HTMLElement>();
const groupListEl = ref<HTMLElement>();
const keeperListEl = ref<HTMLElement>();

// —— 屏 2 ——
const confirmGroups = computed(() => selectedGroups());
const currentGroup = computed<LinkGroup | null>(
  () => confirmGroups.value.find((g) => g.asset.assetId === linkScanState.currentAsset) ?? confirmGroups.value[0] ?? null,
);
const currentRows = computed(() => (currentGroup.value ? keeperRows(currentGroup.value) : []));

function selectGroup(assetId: string): void {
  linkScanState.currentAsset = assetId;
}

// 保留者选择（radio 单选 = keeperRows 下标；整行可点，Eagle 同交互）
function keeperOf(assetId: string): number {
  return linkScanState.keepers[assetId] ?? 0;
}
function setKeeper(assetId: string, index: number): void {
  linkScanState.keepers[assetId] = index;
}

function thumbUrl(g: LinkGroup): string | null {
  return g.asset.thumbPath ? convertFileSrc(g.asset.thumbPath) : null;
}
</script>

<template>
  <BaseModal
    :title="linkScanState.step === 'groups' ? '扫描文件/档案链接' : '合并前确认'"
    :width="linkScanState.step === 'groups' ? 714 : 970"
    framed
    class="scan-dialog"
    @close="closeLinkScan"
  >
    <!-- 屏 1：组列表（35 稿族）；悬浮滚轮悬停显隐（主窗口统一交互） -->
    <div v-if="linkScanState.step === 'groups'" class="scd-scroll">
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

    <!-- 屏 2：合并前确认（左组列表 + 右保留者单选；样式 = scanDialog.css scd- 共享族） -->
    <div v-else class="scd-confirm">
      <div class="scd-gwrap">
        <aside ref="groupListEl" class="scd-glist kit-scroll">
          <button
            v-for="g in confirmGroups"
            :key="g.asset.assetId"
            class="scd-gitem"
            :class="{ cur: g.asset.assetId === currentGroup?.asset.assetId }"
            @click="selectGroup(g.asset.assetId)"
          >
            <span class="scd-gthumb">
              <img
                v-if="thumbUrl(g)"
                :class="{ icon: g.asset.thumbKind === 'formatIcon' }"
                :src="thumbUrl(g)!"
                :alt="g.asset.name"
                draggable="false"
              />
              <span v-else class="ext">{{ g.asset.ext.toUpperCase() }}</span>
            </span>
            <span class="scd-gtext">
              <span class="scd-gname" :title="g.asset.name">{{ g.asset.name }}</span>
              <span class="scd-gcount">{{ g.links.length }} 个链接文件</span>
            </span>
          </button>
        </aside>
        <ScrollThumb :body="groupListEl" />
      </div>
      <section v-if="currentGroup" class="scd-kwrap">
        <div ref="keeperListEl" class="scd-klist kit-scroll">
          <!-- 保留者候选行：origin 行在前（★ 原始文件徽章），其后全部链接（链接徽章）——
               选一个保留当原始文件，其余全部清除 -->
          <div
            v-for="(row, i) in currentRows"
            :key="`${row.folderId}:${row.origin}:${row.name}`"
            class="scd-kitem"
            :class="{ on: keeperOf(currentGroup.asset.assetId) === i }"
            @click="setKeeper(currentGroup.asset.assetId, i)"
          >
            <Radio
              :model-value="keeperOf(currentGroup.asset.assetId)"
              :value="i"
              @update:model-value="setKeeper(currentGroup.asset.assetId, Number($event))"
            />
            <span class="scd-kthumb">
              <img
                v-if="thumbUrl(currentGroup)"
                :class="{ icon: currentGroup.asset.thumbKind === 'formatIcon' }"
                :src="thumbUrl(currentGroup)!"
                :alt="row.name"
                draggable="false"
              />
              <span v-else class="ext">{{ currentGroup.asset.ext.toUpperCase() }}</span>
            </span>
            <span class="scd-kname" :title="row.name">{{ row.name }}</span>
            <AppIcon
              :name="row.origin ? 'original-file-badge' : 'linked-file-badge'"
              :size="14"
              class="kbadge"
              :title="row.origin ? '当前原始文件' : '链接文件'"
            />
            <span class="scd-kmeta">
              <span class="scd-chip" :title="row.folderName">{{ row.folderName }}</span>
            </span>
          </div>
        </div>
        <ScrollThumb :body="keeperListEl" />
      </section>
    </div>

    <template #actions>
      <!-- 屏 1 底部（左计数右按钮；次左主右） -->
      <div v-if="linkScanState.step === 'groups'" class="scd-bottom">
        <div class="scd-summary">
          <span class="scd-sel">已选择 <b>{{ selectedFileCount }}</b> 个文件</span>
          <span class="scd-saved">将清除 {{ clearCount }} 个位置</span>
        </div>
        <div class="scd-btns">
          <ActionButton variant="cancel" size="lg" label="取消" :disabled="linkScanState.busy" @click="closeLinkScan" />
          <ActionButton
            variant="primary"
            size="lg"
            label="合并"
            :disabled="selectedFileCount === 0 || linkScanState.busy"
            @click="enterConfirm"
          />
        </div>
      </div>
      <!-- 屏 2 底部：上一步（次，左）/ 确认合并（主，右）——次左主右 -->
      <div v-else class="scd-bottom end">
        <ActionButton variant="solid" size="lg" label="上一步" :disabled="linkScanState.busy" @click="backToGroups" />
        <ActionButton variant="primary" size="lg" label="确认合并" :disabled="linkScanState.busy" @click="confirmMergeLinks" />
      </div>
    </template>
  </BaseModal>
</template>

<style scoped>
/* 保留者行位置徽章（origin ★ / 链接 icon 成对，详情链接 item §57 同口径）：14 亮色/一级 */
.kbadge {
  flex: none;
  color: var(--n-l1);
}
</style>
