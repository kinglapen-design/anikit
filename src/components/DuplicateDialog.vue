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
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import SwitchButton from "./buttons/SwitchButton.vue";
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

// 确认 = 按各行决策执行（逐项提交，单项失败不阻塞；失败项走 FailedListDialog）
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
  <BaseModal title="重复文件提示" :width="970" framed class="dup-dialog" @close="onCancel">
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
      <!-- 底部按钮行（36 稿 + 新模态口径）：右 = 全部跟随 + 取消（次，左）/ 确认（主，右）——次左主右 -->
      <div class="dup-bottom">
        <span class="follow-label">全部跟随</span>
        <SwitchButton v-model="followAll" />
        <ActionButton variant="cancel" size="lg" label="取消" :disabled="busy" @click="onCancel" />
        <ActionButton variant="primary" size="lg" label="确认" :disabled="busy" @click="onConfirm" />
      </div>
    </template>
  </BaseModal>
</template>

<style scoped>
/* 36 稿总容器：970×878，padding 0（标题栏 / 按钮行贴边，区块内边距自控）。
   壳 = modals/BaseModal framed（2026-09-20 R2 换代）：宽 970 稿值走 width prop（border-box 含 1px 边，
   旧真 border 3px 补偿随换代废除）；framed 自带 54 高标题栏 + 底 1px 通用/五级通栏 + 关闭 X（原
   :deep(.titlebar) 覆盖删除）；class 透传落在 BaseModal 根 .bm-mask 上 → 壳样式须 :deep(.bm-modal) */
.dup-dialog :deep(.bm-modal) {
  height: min(878px, calc(100vh - 96px)); /* 878 稿值 + 窗口封顶留呼吸（旧壳 max-height 口径沿） */
  max-width: calc(100vw - 96px); /* 旧壳 max-width 口径沿 */
}
/* 区块展示：framed 内容段自承 padding 0 24（原 .content 承担，换代后归本段）；吃满剩余高，内部自滚。
   padding-right 必须保留 24（表头 / 行靠它内缩，归零会把内容冲到弹窗缘）；
   贴右缘由内层 .dup-wrap/.dup-scroll 负 margin 对承担（2026-09-13 实机走查贴右缘口径） */
.dup-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  padding: 0 var(--n-space-24);
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
/* 滚动包裹层：负 margin 对（2026-09-13 实机走查贴右缘口径）——margin-right 负扩 24 吃进
   .dup-body 的 0 24 padding 贴到弹窗缘，.dup-scroll padding-right 补回使行内容原位不动 */
.dup-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  margin-right: calc(-1 * var(--n-space-24));
}
/* 滚轮 = ScrollThumb 悬浮件（悬停显隐，主窗口统一交互，2026-08-13 定稿）；
   2026-09-13 走查后 thumb 贴弹窗右缘（原距 inset 边框 24px 口径废） */
.dup-scroll {
  flex: 1;
  min-height: 0;
  padding-right: var(--space-24);
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
/* actions 槽：底部按钮行通栏不缩（壳高被窗口封顶压时按钮行不动） */
.dup-dialog :deep(.bm-actions) {
  flex: none;
}
/* 确认 / 取消 = ActionButton lg 宽自适应 + 左右 48（BaseModal 罩层口径，2026-09-13 定稿值，
   36 稿 88×38 定宽随换代废除） */
</style>
