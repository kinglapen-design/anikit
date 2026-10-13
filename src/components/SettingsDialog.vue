<script setup lang="ts">
// 偏好设置面板（1.0 收尾第 1 项，2026-10-13 接生产——preview 六轮定稿后接线；mock prop 保留作
// preview 陈列，WsManageDialog 先例）：
// 壳 = BaseModal framed 720×480 定高（Eagle 式左右布局·左浅右深全出血——左导航列壳色 g3 + 右缘
// 1px g5 竖线通高，右内容区 g1 深色填满；标题横线 / 竖线 / 两块底色相连无断缝；framed 列 gap 与
// 内容段 padding 经 :deep(.bm-modal.framed) 归零——注意 fallthrough class 落在 BaseModal 根 =
// 遮罩上，压壳样式必须 :deep，五轮修横带的教训）；标题「偏好设置」（与菜单项同名）；
// 无底部按钮行（即时生效，§61/§66 同口径）；遮罩点击不关闭（BaseModal 既定）、Esc/X 关闭
// （Esc 走 LinkManageDialog 同径 capture + stopPropagation，输入框内 Esc 不关窗——AppMenu 同口径）。
// 左导航列 168：padding 12 / gap 2；导航行 = 树行口径（高 32 / 圆角 6 / 13/18；常态 l2，hover 底 l-a3，
// 选中底 l-a2 + 文字 l1）；行首 18px 图标占位空盒（图标用户后补，文字对齐基准不变）。
// 右内容区 padding 24 单区块展示；切分类动画 = 详情面板 tab 切换同款（旧 150ms 缓出 / 新 250ms 缓入
// delay 100ms / 位移 48px，§23/§61——InfoManageDialog im-x 写法搬入）；内容区 kit-scroll + ScrollThumb
//（§02；滚轮贴内容区右缘 padding 区，离场帧由内容区 overflow hidden 裁剪）。
// 区块标题 = SectionTitle block（:actions="false" 消死添加钮；不可折叠——hover 折叠箭头/grab 光标为共享件
// 既定行为，不改共享件）。
// 业务全部在 lib/settingsData.ts（UI 无业务，规范第 1 条）：通用 = ui 持久化键（spaceZoomMode /
// doubleClickOpen）；自动导入 = 空间级 set_auto_import + 盘符查询 get_inbox_drive_letter；
// 缓存 = get_cache_overview / clear_cache + library-thumbs-rebuilt 事件；插件 = cep_export_settings
// 读写（护栏与后端 validate 同区间）；关于 = getVersion + show_logs_in_explorer。
// 快捷键表 = 组件内静态数组（action / keys / bound），未接入行 = 不置灰、键位降四级 +「未接入」警告橙尾标
//（三轮用户定：颜色即状态区分），点亮 = 直接改 bound 标志。字节格式化复用 lib/formatSize。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import SwitchButton from "./buttons/SwitchButton.vue";
import SectionTitle from "./items/SectionTitle.vue";
import TextInput from "./inputs/TextInput.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { formatSize } from "../lib/formatSize";
import { useSettingsData, type SettingsMockData } from "../lib/settingsData";
import logoUrl from "../../assets/image/anikit-logo.png?url";

const props = defineProps<{ mock?: SettingsMockData }>();
const emit = defineEmits<{ close: [] }>();

// 数据层（mock = preview 陈列态，命令一律不发——isMock 早退在 composable 内）
const {
  spaceZoom,
  doubleClick,
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
  version,
  openLogs,
} = useSettingsData(props.mock);

// 分类导航（默认选中「通用」）；行首图标槽位留空，用户后补
const NAV = [
  { key: "general", label: "通用" },
  { key: "import", label: "自动导入" },
  { key: "cache", label: "缓存" },
  { key: "plugins", label: "插件" },
  { key: "shortcuts", label: "快捷键" },
  { key: "about", label: "关于" },
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

// 快捷键表（bound = 已接入；点亮 = 直接改标志。排布 = 已接入在前，未接入橙标组殿后）
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
  { action: "移动", keys: "Ctrl+Y", bound: true },
  { action: "链接管理", keys: "Ctrl+J", bound: true },
  { action: "添加链接", keys: "Ctrl+L", bound: true },
  { action: "发送至 Animate", keys: "Ctrl+E", bound: true },
  { action: "在默认应用打开", keys: "Shift+Enter", bound: true },
  { action: "搜索", keys: "Ctrl+F", bound: true },
  { action: "切换布局", keys: "Alt+1~4", bound: true },
  { action: "悬停预览", keys: "空格", bound: true },
  { action: "呼出或隐藏窗口", keys: "Alt+`", bound: true },
  { action: "偏好设置", keys: "Ctrl+,", bound: true },
  { action: "打开预览", keys: "Enter", bound: false },
  { action: "新标签页打开", keys: "Ctrl+Enter", bound: false },
  { action: "新建文件夹", keys: "Ctrl+Shift+N", bound: false },
  { action: "新建子文件夹", keys: "Ctrl+Shift+Alt+N", bound: false },
  { action: "新建档案", keys: "Shift+N", bound: false },
  { action: "转换为档案", keys: "Alt+N", bound: false },
  { action: "展开或收起所有文件夹", keys: "", bound: false },
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
  <BaseModal title="偏好设置" :width="720" framed class="settings-dialog" @close="emit('close')">
    <div class="sd-wrap">
      <!-- 左导航列（168 定宽；行首 18px 图标占位空盒——图标后补，文字对齐基准不变） -->
      <nav class="sd-nav">
        <button
          v-for="n in NAV"
          :key="n.key"
          class="sd-nav-row"
          :class="{ on: tab === n.key }"
          @click="tab = n.key"
        >
          <span class="sd-nav-icon" />
          <span class="sd-nav-label">{{ n.label }}</span>
        </button>
      </nav>
      <!-- 右内容区 = g1 深色区全出血填满（2026-10-13 四轮定稿：撤内嵌卡片——顶贴标题栏底线、底贴壳底缘、
           右贴壳右缘、左贴导航竖线；标题横线 / 导航竖线 / 两块底色相连无断缝，Eagle 式；
           壳 overflow:hidden 收外圆角兜底；ScrollThumb 贴内容区右缘 padding 区，切分类离场帧由卡裁剪） -->
      <div class="sd-card">
        <div ref="contentEl" class="sd-content kit-scroll">
          <Transition name="sd-x">
          <div :key="tab" class="sd-pane">
            <SectionTitle variant="block" :title="navTitle" expanded :actions="false" />
            <!-- 通用 -->
            <div v-if="tab === 'general'" class="sd-rows">
              <div class="sd-row">
                <span class="sd-label">空格预览</span>
                <SegmentedControl v-model="spaceZoom" :options="SPACE_ZOOM_OPTIONS" variant="text" tone="g2" size="sm" class="sd-seg" />
              </div>
              <div class="sd-row">
                <span class="sd-label">双击素材卡片</span>
                <SegmentedControl v-model="doubleClick" :options="DOUBLE_CLICK_OPTIONS" variant="text" tone="g2" size="sm" class="sd-seg" />
              </div>
            </div>
            <!-- 自动导入 -->
            <div v-else-if="tab === 'import'" class="sd-rows">
              <div class="sd-field">
                <div class="sd-row">
                  <span class="sd-label">自动导入</span>
                  <SwitchButton v-model="autoImport" />
                </div>
                <p class="sd-note">监听储存空间的「自动导入」文件夹，新文件自动收入库中</p>
              </div>
              <div class="sd-row">
                <span class="sd-label">收件箱盘符</span>
                <span class="sd-value">{{ inboxDrive ? `已挂载 ${inboxDrive}` : "未挂载" }}</span>
              </div>
            </div>
            <!-- 缓存（两子组 + 合计行；清理钮 = ActionButton ghost sm） -->
            <div v-else-if="tab === 'cache'" class="sd-cache">
              <div class="sd-cache-group">
                <span class="sd-sub">资源库</span>
                <div v-for="it in cacheLib" :key="it.id" class="sd-cache-row">
                  <span class="sd-cache-text">
                    <span class="sd-label">{{ it.label }}</span>
                    <span v-if="it.note" class="sd-note">{{ it.note }}</span>
                  </span>
                  <span class="sd-value">{{ formatSize(it.bytes) }}</span>
                  <ActionButton label="清理" variant="ghost" size="sm" :disabled="clearing.includes(it.id)" @click="clearOne(it.id)" />
                </div>
              </div>
              <div class="sd-cache-group">
                <span class="sd-sub">工作空间</span>
                <div v-for="it in cacheWs" :key="it.id" class="sd-cache-row">
                  <span class="sd-cache-text">
                    <span class="sd-label">{{ it.label }}</span>
                    <span v-if="it.note" class="sd-note">{{ it.note }}</span>
                  </span>
                  <span class="sd-value">{{ formatSize(it.bytes) }}</span>
                  <ActionButton label="清理" variant="ghost" size="sm" :disabled="clearing.includes(it.id)" @click="clearOne(it.id)" />
                </div>
              </div>
              <div class="sd-cache-row">
                <span class="sd-cache-text">
                  <span class="sd-label">合计</span>
                </span>
                <span class="sd-value">{{ formatSize(cacheTotal) }}</span>
                <ActionButton label="全部清理" variant="ghost" size="sm" :disabled="clearing.length > 0" @click="clearAll()" />
              </div>
            </div>
            <!-- 插件（Animate CEP 导出参数；TextInput sm 定宽 96 + 单位/说明） -->
            <template v-else-if="tab === 'plugins'">
              <p class="sd-note">Animate CEP 面板的导出参数</p>
              <div class="sd-rows">
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
              </div>
            </template>
            <!-- 快捷键（行 22 紧凑排；未接入 = 键位降四级 + 警告橙尾标，不整行置灰；内容区自己滚） -->
            <template v-else-if="tab === 'shortcuts'">
              <p class="sd-note">自定义快捷键随后续版本开放</p>
              <div class="sd-keys">
                <div v-for="s in SHORTCUTS" :key="s.action" class="sd-key-row">
                  <span class="sd-label">{{ s.action }}</span>
                  <span class="sd-key-side">
                    <span v-if="s.keys" class="sd-value" :class="{ dim: !s.bound }">{{ s.keys }}</span>
                    <span v-if="!s.bound" class="sd-key-off">未接入</span>
                  </span>
                </div>
              </div>
            </template>
            <!-- 关于 -->
            <div v-else class="sd-about">
              <img :src="logoUrl" alt="AniKit" class="sd-logo" />
              <div class="sd-about-text">
                <span class="sd-about-name">AniKit</span>
                <span class="sd-note">面向动画师的素材管理软件</span>
                <span class="sd-note">版本 {{ version }}</span>
              </div>
              <ActionButton label="查看软件日志" variant="ghost" size="sm" @click="openLogs()" />
            </div>
          </div>
        </Transition>
        </div>
        <ScrollThumb :body="contentEl" />
      </div>
    </div>
  </BaseModal>
</template>

<style scoped>
/* 偏好设置（Eagle 式左右布局·左浅右深全出血，2026-10-13 四轮定稿；preview 先行稿）——全 token 零新档 */
/* 定高链：壳 480 = 标题栏 54 + body 区 426（framed 列 gap 本实例归 0——标题横线与 body 相连，四轮定稿） */
/* 壳级三笔（只对本实例生效，BaseModal 共享件不动）——注意落点：fallthrough class 落在 BaseModal
   根元素 = 遮罩 .bm-mask 上（**不在 .bm-modal**！），四轮的类名叠加全压在遮罩上空发（横带未消即此因，
   四轮走查用户圈出）；正确径 = :deep 进 .bm-modal.framed——(0,4,0) 稳压共享件 (0,3,0)，与样式表序无关。
   gap 0 = 标题横线与 body 相连；padding 0 = 内容段四边归零保险（framed 本即 padding 0，显式压死）；
   overflow hidden = g1 全出血贴壳缘的外圆角裁剪（此前误挂遮罩，实际从未生效） */
.settings-dialog :deep(.bm-modal.framed) {
  gap: 0;
  padding: 0;
  overflow: hidden;
}
.sd-wrap {
  position: relative;
  display: flex;
  height: 426px;
}
/* 左导航列：168 宽 / padding 12 / gap 2；底色 = 壳色 g3（三轮定稿「左浅」——不独立成色块）；
   右缘 1px g5 竖向分割线，顶贴标题线到底缘通高（与标题横线、两块底色相连无断缝） */
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
/* 导航行 = 树行口径：高 32 / 圆角 6 / 13/18；常态文字二级，hover 底 亮色/半透明-3，
   选中底 亮色/半透明-2 + 文字升一级（TreeRowItem 选中态写法）；hover/选中过渡 150ms */
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
/* 行首 18px 图标占位空盒（图标用户后补，槽位先留） */
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
/* 右内容区 = g1 深色区全出血（四轮定稿「右深」填满：撤三轮内嵌卡片的 margin/圆角——顶贴标题栏底线、
   底贴壳底缘、右贴壳右缘、左贴导航竖线；自身无圆角，外圆角由壳 overflow hidden 收）；
   卡内 = 滚动容器（padding 24 自承）+ ScrollThumb（贴右缘 padding 区）；overflow hidden 裁切分类离场帧 */
.sd-card {
  position: relative;
  flex: 1;
  min-width: 0;
  display: flex;
  background: var(--n-g1);
  overflow: hidden;
}
/* 滚动容器：padding 24 / 单区块展示；超高内容（快捷键）卡内滚动 */
.sd-content {
  flex: 1;
  min-width: 0;
  padding: var(--n-space-24);
  overflow-x: hidden;
}
/* 区块 pane：标题 → 行组 gap 8 */
.sd-pane {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 切分类滑动动画（详情面板 tab 同款，§23/§61：退 150 缓出先行 / 入 250 缓入 delay 100 /
   位移 48px / 离场 absolute 不占位；方向恒定 = 旧左出 / 新右入）——InfoManageDialog im-x 写法搬入 */
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
/* 28 行组（§39⑤ 查看方式稿自动播放组口径：行 28 / gap 8） */
.sd-rows {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 行：高 28，label 与控件两端对齐 */
.sd-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
  height: 28px;
}
/* 行 + 行下说明 的复合块（自动导入开关行） */
.sd-field {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
/* 行 label：13/18 二级 */
.sd-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
}
/* 次要文字三语境同款 12/16 三级：说明行 / 右侧状态·数值·单位 / 缓存子组小标 */
.sd-note,
.sd-value,
.sd-sub {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
.sd-note {
  margin: 0;
}
.sd-value {
  white-space: nowrap;
  flex: none;
}
/* 行内分段控制器：收缩为内容宽（组件根默认 100% 均分） */
.sd-seg {
  width: auto;
  flex: none;
}
/* 缓存区块：子组间 + 合计行 gap 12 */
.sd-cache {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
/* 缓存子组：小标 → 行组，行距 4 */
.sd-cache-group {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
/* 缓存行：min-height 28（清理钮 28 高）；左 文本列（名 13/18 二级 + 可选副说明 12/16 三级双行）
   + 右 占用大小 12/16 三级 + 清理钮 */
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
/* 插件输入行右组：TextInput sm 定宽 96 + 单位/说明 */
.sd-input-side {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex: none;
}
.sd-num {
  width: 96px;
  flex: none;
}
/* 快捷键表：行 22 紧凑排 */
.sd-keys {
  display: flex;
  flex-direction: column;
}
.sd-key-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
  height: 22px;
}
/* 未接入行不置灰（2026-10-13 三轮用户定：颜色即状态区分）——动作名照常 l2，键位降四级 */
.sd-value.dim {
  color: var(--n-l4);
}
.sd-key-side {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex: none;
}
/* 「未接入」尾标：12/16 状态/警告橙（颜色即状态区分；已接入行无尾标） */
.sd-key-off {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-warning);
  white-space: nowrap;
}
/* 关于：左 logo 44 + 右文本列 + 行尾日志钮 */
.sd-about {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
}
.sd-logo {
  width: 44px;
  height: 44px;
  object-fit: contain;
  flex: none;
}
.sd-about-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
/* 「AniKit」：16/22 Bold 一级（按钮文字-大档） */
.sd-about-name {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
</style>
