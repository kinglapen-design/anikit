<script setup lang="ts">
// 偏好设置：BaseModal framed 左右布局，分类内容独立滚动，设置即时生效。
// 2026-11-08：语义分析从通用页独立；壳扩至 880×640 并随视口收缩，分组与换行规则防止内容挤压截断。
// 命令与持久化收口 lib/settingsData.ts / lib/semanticSettings.ts；mock 仅供设计系统陈列，不发命令。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import SwitchButton from "./buttons/SwitchButton.vue";
import SectionTitle from "./items/SectionTitle.vue";
import TextInput from "./inputs/TextInput.vue";
import SemanticSettings from "./settings/SemanticSettings.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { formatSize } from "../lib/formatSize";
import { useSettingsData, type SettingsMockData } from "../lib/settingsData";

const props = defineProps<{ mock?: SettingsMockData }>();
const emit = defineEmits<{ close: [] }>();

const {
  spaceZoom,
  doubleClick,
  mdBundleDir,
  chooseMdBundleDir,
  resetMdBundleDir,
  autoImport,
  inboxDrive,
  cacheLib,
  cacheWs,
  cacheTotal,
  clearing,
  clearOne,
  clearAll,
  cepSecs,
  cepMB,
  cepFps,
  commitCep,
} = useSettingsData(props.mock);

const NAV = [
  { key: "general", label: "通用" },
  { key: "semantic", label: "语义分析" },
  { key: "import", label: "自动导入" },
  { key: "cache", label: "缓存" },
  { key: "plugins", label: "插件" },
  { key: "shortcuts", label: "快捷键" },
] as const;
type NavKey = (typeof NAV)[number]["key"];
const tab = ref<NavKey>("general");
const navTitle = computed(() => NAV.find((n) => n.key === tab.value)?.label ?? "");

// 悬浮滚轮锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const contentEl = ref<HTMLElement>();
// 切分类回滚到顶（滚动容器跨 pane 持久，:key 只重建 pane）
watch(tab, () => {
  if (contentEl.value) contentEl.value.scrollTop = 0;
});

const SPACE_ZOOM_OPTIONS = [
  { value: "hover", label: "悬停优先" },
  { value: "selected", label: "仅选中项" },
] as const;
const DOUBLE_CLICK_OPTIONS = [
  { value: "preview", label: "应用内预览" },
  { value: "app", label: "默认应用打开" },
] as const;

// 快捷键表（2026-10-15 全量点亮：Enter / Ctrl+Enter / 新建类五键与 "/" 实绑 = useGlobalKeysOpen /
// useGlobalKeysCreate，回收站 Shift+Enter / Ctrl+E 守卫放行；Shift+Delete / Alt+F4 两行补齐全菜单标注。
// bound 字段与「未接入」橙标机制保留——后续新增未接键时标 bound:false 即可，无需再改模板）
const SHORTCUTS: { action: string; keys: string; bound: boolean }[] = [
  { action: "撤销", keys: "Ctrl+Z", bound: true },
  { action: "重做", keys: "Ctrl+Shift+Z", bound: true },
  { action: "剪切", keys: "Ctrl+X", bound: true },
  { action: "复制", keys: "Ctrl+C", bound: true },
  { action: "粘贴", keys: "Ctrl+V", bound: true },
  { action: "作为副本粘贴", keys: "Ctrl+Shift+V", bound: true },
  { action: "全选", keys: "Ctrl+A", bound: true },
  { action: "清除选择", keys: "Esc", bound: true },
  { action: "重命名", keys: "F2", bound: true },
  { action: "创建副本", keys: "Ctrl+D", bound: true },
  { action: "删除", keys: "Delete", bound: true },
  { action: "从文件夹中移除", keys: "Shift+Delete", bound: true },
  { action: "移动", keys: "Ctrl+Y", bound: true },
  { action: "链接管理", keys: "Ctrl+J", bound: true },
  { action: "添加链接", keys: "Ctrl+L", bound: true },
  { action: "发送至 Animate", keys: "Ctrl+E", bound: true },
  { action: "打开预览", keys: "Enter", bound: true },
  { action: "新标签页打开", keys: "Ctrl+Enter", bound: true },
  { action: "在默认应用打开", keys: "Shift+Enter", bound: true },
  { action: "新建文件夹", keys: "Ctrl+Shift+N", bound: true },
  { action: "新建子文件夹", keys: "Ctrl+Shift+Alt+N", bound: true },
  { action: "新建档案", keys: "Shift+N", bound: true },
  { action: "转换为档案", keys: "Alt+N", bound: true },
  { action: "展开或收起所有文件夹", keys: "/", bound: true },
  { action: "搜索", keys: "Ctrl+F", bound: true },
  { action: "切换布局", keys: "Alt+1~4", bound: true },
  { action: "悬停预览", keys: "空格", bound: true },
  { action: "呼出或隐藏窗口", keys: "Alt+`", bound: true },
  { action: "偏好设置", keys: "Ctrl+,", bound: true },
  { action: "退出", keys: "Alt+F4", bound: true },
];

// Esc 关闭（LinkManageDialog 同径：capture + stopPropagation；输入框内 Esc 不关窗——AppMenu 同口径）
function onKey(e: KeyboardEvent): void {
  if (e.key !== "Escape") return;
  const t = e.target as HTMLElement | null;
  if (t?.tagName === "INPUT") return;
  e.stopPropagation();
  emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey, true));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey, true));
</script>

<template>
  <BaseModal title="偏好设置" :width="880" framed class="settings-dialog" @close="emit('close')">
    <div class="sd-wrap">
      <nav class="sd-nav" aria-label="设置分类">
        <button
          v-for="n in NAV"
          :key="n.key"
          class="sd-nav-row"
          :class="{ on: tab === n.key }"
          :aria-current="tab === n.key ? 'page' : undefined"
          @click="tab = n.key"
        >
          <span class="sd-nav-icon" />
          <span class="sd-nav-label">{{ n.label }}</span>
        </button>
      </nav>
      <div class="sd-card">
        <div ref="contentEl" class="sd-content kit-scroll">
          <Transition name="sd-x">
            <div :key="tab" class="sd-pane">
              <SectionTitle variant="block" :title="navTitle" expanded :actions="false" />
              <div v-if="tab === 'general'" class="sd-rows">
                <section class="sd-section">
                  <SectionTitle variant="block" title="素材浏览" expanded :actions="false" />
                  <div class="sd-row">
                    <span class="sd-label">空格预览</span>
                    <SegmentedControl v-model="spaceZoom" :options="SPACE_ZOOM_OPTIONS" variant="text" size="sm" class="sd-seg" />
                  </div>
                  <div class="sd-row">
                    <span class="sd-label">双击素材卡片</span>
                    <SegmentedControl v-model="doubleClick" :options="DOUBLE_CLICK_OPTIONS" variant="text" size="sm" class="sd-seg" />
                  </div>
                </section>
                <section class="sd-section">
                  <SectionTitle variant="block" title="文档导出" expanded :actions="false" />
                  <div class="sd-field">
                    <div class="sd-row">
                      <span class="sd-label">MD 打包导出目录</span>
                      <span class="sd-input-side">
                        <ActionButton label="选择目录…" variant="ghost" size="sm" @click="chooseMdBundleDir" />
                        <ActionButton label="恢复默认" variant="ghost" size="sm" :disabled="!mdBundleDir" @click="resetMdBundleDir" />
                      </span>
                    </div>
                    <p class="sd-note">{{ mdBundleDir || "未设置：导出到当前文档旁" }}</p>
                  </div>
                </section>
              </div>
              <SemanticSettings v-else-if="tab === 'semantic'" :mock="mock?.semantic" />
              <section v-else-if="tab === 'import'" class="sd-section">
                <SectionTitle variant="block" title="收件箱" expanded :actions="false" />
                <div class="sd-field">
                  <div class="sd-row">
                    <span class="sd-label">自动导入</span>
                    <SwitchButton v-model="autoImport" />
                  </div>
                  <p class="sd-note">监听储存空间的「自动导入」文件夹，新文件自动收入库中。</p>
                </div>
                <div class="sd-row">
                  <span class="sd-label">收件箱盘符</span>
                  <span class="sd-value">{{ inboxDrive ? `已挂载 ${inboxDrive}` : "未挂载" }}</span>
                </div>
              </section>
              <div v-else-if="tab === 'cache'" class="sd-cache">
                <div class="sd-cache-group sd-section">
                  <SectionTitle variant="block" title="资源库" expanded :actions="false" />
                  <div v-for="it in cacheLib" :key="it.id" class="sd-cache-row">
                    <span class="sd-cache-text">
                      <span class="sd-label">{{ it.label }}</span>
                      <span v-if="it.note" class="sd-note">{{ it.note }}</span>
                    </span>
                    <span class="sd-value">{{ formatSize(it.bytes) }}</span>
                    <ActionButton label="清理" variant="ghost" size="sm" :disabled="clearing.includes(it.id)" @click="clearOne(it.id)" />
                  </div>
                </div>
                <div class="sd-cache-group sd-section">
                  <SectionTitle variant="block" title="工作空间" expanded :actions="false" />
                  <div v-for="it in cacheWs" :key="it.id" class="sd-cache-row">
                    <span class="sd-cache-text">
                      <span class="sd-label">{{ it.label }}</span>
                      <span v-if="it.note" class="sd-note">{{ it.note }}</span>
                    </span>
                    <span class="sd-value">{{ formatSize(it.bytes) }}</span>
                    <ActionButton label="清理" variant="ghost" size="sm" :disabled="clearing.includes(it.id)" @click="clearOne(it.id)" />
                  </div>
                </div>
                <div class="sd-cache-row sd-cache-total">
                  <span class="sd-label">合计</span>
                  <span class="sd-value">{{ formatSize(cacheTotal) }}</span>
                  <ActionButton label="全部清理" variant="ghost" size="sm" :disabled="clearing.length > 0" @click="clearAll()" />
                </div>
              </div>
              <section v-else-if="tab === 'plugins'" class="sd-section">
                <SectionTitle variant="block" title="Animate CEP" expanded :actions="false" />
                <p class="sd-note">以下参数与 Animate CEP 面板共用，失焦或按 Enter 后保存。</p>
                <div class="sd-row">
                  <span class="sd-label">单段时长上限</span>
                  <span class="sd-input-side">
                    <TextInput v-model="cepSecs" size="sm" class="sd-num" @focusout="commitCep" @keydown.enter="commitCep" />
                    <span class="sd-value">秒</span>
                  </span>
                </div>
                <div class="sd-row">
                  <span class="sd-label">导出大小上限</span>
                  <span class="sd-input-side">
                    <TextInput v-model="cepMB" size="sm" class="sd-num" @focusout="commitCep" @keydown.enter="commitCep" />
                    <span class="sd-value">MB</span>
                  </span>
                </div>
                <div class="sd-row">
                  <span class="sd-label">帧率</span>
                  <span class="sd-input-side">
                    <TextInput v-model="cepFps" size="sm" class="sd-num" @focusout="commitCep" @keydown.enter="commitCep" />
                    <span class="sd-value">0 = 跟随文档</span>
                  </span>
                </div>
              </section>
              <section v-else-if="tab === 'shortcuts'" class="sd-section">
                <p class="sd-note">自定义快捷键随后续版本开放。</p>
                <div class="sd-keys">
                  <div v-for="s in SHORTCUTS" :key="s.action" class="sd-key-row">
                    <span class="sd-label">{{ s.action }}</span>
                    <span class="sd-key-side">
                      <span v-if="s.keys" class="sd-value" :class="{ dim: !s.bound }">{{ s.keys }}</span>
                      <span v-if="!s.bound" class="sd-key-off">未接入</span>
                    </span>
                  </div>
                </div>
              </section>
            </div>
          </Transition>
        </div>
        <ScrollThumb :body="contentEl" />
      </div>
    </div>
  </BaseModal>
</template>

<style scoped>
.settings-dialog :deep(.bm-modal.framed) {
  gap: 0;
  padding: 0;
  overflow: hidden;
  height: min(640px, calc(100vh - var(--n-space-72)));
  max-width: calc(100vw - var(--n-space-72));
}
.settings-dialog :deep(.bm-modal.framed .sd-content .act-btn.sm) {
  width: auto;
  padding: var(--n-space-4) var(--n-space-8);
}
.sd-wrap {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
}
.sd-nav {
  width: 168px;
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  padding: var(--n-space-12);
  box-sizing: border-box;
  border-right: 1px solid var(--n-g5);
}
.sd-nav-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 32px;
  padding: 0 var(--n-space-8);
  border-radius: var(--n-radius-6);
  flex: none;
  text-align: left;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
.sd-nav-row:hover {
  background: var(--n-l-a3);
}
.sd-nav-row.on,
.sd-nav-row.on:hover {
  background: var(--n-l-a2);
}
.sd-nav-icon {
  width: 18px;
  height: 18px;
  flex: none;
}
.sd-nav-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.sd-nav-row.on .sd-nav-label {
  color: var(--n-l1);
}
.sd-card {
  position: relative;
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  background: var(--n-g1);
  overflow: hidden;
}
.sd-content {
  flex: 1;
  min-width: 0;
  padding: var(--n-space-24);
  overflow-x: hidden;
  overflow-y: auto;
}
.sd-pane {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-16);
}
.sd-x-enter-active {
  transition:
    transform var(--n-dur-250) var(--n-ease-in) var(--n-dur-100),
    opacity var(--n-dur-250) var(--n-ease-in) var(--n-dur-100);
}
.sd-x-leave-active {
  position: absolute;
  inset: var(--n-space-24);
  transition:
    transform var(--n-dur-150) var(--n-ease-out),
    opacity var(--n-dur-150) var(--n-ease-out);
}
.sd-x-enter-from {
  transform: translateX(48px);
  opacity: 0;
}
.sd-x-leave-to {
  transform: translateX(-48px);
  opacity: 0;
}
.sd-cache {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
.sd-cache-group {
  gap: var(--n-space-6);
}
.sd-cache-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  min-height: 28px;
}
.sd-cache-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
.sd-cache-total {
  padding: var(--n-space-12) var(--n-space-16);
  border-radius: var(--n-radius-8);
  background: var(--n-g2);
  box-shadow: inset 0 0 0 1px var(--n-g3);
}
.sd-cache-total > .sd-label {
  flex: 1;
}
.sd-keys {
  display: flex;
  flex-direction: column;
}
.sd-key-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
  min-height: 28px;
}
.sd-value.dim {
  color: var(--n-l4);
}
.sd-key-side {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex: none;
}
.sd-key-side .sd-value {
  font-family: var(--n-font-mono);
  white-space: nowrap;
}
.sd-key-off {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-warning);
  white-space: nowrap;
}
</style>
