<script setup lang="ts">
// 扫描重复文件弹窗（35_扫描重复文件稿 + Eagle 式合并前确认，2.7 二轮 2026-08-13，PRD 03_查找文件-03）。
// 屏 1（groups，714 宽）= 35 稿结果列表：组行（复选默认全选 / 红钮移除）+ 底部计数与合并/取消；
// 屏 2（confirm，970 宽）= 合并前确认：左列组列表 + 右列保留者 radio 单选
//（默认导入最早 = 后端排序 assets[0]），底部确认合并 / 上一步（回屏 1）。
// 取消 / 关闭 = 全部不处理（纯前端清状态）；合并经 dupScanOps.confirmMerge 一次 delete_placements。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import Radio from "./buttons/Radio.vue";
import DupScanGroupRow from "./DupScanGroupRow.vue";
import ScrollThumb from "./ScrollThumb.vue";
import "./scanDialog.css";
import { formatSize } from "../lib/formatSize";
import {
  dupScanState,
  removeGroup,
  selectedGroups,
  enterConfirm,
  backToGroups,
  closeScan,
  confirmMerge,
  type DupAsset,
  type DupGroup,
} from "../lib/dupScanOps";

// —— 屏 1 计数（35 稿：已选择 N 个文件 = 选中组资产总数；可节约 = Σ(n-1)×size）——
const selectedFileCount = computed(() =>
  selectedGroups().reduce((sum, g) => sum + g.assets.length, 0),
);
const savedBytes = computed(() =>
  selectedGroups().reduce((sum, g) => sum + (g.assets.length - 1) * g.size, 0),
);

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const groupsBody = ref<HTMLElement>();
const groupListEl = ref<HTMLElement>();
const keeperListEl = ref<HTMLElement>();

// —— 屏 2 ——
const confirmGroups = computed(() => selectedGroups());
const currentGroup = computed<DupGroup | null>(
  () => confirmGroups.value.find((g) => g.hash === dupScanState.currentHash) ?? confirmGroups.value[0] ?? null,
);

function selectGroup(hash: string): void {
  dupScanState.currentHash = hash;
}

// 保留者选择（radio 单选；整行可点，Eagle 同交互）
function keeperOf(hash: string): string {
  return dupScanState.keepers[hash] ?? "";
}
function setKeeper(hash: string, assetId: string): void {
  dupScanState.keepers[hash] = assetId;
}

function thumbUrl(a: DupAsset): string | null {
  return a.thumbPath ? convertFileSrc(a.thumbPath) : null;
}
</script>

<template>
  <BaseModal
    :title="dupScanState.step === 'groups' ? '扫描重复文件' : '合并前确认'"
    :width="dupScanState.step === 'groups' ? 714 : 970"
    framed
    class="scan-dialog"
    :class="{ confirm: dupScanState.step === 'confirm' }"
    @close="closeScan"
  >
    <!-- 屏 1：结果列表（35 稿）；悬浮滚轮悬停显隐（主窗口统一交互） -->
    <div v-if="dupScanState.step === 'groups'" class="scd-scroll">
      <div ref="groupsBody" class="scd-body kit-scroll">
        <DupScanGroupRow
          v-for="g in dupScanState.groups"
          :key="g.hash"
          :group="g"
          @remove="removeGroup(g.hash)"
        />
      </div>
      <ScrollThumb :body="groupsBody" />
    </div>

    <!-- 屏 2：合并前确认（Eagle 式：左组列表 + 右保留者单选；样式 = scanDialog.css scd- 共享族） -->
    <div v-else class="scd-confirm">
      <div class="scd-gwrap">
        <aside ref="groupListEl" class="scd-glist kit-scroll">
          <button
            v-for="g in confirmGroups"
            :key="g.hash"
            class="scd-gitem"
            :class="{ cur: g.hash === currentGroup?.hash }"
            @click="selectGroup(g.hash)"
          >
            <span class="scd-gthumb">
              <img
                v-if="g.assets[0] && thumbUrl(g.assets[0])"
                :class="{ icon: g.assets[0].thumbKind === 'formatIcon' }"
                :src="thumbUrl(g.assets[0])!"
                :alt="g.assets[0].name"
                draggable="false"
              />
              <span v-else class="ext">{{ g.assets[0]?.ext.toUpperCase() }}</span>
            </span>
            <span class="scd-gtext">
              <span class="scd-gname" :title="g.assets[0]?.name">{{ g.assets[0]?.name }}</span>
              <span class="scd-gcount">{{ g.assets.length }} 个文件</span>
            </span>
          </button>
        </aside>
        <ScrollThumb :body="groupListEl" />
      </div>
      <section v-if="currentGroup" class="scd-kwrap">
        <div ref="keeperListEl" class="scd-klist kit-scroll">
          <div
            v-for="a in currentGroup.assets"
            :key="a.assetId"
            class="scd-kitem"
            :class="{ on: keeperOf(currentGroup.hash) === a.assetId }"
            @click="setKeeper(currentGroup.hash, a.assetId)"
          >
            <Radio
              :model-value="keeperOf(currentGroup.hash)"
              :value="a.assetId"
              @update:model-value="setKeeper(currentGroup.hash!, String($event))"
            />
            <span class="scd-kthumb">
              <img
                v-if="thumbUrl(a)"
                :class="{ icon: a.thumbKind === 'formatIcon' }"
                :src="thumbUrl(a)!"
                :alt="a.name"
                draggable="false"
              />
              <span v-else class="ext">{{ a.ext.toUpperCase() }}</span>
            </span>
            <span class="scd-kname" :title="a.name">{{ a.name }}</span>
            <span class="scd-kmeta">
              <span class="size">{{ formatSize(a.size) }}</span>
              <span class="scd-chip" :title="a.folderName">{{ a.folderName }}</span>
            </span>
          </div>
        </div>
        <ScrollThumb :body="keeperListEl" />
      </section>
    </div>

    <template #actions>
      <!-- 屏 1 底部（35 稿：62 高 / padding 12 12 12 24 / 左计数右按钮；按钮次左主右新模态口径） -->
      <div v-if="dupScanState.step === 'groups'" class="scd-bottom">
        <div class="scd-summary">
          <span class="scd-sel">已选择 <b>{{ selectedFileCount }}</b> 个文件</span>
          <span class="scd-saved">可节约 {{ formatSize(savedBytes) }} 储存空间</span>
        </div>
        <div class="scd-btns">
          <ActionButton variant="cancel" size="lg" label="取消" :disabled="dupScanState.busy" @click="closeScan" />
          <ActionButton
            variant="primary"
            size="lg"
            label="合并"
            :disabled="selectedFileCount === 0 || dupScanState.busy"
            @click="enterConfirm"
          />
        </div>
      </div>
      <!-- 屏 2 底部：上一步（次，左）/ 确认合并（主，右）——次左主右 -->
      <div v-else class="scd-bottom end">
        <ActionButton variant="solid" size="lg" label="上一步" :disabled="dupScanState.busy" @click="backToGroups" />
        <ActionButton variant="primary" size="lg" label="确认合并" :disabled="dupScanState.busy" @click="confirmMerge" />
      </div>
    </template>
  </BaseModal>
</template>

