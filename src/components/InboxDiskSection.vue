<script setup lang="ts">
// 收件箱"工程文件"磁盘实况区块（4.4，PRD 05-01 2026-07-25 语义重定）：
// 磁盘为事实来源——不建档不写库，热刷新由 content.inboxDisk 全量替换驱动；
// 白名单以外不自动导入，右键手动"导入"= 移走入库（剪切语义；先弹确认明示"源文件将从收件箱移走"，
// PRD 05-01 硬性要求，2.6；确认后进度条 / 重复提示流与常规导入一致）。
// 占用条目：仅实况展示（"被占用"标记），导入 / 重命名禁用，后端同口径再拦一道。
// 文件图标：优先系统格式图标缓存（iconPath，details 事件补齐），无缓存回退扩展名角标；
// 目录恒 folder-closed。占用标记以 details 合并后为准（快层目录 occupied=false 是过渡态）。
// 卡片样式 2026-08-13 同步素材卡片 14 稿（整卡圆角 6 + ::after 状态边框 + 图标区 C2 /
// 简介区 C3 名称单行左对齐）；右键 @contextmenu.stop 拦冒泡（gridDispatch 空白新增菜单会覆盖，同日修复）。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import SectionHeader from "./SectionHeader.vue";
import { formatBadgeColor } from "../lib/formatBadge";
import { openContextMenu } from "../lib/contextMenus";
import { inboxDiskSpec } from "../lib/menuSpecs/inboxDisk";
import { deleteInboxEntry, confirmImportInboxEntry, openInboxEntry, renameInboxEntry } from "../lib/inboxOps";
import { useInlineRename } from "../lib/useInlineRename";
import { useContentStore, type InboxDiskEntry } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const content = useContentStore();
const selection = useSelectionStore();
const diskOpen = ref(true);

// 操作错误（打开 / 重命名 / 删除 / 导入占用拦截）：后端中文人话原样展示，5 秒自消
const errorMsg = ref("");
let errorTimer: ReturnType<typeof setTimeout> | undefined;
function showError(e: unknown): void {
  errorMsg.value = String(e);
  clearTimeout(errorTimer);
  errorTimer = setTimeout(() => (errorMsg.value = ""), 5000);
}

// 扩展名角标：复用格式角标色映射（静态图组无底色时回退 C4——实况区无缩略图，角标必须可见）
function extOf(e: InboxDiskEntry): string {
  const i = e.name.lastIndexOf(".");
  return i > 0 ? e.name.slice(i + 1) : "";
}
function extLabel(e: InboxDiskEntry): string {
  return extOf(e).toUpperCase();
}
function badgeColor(e: InboxDiskEntry): string {
  return formatBadgeColor(extOf(e)) ?? "var(--c4)";
}
function sizeLabel(e: InboxDiskEntry): string {
  const kb = e.size / 1024;
  if (kb < 1024) return `${Math.max(1, Math.round(kb))} KB`;
  return `${(kb / 1024).toFixed(1)} MB`;
}

// 选中：与素材 / 文件夹选中互斥（content.selectedInboxName 第三态；
// selection.clear() 内部已清素材与文件夹两域，2026-07-28 复验反馈 5）
function select(e: InboxDiskEntry): void {
  content.selectedInboxName = e.name;
  selection.clear();
}

// 右键菜单（1.3 归并全局宿主 ContextMenu）：打开 / 重命名 / 删除 / 导入（占用禁用导入与重命名）
function openMenu(e: MouseEvent, entry: InboxDiskEntry): void {
  select(entry);
  openContextMenu(e, inboxDiskSpec(entry, {
    onOpen: () => void openEntry(entry),
    onRename: () => startRename(entry),
    onDelete: () => void deleteEntry(entry),
    // 2.6：手动导入先弹确认（PRD 05-01 硬性：入口必须明示"源文件将从收件箱移走"），确认后执行
    onImport: () => confirmImportInboxEntry(entry.name, showError),
  }));
}

async function openEntry(entry: InboxDiskEntry): Promise<void> {
  try {
    await openInboxEntry(entry.name);
  } catch (err) {
    showError(err);
  }
}

async function deleteEntry(entry: InboxDiskEntry): Promise<void> {
  try {
    await deleteInboxEntry(entry.name);
    if (content.selectedInboxName === entry.name) content.selectedInboxName = "";
  } catch (err) {
    showError(err);
  }
}

// 行内重命名（2026-08-13 接入 useInlineRename 统一语义，原散装实现废除——blur 取消离群
// 导致"输入后返回旧名"、全选含后缀、无双击）：selectStem 选区只到词干（扩展名不入选）；
// Enter / 失焦恒提交，Esc 取消；双击名称进入
const renaming = ref<string | null>(null); // 目标条目名（当前原名）
const {
  draft: renameDraft,
  bindInput: bindRenameInput,
  commit: commitRename,
  onEsc: onRenameEsc,
} = useInlineRename({
  active: () => renaming.value !== null,
  initial: () => renaming.value ?? "",
  selectStem: true,
  onCommit: (name) => {
    const old = renaming.value;
    renaming.value = null;
    if (!old || !name || name === old) return;
    void doRename(old, name);
  },
  onCancel: () => {
    renaming.value = null;
  },
});
function startRename(entry: InboxDiskEntry): void {
  if (entry.occupied) return;
  renaming.value = entry.name;
}
async function doRename(oldName: string, newName: string): Promise<void> {
  // 扩展名不动（Eagle 口径：改名只编词干）——draft 被改成无后缀/异后缀时补回原扩展名
  const dot = oldName.lastIndexOf(".");
  const ext = dot > 0 ? oldName.slice(dot) : "";
  if (ext && !newName.toLowerCase().endsWith(ext.toLowerCase())) newName += ext;
  try {
    await renameInboxEntry(oldName, newName);
    if (content.selectedInboxName === oldName) content.selectedInboxName = newName;
  } catch (err) {
    showError(err);
  }
}

const entries = computed(() => content.inboxDisk);
</script>

<template>
  <section class="disk-block">
    <SectionHeader title="工程文件" :count="entries.length" v-model:open="diskOpen" />
    <template v-if="diskOpen">
      <!-- 固定提醒（4.4 定案文案）：白名单以外一律不自动导入 -->
      <p class="notice">工程文件不支持自动导入，请手动导入</p>
      <p v-if="errorMsg" class="error-line">{{ errorMsg }}</p>
      <div class="flow">
        <div
          v-for="e in entries"
          :key="e.name"
          class="disk-card"
          :class="{ selected: content.selectedInboxName === e.name, renaming: renaming === e.name }"
          @click="select(e)"
          @contextmenu.prevent.stop="openMenu($event, e)"
        >
          <div class="icon-wrap">
            <AppIcon v-if="e.isDir" name="folder-closed" :size="40" class="folder-icon" />
            <img
              v-else-if="e.iconPath"
              :src="convertFileSrc(e.iconPath)"
              class="format-icon"
              :alt="extLabel(e)"
              draggable="false"
            />
            <span v-else class="ext-badge" :style="{ background: badgeColor(e) }">{{ extLabel(e) }}</span>
            <span v-if="e.occupied" class="occupied-tag">被占用</span>
          </div>
          <div class="texts">
            <input
              v-if="renaming === e.name"
              :ref="bindRenameInput"
              v-model="renameDraft"
              class="rename-input"
              @keydown.enter.prevent="commitRename"
              @keydown.esc.prevent="onRenameEsc"
              @blur="commitRename"
              @click.stop
            />
            <span v-else class="title" v-tip="e.name" @dblclick.stop="startRename(e)">{{ e.name }}</span>
            <span class="meta">{{ e.isDir ? "文件夹" : sizeLabel(e) }}</span>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
/* 区块容器：与文件夹区块同口径（figma 42：padding 0 由 asset-section 承担边距 / 标题↔卡片 gap 12；
   右 16 对齐网格 PAD.right，不与贴边滚动条重叠） */
.disk-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding-right: var(--space-16);
}
/* 固定提醒与操作错误：标签档（text-label / caption），错误用语义色 --error */
.notice {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
}
.error-line {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--error);
  word-break: break-all;
}
/* 卡片列表：与素材网格视图同宽（260，2026-08-13 用户：120 太窄名称五字就截断） */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  column-gap: var(--space-16);
  row-gap: var(--space-16);
}
/* 卡片（同步素材卡片 14 稿，2026-08-13）：整卡圆角 6 / 状态边框走 ::after 伪元素
   （常态 1px 暗-悬停；hover 1.5px 实色 line-card-hover；选中 1.5px 品牌色-边框线）——
   图标区 C2 / 简介区 C3 整片不透明底会盖住父级 inset 阴影，边框必须在最上层伪元素 */
.disk-card {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  border-radius: var(--radius-6);
  cursor: default;
}
.disk-card::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1px solid var(--line-hover);
  border-radius: var(--radius-6);
  pointer-events: none;
  transition: border-color 0.12s ease;
}
.disk-card:hover::after {
  border: 1.5px solid var(--line-card-hover);
}
.disk-card.selected::after,
.disk-card.renaming::after {
  border: 1.5px solid var(--brand-border);
}
/* 图标区（14 稿缩略图区口径）：C2 底 / 圆角上沿；
   比例按网格视图格子 260×220（2026-08-13 用户定）：简介区恒 57，图标区 260:163 */
.icon-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 260 / 163;
  background: var(--c2);
  border-radius: var(--radius-6) var(--radius-6) 0 0;
  overflow: hidden;
}
.folder-icon {
  color: var(--text-label);
}
/* 系统格式图标：缓存源图 256px，最大边 = 正方形图标位 50%（与素材卡片 / 列表行同一口径，
   shell 小一点给内容缩略图让视觉噪音，2026-07-25 用户定） */
.format-icon {
  max-width: 50%;
  max-height: 50%;
  object-fit: contain;
}
/* 扩展名角标文本：实底白字（格式角标 7 组实色映射，01 样式 token 01-6）；
   静态图组无底色时回退 C4——实况区无缩略图，角标必须可见 */
.ext-badge {
  padding: var(--space-2) var(--space-6);
  border-radius: var(--radius-4);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-highlight);
}
/* 被占用标记：右上小标签（警告色——占用 = 暂不可操作态） */
.occupied-tag {
  position: absolute;
  top: var(--space-4);
  right: var(--space-4);
  padding: 0 var(--space-4);
  border-radius: var(--radius-4);
  background: var(--c3);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--warning);
}
/* 简介区（14 稿）：C3 / 上分割线（inset 不占位）/ 圆角下沿 / padding 8 8 8 12 / 间距 4 / 恒 57 高 */
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  height: 57px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
  background: var(--c3);
  box-shadow: inset 0 1px 0 0 var(--line-hover);
  border-radius: 0 0 var(--radius-6) var(--radius-6);
}
/* 名称：正文 14/20 高亮，单行左对齐截断（14 稿口径，旧两行居中作废） */
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 行内重命名：不额外给输入框（2026-08-13 用户定，同素材卡片透明族）——
   直接用 base.css .rename-input 共享基座（透明底无描边），整卡 ::after 品牌色边框提示重命名态 */
</style>
