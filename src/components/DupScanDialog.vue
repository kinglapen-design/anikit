<script setup lang="ts">
// 扫描重复文件弹窗（35_扫描重复文件稿 + Eagle 式合并前确认，2.7 二轮 2026-08-13，PRD 03_查找文件-03）。
// 屏 1（groups，714 宽）= 35 稿结果列表：组行（复选默认全选 / 红钮移除）+ 底部计数与合并/取消；
// 屏 2（confirm，970 宽）= 合并前确认：左列组列表 + 右列保留者 radio 单选
//（默认导入最早 = 后端排序 assets[0]），底部确认合并 / 上一步（回屏 1）。
// 取消 / 关闭 = 全部不处理（纯前端清状态）；合并经 dupScanOps.confirmMerge 一次 delete_placements。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppRadio from "./AppRadio.vue";
import DupScanGroupRow from "./DupScanGroupRow.vue";
import ScrollThumb from "./ScrollThumb.vue";
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
  <AppModal
    variant="standard"
    :title="dupScanState.step === 'groups' ? '扫描重复文件' : '合并前确认'"
    class="scan-dialog"
    :class="{ confirm: dupScanState.step === 'confirm' }"
    @close="closeScan"
  >
    <!-- 屏 1：结果列表（35 稿）；悬浮滚轮悬停显隐（主窗口统一交互） -->
    <div v-if="dupScanState.step === 'groups'" class="scroll-wrap">
      <div ref="groupsBody" class="groups-body kit-scroll">
        <DupScanGroupRow
          v-for="g in dupScanState.groups"
          :key="g.hash"
          :group="g"
          @remove="removeGroup(g.hash)"
        />
      </div>
      <ScrollThumb :body="groupsBody" />
    </div>

    <!-- 屏 2：合并前确认（Eagle 式：左组列表 + 右保留者单选） -->
    <div v-else class="confirm-body">
      <div class="list-wrap">
        <aside ref="groupListEl" class="group-list kit-scroll">
          <button
            v-for="g in confirmGroups"
            :key="g.hash"
            class="group-item"
            :class="{ cur: g.hash === currentGroup?.hash }"
            @click="selectGroup(g.hash)"
          >
            <span class="group-thumb">
              <img
                v-if="g.assets[0] && thumbUrl(g.assets[0])"
                :class="{ icon: g.assets[0].thumbKind === 'formatIcon' }"
                :src="thumbUrl(g.assets[0])!"
                :alt="g.assets[0].name"
                draggable="false"
              />
              <span v-else class="ext">{{ g.assets[0]?.ext.toUpperCase() }}</span>
            </span>
            <span class="group-text">
              <span class="group-name" :title="g.assets[0]?.name">{{ g.assets[0]?.name }}</span>
              <span class="group-count">{{ g.assets.length }} 个文件</span>
            </span>
          </button>
        </aside>
        <ScrollThumb :body="groupListEl" />
      </div>
      <section v-if="currentGroup" class="keeper-wrap">
        <div ref="keeperListEl" class="keeper-list kit-scroll">
          <div
            v-for="a in currentGroup.assets"
            :key="a.assetId"
            class="keeper-item"
            :class="{ on: keeperOf(currentGroup.hash) === a.assetId }"
            @click="setKeeper(currentGroup.hash, a.assetId)"
          >
            <AppRadio
              :model-value="keeperOf(currentGroup.hash)"
              :value="a.assetId"
              @update:model-value="setKeeper(currentGroup.hash!, String($event))"
            />
            <span class="keeper-thumb">
              <img
                v-if="thumbUrl(a)"
                :class="{ icon: a.thumbKind === 'formatIcon' }"
                :src="thumbUrl(a)!"
                :alt="a.name"
                draggable="false"
              />
              <span v-else class="ext">{{ a.ext.toUpperCase() }}</span>
            </span>
            <span class="keeper-name" :title="a.name">{{ a.name }}</span>
            <span class="keeper-meta">
              <span class="size">{{ formatSize(a.size) }}</span>
              <span class="chip" :title="a.folderName">{{ a.folderName }}</span>
            </span>
          </div>
        </div>
        <ScrollThumb :body="keeperListEl" />
      </section>
    </div>

    <template #actions>
      <!-- 屏 1 底部（35 稿：62 高 / padding 12 12 12 24 / 左计数右按钮） -->
      <div v-if="dupScanState.step === 'groups'" class="scan-bottom">
        <div class="summary">
          <span class="sel">已选择 <b>{{ selectedFileCount }}</b> 个文件</span>
          <span class="saved">可节约 {{ formatSize(savedBytes) }} 储存空间</span>
        </div>
        <div class="btns">
          <AppButton
            variant="primary"
            label="合并"
            :disabled="selectedFileCount === 0 || dupScanState.busy"
            @click="enterConfirm"
          />
          <AppButton variant="panel" label="取消" :disabled="dupScanState.busy" @click="closeScan" />
        </div>
      </div>
      <!-- 屏 2 底部：确认合并（主，左）/ 上一步（次，右） -->
      <div v-else class="scan-bottom end">
        <AppButton variant="primary" label="确认合并" :disabled="dupScanState.busy" @click="confirmMerge" />
        <AppButton variant="panel" label="上一步" :disabled="dupScanState.busy" @click="backToGroups" />
      </div>
    </template>
  </AppModal>
</template>

<style scoped>
/* 35 稿壳：内容宽 714 + 真 border 3 补偿 = 717，padding 0（标题栏 / 按钮行贴边）；
   屏 2 过渡为 973（内容 970）双栏 */
.scan-dialog :deep(.app-modal) {
  width: 717px;
  padding: 0;
  gap: 0;
  transition: width 0.2s ease;
}
.scan-dialog.confirm :deep(.app-modal) {
  width: 973px;
}
/* 标题栏（35 稿）：54 高 / padding 12 12 12 24 / 底线通栏（真 border 后横线与竖框不重叠——
   边框占盒模型在内容区外侧，叠色根因已除，2026-08-13） */
.scan-dialog :deep(.titlebar) {
  position: relative;
  height: 54px;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  flex: none;
}
.scan-dialog :deep(.titlebar)::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 1px;
  background: var(--line-hover);
}
.scan-dialog :deep(.content) {
  flex: 1;
  min-height: 0;
  padding: 0;
  gap: 0;
  overflow: hidden;
  /* 壳 padding 0：归零 AppModal .content 的负 margin 对（2026-09-13 实机走查贴右缘口径），
     内层包裹（.scroll-wrap/.list-wrap/.keeper-wrap）通栏，thumb 维持贴弹窗缘 */
  margin-right: 0;
  padding-right: 0;
}
.scan-dialog :deep(.actions) {
  flex: none;
}

/* 滚动包裹层：relative 锚 ScrollThumb（悬浮滚轮悬停显隐，主窗口统一交互）。
   真 border（2026-08-13 用户定）后弹窗边框占盒模型——滚轮贴内容区右缘与边框贴邻不重叠，
   无需补画右缘线（原 box-shadow inset 时代的 ::after 补线已撤）；左列分隔线仍在内容区内，
   滚轮会压，由 .list-wrap::after 上层补画 */
.scroll-wrap,
.list-wrap,
.keeper-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.scroll-wrap {
  flex: 1;
}
.list-wrap {
  width: 250px;
  flex: none;
}
.keeper-wrap {
  flex: 1;
  min-width: 0;
}
/* 左列：与右列的 1px 分隔线（暗-悬停）补画在上层（z-index 11 > 滚轮 10，事件穿透） */
.list-wrap::after {
  content: "";
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--line-hover);
  z-index: 11;
  pointer-events: none;
}

/* —— 屏 1：内容自适应 + 上限自滚（原生条隐藏，滚轮 = ScrollThumb 悬浮件）—— */
.groups-body {
  max-height: calc(100vh - 280px);
}

/* —— 屏 2：双栏 —— */
.confirm-body {
  display: flex;
  height: min(560px, calc(100vh - 300px));
}
/* 左列：250 组列表；容器 padding 12 全边（2026-08-13 用户定）+ gap 4 + item 圆角 6；
   与右列的 1px 分隔线由 .list-wrap::after 上层补画（滚轮贴边不压线），此处不再自画；
   滚轮 = ScrollThumb 悬浮件（不占布局，边距即纯 padding，无需凑 gutter） */
.group-list {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-12);
}
/* 组列表项：56 高 = 40×40 缩略图 + 名称 / N 个文件竖排，padding 四周 8（2026-08-13 用户定）；
   hover 叠 暗-悬停 罩层；当前组 = 亮-选中 罩层 + 暗-悬停 1px 线条（树 item 强度阶梯同口径） */
.group-item {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  height: 56px;
  padding: var(--space-8);
  border-radius: var(--radius-6);
  flex: none;
  text-align: left;
  cursor: pointer;
  transition: background 0.15s ease;
}
.group-item:hover {
  background: var(--line-hover);
}
/* 当前组：与目录树 item 选中同款（2026-08-13 用户定）——亮-选中罩层 + 1.5px 文字常态内阴影 */
.group-item.cur {
  background: var(--line-active);
  box-shadow: inset 0 0 0 1.5px var(--text-normal);
}
.group-thumb {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-6);
  background: var(--c3);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
.group-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: var(--radius-6); /* 双保险：容器 overflow 裁剪之外 img 显式圆角（2026-08-13 走查核对 = 6） */
}
.group-thumb img.icon {
  object-fit: contain;
}
.group-thumb .ext {
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--text-label);
}
.group-text {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}
.group-name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.group-count {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
}
/* 右列：保留者单选列表（item = 35 稿卡片横条化：缩略图 + 名称 + 大小 + 所在文件夹标签）；
   滚轮 = ScrollThumb 悬浮件，padding 恢复纯 24（无 gutter 补偿） */
.keeper-list {
  flex: 1;
  min-height: 0;
  padding: var(--space-16) var(--space-24);
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
.keeper-item {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  height: 52px;
  padding: var(--space-6) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c2); /* 弹窗内 item 底色与二级窗口一致（2026-08-13 用户定：原 C3 改 C2） */
  box-shadow: inset 0 0 0 1px var(--line-hover);
  flex: none;
  cursor: pointer;
  transition: background 0.15s ease;
}
/* hover 叠 暗-悬停 罩层；选中行（.on）悬停不再变色——选中态由品牌色描边表达，
   叠罩会稀释描边观感（2026-08-13 走查） */
.keeper-item:not(.on):hover {
  background: var(--line-hover);
}
/* 选中行 = 品牌色描边（选中框统一 --brand-border，1.2 走查口径） */
.keeper-item.on {
  box-shadow: inset 0 0 0 1.5px var(--brand-border);
}
.keeper-thumb {
  width: 64px;
  height: 36px;
  border-radius: var(--radius-6);
  background: var(--c2);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
.keeper-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: var(--radius-6);
}
.keeper-thumb img.icon {
  object-fit: contain;
}
.keeper-thumb .ext {
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--text-label);
}
.keeper-name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.keeper-meta {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  flex: none;
}
.keeper-meta .size {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
  white-space: nowrap;
}
/* 所在文件夹标签（36 稿 chip 同款）：C4 底 + 1px 暗-悬停 / 圆角 6 / padding 4 8 / 标签档 */
.chip {
  max-width: 140px;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 底部（35 稿）：62 高 / padding 12 12 12 24；flex: 1 撑满 actions 通栏
   （不撑满上边框线会断在内容左缘，DuplicateDialog 走查同口径）；顶线 ::after 通栏 */
.scan-bottom {
  position: relative;
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  height: 62px;
}
.scan-bottom::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  height: 1px;
  background: var(--line-hover);
}
.scan-bottom.end {
  justify-content: flex-end;
}
.summary {
  display: flex;
  align-items: center;
  gap: var(--space-8);
}
/* 已选择 N 个文件（35 稿：标题档 16；N 品牌色） */
.sel {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-normal);
}
.sel b {
  color: var(--brand);
  font-weight: var(--text-title-weight);
}
.saved {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
}
.btns {
  display: flex;
  gap: var(--space-12);
}
/* 合并 / 取消 88 定宽（35 稿：88×38，与 36 稿同口径） */
.scan-bottom :deep(.app-btn) {
  width: 88px;
}
</style>
