<script setup lang="ts">
// 重复文件提示（36_重复文件提示稿，2026-08-13 全量重写）：导入完成后 hash 冲突统一询问。
// 行 = 左右对照卡片 + radio 三选一（作为新链接导入，默认 / 作为新文件导入 / 用已存在文件导入）
// + 行尾"跳过此文件"开关（可逆，跳过行 30% 压暗）；底部内嵌批量重命名（37 稿：序号 / 替换命名）
// + 「全部跟随」开关（默认开：改一行决策全部行跟随，关掉逐行精调；重命名规则为全局面板不参与跟随）。
// 候车区收窄（2026-08-13 用户定）：落点 = 未分类（root）时只留"作为新文件导入"（默认选中）——
// 候车区不是分类位置，建链接 / 移入既有资产对候车区无意义。
// 确认 = 按各行决策执行（改名只影响导入后显示名，不影响 hash 判定）；
// 取消 / 关闭（含遮罩点击）= 全部跳过——源文件从未被动过，纯前端清队列即可。
import { computed, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import AppSegmented from "./AppSegmented.vue";
import AppSwitch from "./AppSwitch.vue";
import DuplicateRow, { type DupDecision } from "./DuplicateRow.vue";
import DuplicateRenameBar from "./DuplicateRenameBar.vue";
import { useContentStore, type ConflictItem, type ConflictResolution } from "../stores/content";

const content = useContentStore();
const conflicts = computed(() => content.importConflicts ?? []);

// 默认决策：候车区（落点 root）= 作为新文件导入；否则 = 作为新链接导入
function defaultDecision(c: ConflictItem): DupDecision {
  return { action: c.targetFolder === "root" ? "copy" : "link", skipped: false };
}

// 每行决策；弹窗挂载时队列已就位，初始化一次即可
const rows = ref<DupDecision[]>(conflicts.value.map(defaultDecision));
const followAll = ref(true);
const busy = ref(false);

// 批量重命名（37 稿：分段选择器单显一行输入）
const renameMode = ref<"sequence" | "replace">("sequence");
const renameName = ref("");
const renameStart = ref("1");
const renameFind = ref("");
const renameReplace = ref("");

function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}
function stemOf(name: string): string {
  return name.replace(/\.[^.]+$/, "");
}

// 序号命名：名称-序号，位宽自动 = 最大编号（起始值 + 行数 - 1）的位数
function seqName(base: string, index: number, ext: string): string {
  const start = Number.parseInt(renameStart.value, 10) || 1;
  const width = String(start + rows.value.length - 1).length;
  const num = String(start + index).padStart(width, "0");
  return `${base}-${num}${ext ? `.${ext}` : ""}`;
}

// 行新名（规则只作用词干，扩展名保留；规则不产生变化 = null 沿用源文件名）。
// 序号按行序编号（含跳过行——切换跳过不重排他行预览，稳定优先）
function newNameFor(item: ConflictItem, index: number): string | null {
  const ext = extOf(item.sourceName);
  const stem = stemOf(item.sourceName);
  let next: string | null = null;
  if (renameMode.value === "sequence") {
    const base = renameName.value.trim();
    if (base) next = seqName(base, index, ext);
  } else {
    const find = renameFind.value;
    if (find && stem.includes(find)) {
      const replaced = stem.split(find).join(renameReplace.value).trim();
      if (replaced) next = `${replaced}${ext ? `.${ext}` : ""}`;
    }
  }
  return next && next !== item.sourceName ? next : null;
}

// 新名预览：仅 copy / link 决策会按新名导入；useExisting / 跳过行不预览改名
const previews = computed(() =>
  conflicts.value.map((c, i) => {
    const d = rows.value[i];
    return d && !d.skipped && d.action !== "useExisting" ? newNameFor(c, i) : null;
  }),
);

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
      newName: d.skipped || d.action === "useExisting" ? null : newNameFor(c, i),
      sequence: c.sequence ?? null, // 序列帧组信息原样回传（copy 重新打包用，4.1）
      sourceThumbPath: c.sourceThumbPath ?? null, // 预览缓存原样回传（决策完成后后端删除，4.2）
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
      <div class="dup-scroll">
        <DuplicateRow
          v-for="(c, i) in conflicts"
          :key="c.sourcePath"
          :item="c"
          :decision="rows[i] ?? defaultDecision(c)"
          :new-name="previews[i] ?? null"
          :waiting-area="c.targetFolder === 'root'"
          @change="onRowChange(i, $event)"
        />
      </div>
    </div>
    <template #actions>
      <!-- 批量重命名输入行（37 稿：随分段模式单显一行） -->
      <DuplicateRenameBar
        v-model:name="renameName"
        v-model:start="renameStart"
        v-model:find="renameFind"
        v-model:replace="renameReplace"
        :mode="renameMode"
      />
      <!-- 底部按钮行（36 稿）：左 = 分段选择器；右 = 全部跟随 + 确认（主，左）/ 取消（次，右） -->
      <div class="dup-bottom">
        <AppSegmented
          v-model="renameMode"
          :options="[
            { value: 'sequence', label: '序号命名' },
            { value: 'replace', label: '替换命名' },
          ]"
        />
        <div class="bottom-right">
          <span class="follow-label">全部跟随</span>
          <AppSwitch v-model="followAll" />
          <AppButton variant="primary" label="确认" :disabled="busy" @click="onConfirm" />
          <AppButton variant="panel" label="取消" :disabled="busy" @click="onCancel" />
        </div>
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
/* actions 槽改为竖排容器：重命名输入行 + 底部按钮行 */
.dup-dialog :deep(.actions) {
  flex-direction: column;
  align-items: stretch;
  justify-content: flex-start;
  gap: 0;
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
.dup-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}
/* 底部按钮行（36 稿）：62 高 / 上分隔线 / padding 12 12 12 24 */
.dup-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  height: 62px;
  box-shadow: inset 0 1px 0 0 var(--line-hover);
}
.bottom-right {
  display: flex;
  align-items: center;
  gap: var(--space-12);
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
