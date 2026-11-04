<script setup lang="ts">
// 收件箱"工程文件"磁盘实况区块（4.4，PRD 05-01 2026-07-25 语义重定）：
// 磁盘为事实来源——不建档不写库，热刷新由 content.inboxDisk 全量替换驱动；
// 白名单以外不自动导入，右键手动"导入"= 单个 / 批量移走入库（剪切语义；一次确认明示
// "源文件将从收件箱移走"，确认后进度条 / 重复提示流与常规导入一致）。
// 占用条目：仅实况展示（"被占用"标记），导入 / 重命名禁用，后端同口径再拦一道。
// 文件图标：优先系统格式图标缓存（iconPath，details 事件补齐），无缓存则仅左上格式角标；
// 目录恒 folder-closed 居中（存量元素保旧 40 档，新稿未画）。占用标记以 details 合并后为准
//（快层目录 occupied=false 是过渡态）。
// 卡片 2026-09-08 换代 = cards/FileCard square 变体（§16 稿注：shell 缩略图 / 自动导入未导入 =
// 正方形 + 缩略图 50% 居中；格式角标统一 FormatBadge 铬，不再按格式设色——旧 14 稿散装样式
//（C2 图标区 + C3 简介区 + 实色扩展名角标）废除）；重命名输入态归 FileCard 内建（naming prop）。
// 区块标题同日换代 = items/SectionTitle block（旧 SectionHeader 废除：计数「- N」改「/ N」三段、
// 标题次要文字-加粗 12/16 亮色/四级、展开钮 hover 才现身——与左栏树容器区块标题统一）；
// 提示行 = 正文档 13/18 亮色/三级（空文件夹稿「提示文字」档，旧 caption/label 旧 token 废除）。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import FileCard from "./cards/FileCard.vue";
import SectionTitle from "./items/SectionTitle.vue";
import { dragView } from "../lib/dragOperation";
import { openContextMenu } from "../lib/contextMenus";
import { inboxDiskSpec } from "../lib/menuSpecs/inboxDisk";
import { deleteInboxEntry, confirmImportInboxEntries, openInboxEntry, renameInboxEntry } from "../lib/inboxOps";
import { useContentStore, type InboxDiskEntry } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";

const content = useContentStore();
const selection = useSelectionStore();
const diskOpen = ref(true);
// 折叠切换：SectionTitle 守卫在调用方（§20——dragView.suppressClick 吞拖拽后紧随 click）
function onFold(): void {
  if (dragView.suppressClick) return;
  diskOpen.value = !diskOpen.value;
}

// 操作错误（打开 / 重命名 / 删除 / 导入占用拦截）：后端中文人话原样展示，5 秒自消
const errorMsg = ref("");
let errorTimer: ReturnType<typeof setTimeout> | undefined;
function showError(e: unknown): void {
  errorMsg.value = String(e);
  clearTimeout(errorTimer);
  errorTimer = setTimeout(() => (errorMsg.value = ""), 5000);
}

// 格式角标 = 扩展名大写（FormatBadge 统一铬，§16；目录 / 无后缀文件不出）
function extOf(e: InboxDiskEntry): string {
  const i = e.name.lastIndexOf(".");
  return i > 0 ? e.name.slice(i + 1) : "";
}
function extLabel(e: InboxDiskEntry): string {
  return extOf(e).toUpperCase();
}
function sizeLabel(e: InboxDiskEntry): string {
  const kb = e.size / 1024;
  if (kb < 1024) return `${Math.max(1, Math.round(kb))} KB`;
  return `${(kb / 1024).toFixed(1)} MB`;
}

// 选中：与素材 / 文件夹选中互斥；Ctrl 逐项切换、Shift 范围选与素材卡同口径
function select(e: InboxDiskEntry, mouse?: MouseEvent): void {
  if (usePickerStore().active) return; // 选择模式（6.2，白名单收窄）：收件箱条目不可选（非库内资产）
  selection.clickInbox(
    e.name,
    { ctrl: mouse?.ctrlKey || mouse?.metaKey, shift: mouse?.shiftKey },
    entries.value.map((entry) => entry.name),
  );
}

// 右键已选中项保留整组；未选中项先单选。批量导入一次确认、一次任务，任一项占用则禁用
function openMenu(e: MouseEvent, entry: InboxDiskEntry): void {
  if (usePickerStore().active) return; // 选择模式（6.2，白名单收窄）：收件箱菜单全是编辑类，不出
  if (!selection.selectedInbox.has(entry.name)) select(entry);
  const selectedEntries = entries.value.filter((item) => selection.selectedInbox.has(item.name));
  const names = selectedEntries.map((item) => item.name);
  openContextMenu(e, inboxDiskSpec(entry, {
    count: names.length,
    occupied: selectedEntries.some((item) => item.occupied),
  }, {
    onOpen: () => void openEntry(entry),
    onRename: () => startRename(entry),
    onDelete: () => void deleteEntry(entry),
    onImport: () => confirmImportInboxEntries(names, showError),
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
    selection.selectedInbox.delete(entry.name);
    if (selection.inboxAnchor === entry.name) selection.inboxAnchor = null;
  } catch (err) {
    showError(err);
  }
}

// 行内重命名：输入态在 FileCard（naming prop，useInlineRename 内建——selectStem 词干选区 /
// Enter·失焦提交 / Esc 取消，速查 §4 统一口径），本区只管目标条目与落命令；
// 双击名称（name-edit）或菜单进命名态，占用条目禁重命名
const renaming = ref<string | null>(null); // 目标条目名（当前原名）
function startRename(entry: InboxDiskEntry): void {
  if (entry.occupied) return;
  renaming.value = entry.name;
}
function onRenameCommit(entry: InboxDiskEntry, name: string): void {
  renaming.value = null;
  if (!name || name === entry.name) return;
  // 扩展名不动（Eagle 口径：改名只编词干）——draft 被改成无后缀/异后缀时补回原扩展名
  const dot = entry.name.lastIndexOf(".");
  const ext = dot > 0 ? entry.name.slice(dot) : "";
  if (ext && !name.toLowerCase().endsWith(ext.toLowerCase())) name += ext;
  void doRename(entry.name, name);
}
async function doRename(oldName: string, newName: string): Promise<void> {
  try {
    await renameInboxEntry(oldName, newName);
    if (selection.selectedInbox.delete(oldName)) selection.selectedInbox.add(newName);
    if (selection.inboxAnchor === oldName) selection.inboxAnchor = newName;
  } catch (err) {
    showError(err);
  }
}

const entries = computed(() => content.inboxDisk);
</script>

<template>
  <section class="disk-block">
    <SectionTitle
      variant="block"
      title="工程文件"
      :count="entries.length"
      :expanded="diskOpen"
      :actions="false"
      hot-zone="lead"
      @toggle="onFold"
    />
    <template v-if="diskOpen">
      <!-- 固定提醒（4.4 定案文案）：白名单以外一律不自动导入 -->
      <p class="notice">工程文件不支持自动导入，请手动导入</p>
      <p v-if="errorMsg" class="error-line">{{ errorMsg }}</p>
      <div class="flow">
        <div
          v-for="e in entries"
          :key="e.name"
          class="disk-card"
          @click="select(e, $event)"
          @contextmenu.prevent.stop="openMenu($event, e)"
        >
          <FileCard
            :name="e.name"
            :info="e.isDir ? '文件夹' : sizeLabel(e)"
            :badge="e.isDir ? '' : extLabel(e)"
            :thumb-url="e.iconPath ? convertFileSrc(e.iconPath) : ''"
            square
            :selected="selection.selectedInbox.has(e.name)"
            :naming="renaming === e.name"
            @name-edit="startRename(e)"
            @name-commit="onRenameCommit(e, $event)"
            @name-cancel="renaming = null"
          >
            <AppIcon v-if="e.isDir" name="folder-closed" :size="40" class="dir-icon" />
          </FileCard>
          <span v-if="e.occupied" class="occupied-tag">被占用</span>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
/* 区块容器：与文件夹区块同口径（标题↔卡片 gap 12；
   右 12 对齐网格 PAD.right——卡片右缘距容器 12 不被裁，7.1 内容容器轮修订） */
.disk-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding-right: var(--n-space-12);
}
/* 固定提醒与操作错误：正文档 13/18（空文件夹稿「提示文字」档，2026-09-08 换代——
   旧 caption/label 旧 token 废除），提醒 = 亮色/三级，错误用语义色 --n-danger */
.notice {
  margin: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.error-line {
  margin: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-danger);
  word-break: break-all;
}
/* 卡片列表：与素材网格视图同宽（260，2026-08-13 用户：120 太窄名称五字就截断）；
   square 变体 = 正方形卡（§16 稿注：自动导入未导入正方形比例），行高随列宽 */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  column-gap: var(--n-space-16);
  row-gap: var(--n-space-16);
}
/* 卡壳：被占用标记定位锚；常态描边 / 选中环 / hover 位移 -4 全归 FileCard（cards.css kc 族） */
.disk-card {
  position: relative;
}
/* 目录条目：folder-closed 整卡居中（与 shell 缩略图同位 = 50% 居中档；40 尺寸保旧，
   新稿未画——同 AssetCard 笔记角标「存量元素保旧」先例） */
.dir-icon {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  color: var(--n-l3);
}
/* 被占用标记：保旧视觉旧位（右上各 4，c3 底警告字——新稿未画，同笔记角标先例）；
   壳层附件随卡芯 hover 位移 -4px（kc-media 250ms，AssetCard 位置徽章同参数跟随模式） */
.occupied-tag {
  position: absolute;
  top: var(--n-space-4);
  right: var(--n-space-4);
  z-index: 6;
  padding: 0 var(--n-space-4);
  border-radius: var(--n-radius-4);
  background: var(--n-g3);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-warning);
  transition: transform var(--n-dur-250) var(--n-ease-in-out);
}
.disk-card:hover .occupied-tag {
  transform: translateY(-4px);
}
</style>
