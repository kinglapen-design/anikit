<script setup lang="ts">
// 重复文件提示（36_重复文件提示稿，2026-08-13 全量重写）：导入完成后 hash 冲突统一询问。
// 行 = 左右对照卡片 + radio 三选一（作为新链接导入，默认 / 作为新文件导入 / 用已存在文件导入）
// + 行尾"跳过此文件"开关（可逆，跳过行 30% 压暗）+「全部跟随」开关（默认开：改一行决策全部行跟随）。
// 决策与落点不设限（2026-08-13 用户定：不替用户做整理决定——落点未分类同样三选一恒出，
// 原"候车区收窄只留作为新文件导入"废除）。
// 2026-08-13 用户定：内嵌批量重命名属过度设计，整个废除——"导入就是导入，只选择怎么导入；
// 要命名导进来再说"（批量重命名归独立弹窗，阶段三，37 稿）。
// 确认 = 按各行决策执行；取消 / 关闭（含遮罩点击）= 全部跳过——源文件从未被动过，纯前端清队列即可。
import { computed, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppSwitch from "./AppSwitch.vue";
import DuplicateRow, { type DupDecision } from "./DuplicateRow.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { useContentStore, type ConflictItem, type ConflictResolution } from "../stores/content";

const content = useContentStore();
const conflicts = computed(() => content.importConflicts ?? []);
const dupScrollEl = ref<HTMLElement>();

// 默认决策：作为新链接导入（36 稿默认，不设落点收窄——2026-08-13 用户定案）
function defaultDecision(_c: ConflictItem): DupDecision {
  return { action: "link", skipped: false };
}

// 每行决策；弹窗挂载时队列已就位，初始化一次即可
const rows = ref<DupDecision[]>(conflicts.value.map(defaultDecision));
const followAll = ref(true);
const busy = ref(false);

// 全部跟随：开时改任一行 radio / 跳过 → 全部行跟随；关掉后逐行独立
function onRowChange(index: number, patch: Partial<DupDecision>): void {
  if (followAll.value) {
    rows.value = rows.value.map((r) => ({ ...r, ...patch }));
  } else {
    const cur = rows.value[index];
    if (cur) rows.value[index] = { ...cur, ...patch };
  }
}

function buildResolutions(): ConflictResolution[] {
  return conflicts.value.map((c, i) => {
    const d = rows.value[i] ?? defaultDecision(c);
    return {
      sourcePath: c.sourcePath,
      existingAssetId: c.existingAssetId,
      targetFolder: c.targetFolder,
      action: d.skipped ? ("skip" as const) : d.action,
      sequence: c.sequence ?? null, // 序列帧组信息原样回传（copy 重新打包用，4.1）
      sourceThumbPath: c.sourceThumbPath ?? null, // 预览缓存原样回传（决策完成后后端删除，4.2）
      importedFrom: c.importedFrom ?? null, // 批次渠道原样回传（copy 新资产继承，2026-08-13）
    };
  });
}

// 确认 = 按各行决策执行（逐项提交，单项失败不阻塞；失败项走 ImportResultDialog）
async function onConfirm(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const report = await content.resolveConflicts(buildResolutions());
    if (report.failed.length > 0) content.importFailed = report.failed;
    content.importConflicts = null;
    await content.refresh();
  } finally {
    busy.value = false;
  }
}

// 取消 / 关闭 = 全部跳过（源文件保留；临时预览缓存 cache/tmp 会话级自清）
function onCancel(): void {
  if (busy.value) return;
  content.importConflicts = null;
}
</script>

<template>
  <AppModal variant="standard" title="重复文件提示" class="dup-dialog" @close="onCancel">
    <div class="dup-body">
      <!-- 表头双列（36 稿：46 高 + 下分隔线，标题档文字-标签色） -->
      <div class="dup-head">
        <span class="col">已存在文件</span>
        <span class="col">新的文件</span>
      </div>
      <div class="dup-wrap">
        <div ref="dupScrollEl" class="dup-scroll kit-scroll">
          <DuplicateRow
            v-for="(c, i) in conflicts"
            :key="c.sourcePath"
            :item="c"
            :decision="rows[i] ?? defaultDecision(c)"
            @change="onRowChange(i, $event)"
          />
        </div>
        <ScrollThumb :body="dupScrollEl" />
      </div>
    </div>
    <template #actions>
      <!-- 底部按钮行（36 稿）：右 = 全部跟随 + 确认（主，左）/ 取消（次，右） -->
      <div class="dup-bottom">
        <span class="follow-label">全部跟随</span>
        <AppSwitch v-model="followAll" />
        <AppButton variant="primary" label="确认" :disabled="busy" @click="onConfirm" />
        <AppButton variant="panel" label="取消" :disabled="busy" @click="onCancel" />
      </div>
    </template>
  </AppModal>
</template>

<style scoped>
/* 36 稿总容器：970×878，padding 0（标题栏 / 按钮行贴边，区块内边距自控） */
.dup-dialog :deep(.app-modal) {
  width: 970px;
  height: 878px;
  padding: 0;
  gap: 0;
}
/* 标题栏（36 稿）：54 高 / padding 12 12 12 24 / 下分隔线；标题 = 文字/常态 */
.dup-dialog :deep(.titlebar) {
  height: 54px;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  box-shadow: inset 0 -1px 0 0 var(--line-hover);
  flex: none;
}
.dup-dialog :deep(.title) {
  color: var(--text-normal);
}
/* 区块展示：吃满剩余高，padding 0 24，内部自滚 */
.dup-dialog :deep(.content) {
  flex: 1;
  min-height: 0;
  padding: 0 var(--space-24);
  gap: 0;
  overflow: hidden;
}
/* actions 槽：底部按钮行通栏 */
.dup-dialog :deep(.actions) {
  flex: none;
}
.dup-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 表头（36 稿）：46 高 / padding 12 0 / gap 24 / 下分隔线 */
.dup-head {
  display: flex;
  gap: var(--space-24);
  padding: var(--space-12) 0;
  box-shadow: inset 0 -1px 0 0 var(--line-hover);
  flex: none;
}
.dup-head .col {
  flex: 1;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-label);
}
.dup-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.dup-scroll {
  flex: 1;
  min-height: 0;
  /* 滚轮 = ScrollThumb 悬浮件（悬停显隐，主窗口统一交互，2026-08-13 定稿） */
}
/* 底部按钮行（36 稿）：62 高 / 上分隔线 / padding 12 12 12 24，右对齐；
   flex: 1 占满 actions 主轴（row flex 项宽默认内容宽——不撑满上边框线会断在内容左缘，2026-08-13 走查） */
.dup-bottom {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  height: 62px;
  box-shadow: inset 0 1px 0 0 var(--line-hover);
}
/* 全部跟随标签（36 稿：标题档 / 文字-常态） */
.follow-label {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-normal);
}
/* 确认 / 取消 88 定宽（36 稿：88×38） */
.dup-bottom :deep(.app-btn) {
  width: 88px;
}
</style>
