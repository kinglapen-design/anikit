<script setup lang="ts">
// 收件箱"工程文件"磁盘实况区块（4.4，PRD 05-01 2026-07-25 语义重定）：
// 磁盘为事实来源——不建档不写库，热刷新由 content.inboxDisk 全量替换驱动；
// 白名单以外不自动导入，右键手动"导入"= 移走入库（剪切语义，进度条 / 重复提示流与常规导入一致）。
// 占用条目：仅实况展示（"被占用"标记），导入 / 重命名禁用，后端同口径再拦一道。
// 文件图标：优先系统格式图标缓存（iconPath，details 事件补齐），无缓存回退扩展名角标；
// 目录恒 folder-closed。占用标记以 details 合并后为准（快层目录 occupied=false 是过渡态）。
import { computed, nextTick, ref } from "vue";
import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import MenuDivider from "./MenuDivider.vue";
import SectionHeader from "./SectionHeader.vue";
import { formatBadgeColor } from "../lib/formatBadge";
import { useContentStore, type ImportEvent, type InboxDiskEntry } from "../stores/content";

const content = useContentStore();
const diskOpen = ref(true);

// 操作错误（打开 / 重命名 / 删除 / 导入占用拦截）：后端中文人话原样展示，5 秒自消
const errorMsg = ref("");
let errorTimer: ReturnType<typeof setTimeout> | undefined;
function showError(e: unknown): void {
  errorMsg.value = String(e);
  clearTimeout(errorTimer);
  errorTimer = setTimeout(() => (errorMsg.value = ""), 5000);
}

// 扩展名角标：复用格式角标色映射（静态图组无底色时回退 C1-40——实况区无缩略图，角标必须可见）
function extOf(e: InboxDiskEntry): string {
  const i = e.name.lastIndexOf(".");
  return i > 0 ? e.name.slice(i + 1) : "";
}
function extLabel(e: InboxDiskEntry): string {
  return extOf(e).toUpperCase();
}
function badgeColor(e: InboxDiskEntry): string {
  return formatBadgeColor(extOf(e)) ?? "var(--c1-40)";
}
function sizeLabel(e: InboxDiskEntry): string {
  const kb = e.size / 1024;
  if (kb < 1024) return `${Math.max(1, Math.round(kb))} KB`;
  return `${(kb / 1024).toFixed(1)} MB`;
}

// 选中：与素材卡片 / 文件夹卡片互斥（content.selectedInboxName 第三态）
function select(e: InboxDiskEntry): void {
  content.selectedInboxName = e.name;
  content.selectedAssetKey = "";
  content.selectedFolderId = "";
}

// 右键菜单：打开 / 重命名 / 删除 / 导入（占用禁用导入；菜单点击不自动关，动作内手动关）
const menu = ref<{ x: number; y: number; entry: InboxDiskEntry } | null>(null);
function openMenu(e: MouseEvent, entry: InboxDiskEntry): void {
  select(entry);
  menu.value = { x: e.clientX, y: e.clientY, entry };
}

async function openEntry(): Promise<void> {
  const entry = menu.value?.entry;
  menu.value = null;
  if (!entry) return;
  try {
    await invoke("open_inbox_entry", { name: entry.name });
  } catch (err) {
    showError(err);
  }
}

async function deleteEntry(): Promise<void> {
  const entry = menu.value?.entry;
  menu.value = null;
  if (!entry) return;
  try {
    await invoke("delete_inbox_entry", { name: entry.name });
    if (content.selectedInboxName === entry.name) content.selectedInboxName = "";
  } catch (err) {
    showError(err);
  }
  await content.refreshInboxDisk();
}

// 手动导入 = 移走入库（Move + Keep；Channel 复用现有导入事件流，进度条 / 重复提示一致）
async function importEntry(): Promise<void> {
  const entry = menu.value?.entry;
  menu.value = null;
  if (!entry || entry.occupied) return;
  content.lastImportSource = "move";
  content.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
  const onEvent = new Channel<ImportEvent>();
  onEvent.onmessage = (e) => {
    content.onImportEvent(e);
    if (e.type === "finished") void content.refreshInboxDisk(); // 移走后实况剔除该条目
  };
  try {
    const taskId = await invoke<number>("import_inbox_entry", { name: entry.name, onEvent });
    if (content.importProgress && content.importProgress.taskId === null) {
      content.importProgress.taskId = taskId;
    }
  } catch (err) {
    content.importProgress = null;
    showError(err);
  }
}

// 行内重命名：名称位切 input，Enter 提交 / Esc 取消
const renaming = ref<{ name: string; draft: string } | null>(null);
const renameInput = ref<HTMLInputElement>();
function startRename(): void {
  const entry = menu.value?.entry;
  menu.value = null;
  if (!entry || entry.occupied) return;
  renaming.value = { name: entry.name, draft: entry.name };
  void nextTick(() => {
    renameInput.value?.focus();
    renameInput.value?.select();
  });
}
async function commitRename(): Promise<void> {
  const r = renaming.value;
  if (!r) return;
  const newName = r.draft.trim();
  renaming.value = null;
  if (!newName || newName === r.name) return;
  try {
    await invoke("rename_inbox_entry", { name: r.name, newName });
    if (content.selectedInboxName === r.name) content.selectedInboxName = newName;
  } catch (err) {
    showError(err);
  }
  await content.refreshInboxDisk();
}
function cancelRename(): void {
  renaming.value = null;
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
          :class="{ selected: content.selectedInboxName === e.name }"
          @click="select(e)"
          @contextmenu.prevent="openMenu($event, e)"
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
              v-if="renaming?.name === e.name"
              ref="renameInput"
              v-model="renaming.draft"
              class="rename-input"
              @keydown.enter.prevent="commitRename"
              @keydown.esc.prevent="cancelRename"
              @blur="cancelRename"
              @click.stop
            />
            <span v-else class="title">{{ e.name }}</span>
            <span class="meta">{{ e.isDir ? "文件夹" : sizeLabel(e) }}</span>
          </div>
        </div>
      </div>
    </template>

    <!-- 右键菜单必须 Teleport 出 AssetGrid 变换层：grid-canvas/cards-layer 带 transform，
         position:fixed 在变换祖先内会改为相对该祖先定位，菜单就会跟着滚动偏移隔很远 -->
    <Teleport to="body">
      <AppMenu v-if="menu" :x="menu.x" :y="menu.y" @close="menu = null">
        <MenuItem label="打开" @select="openEntry" />
        <MenuItem label="重命名" :disabled="menu.entry.occupied" @select="startRename" />
        <MenuItem label="删除" @select="deleteEntry" />
        <MenuDivider />
        <MenuItem label="导入" :disabled="menu.entry.occupied" @select="importEntry" />
      </AppMenu>
    </Teleport>
  </section>
</template>

<style scoped>
/* 区块容器：与文件夹区块同口径（间距 8 / 右 20 对齐网格预留） */
.disk-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-20) var(--space-8) 0;
}
/* 固定提醒与操作错误：副文本档（font-icon-3 / caption），错误用语义色 --error */
.notice {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
}
.error-line {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--error);
  word-break: break-all;
}
/* 卡片列表：与文件夹卡片同 grid 口径（auto-fill 同宽，列间距 16） */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  column-gap: var(--space-16);
  row-gap: var(--space-16);
}
.disk-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
  cursor: default;
}
/* 图标容器：正方形（2026-07-25 用户定）/ 圆角 8 / 1px border-2
   （hover border-1 / 选中品牌色——与素材、文件夹卡片一致） */
.icon-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1 / 1;
  border-radius: var(--radius-8);
  box-shadow: inset 0 0 0 1px var(--border-2);
  transition: box-shadow 0.12s ease;
}
.disk-card:hover .icon-wrap {
  box-shadow: inset 0 0 0 2px var(--border-1);
}
.disk-card.selected .icon-wrap {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
.folder-icon {
  color: var(--font-icon-3);
}
/* 系统格式图标：缓存源图 256px，contain 缩进正方形图标位（与缩略图同 object-fit 口径，不新造 token） */
.format-icon {
  max-width: 48px;
  max-height: 48px;
  object-fit: contain;
}
/* 扩展名角标文本：实底白字（格式角标 7 组实色映射，01 样式 token 01-6） */
.ext-badge {
  padding: var(--space-2) var(--space-6);
  border-radius: var(--radius-4);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-1);
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
/* 文字组：与文件夹卡片一致（间距 2 / 居中 / 标题两行截断） */
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  text-align: center;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-all;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 行内重命名输入：C4 底 / 1px border-1 / 圆角 6（输入框 token 口径） */
.rename-input {
  width: 100%;
  box-sizing: border-box;
  padding: var(--space-2) var(--space-6);
  border: none;
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-1);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  text-align: center;
  outline: none;
}
</style>
