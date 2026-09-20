<script setup lang="ts">
// 独立窗口 / 整页挂载件展区——覆盖清单与处理模式：
// 1. PreviewWindow.vue（预览子窗口 4.3 新窗口打开）= 静态复刻壳（pv-pw-shell）：组件 onMounted 拉全量数据
//    （content.refresh + preview.open）不能真实挂载；标题栏按 PreviewTitleBar window 模式结构复刻，
//    嵌真实旧 IconButton / SideSlider（纯件）；窗口控制三钮 = 旧 WindowControls 复刻（§23：随预览族轮再改）；
//    画面区 / PreviewNavigator = 静态占位（依赖 preview store）。口径变更与组件同步。
// 2. SetupWizard.vue（首启向导 38 稿，主窗口全屏层 + 600×559 卡片）= 静态复刻壳（pv-wiz-shell）+ 真实步骤件
//    （WizardStepSpace / WizardStepLibrary / WizardStepPlaceholder 纯 props 驱动，本地态演示，不走后端）；
//    遮罩层（absolute inset 0 全覆盖）复刻略；步骤切换可点（本地 step），「打开储存空间 / 完成」接后端 = 禁用。
// 3. PickerPanel.vue（右侧边栏选择模式备选区，§12/§23）= 静态复刻壳（pv-pk-shell）+ 真实
//    items/PickFileItem + PickRelationItem（纯 props 驱动，本地态演示关闭/编辑）；确认/取消接 picker store = 不响应。
import { computed, ref } from "vue";
import IconButtonLegacy from "../../components/IconButton.vue";
import SideSlider from "../../components/SideSlider.vue";
import AppButton from "../../components/AppButton.vue";
import WizardStepSpace from "../../components/WizardStepSpace.vue";
import WizardStepLibrary from "../../components/WizardStepLibrary.vue";
import WizardStepPlaceholder, { type PlaceholderIcon } from "../../components/WizardStepPlaceholder.vue";
import PickFileItem from "../../components/items/PickFileItem.vue";
import PickRelationItem from "../../components/items/PickRelationItem.vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import { getIcon } from "../../lib/icons";

// —— 预览子窗口复刻：缩放滑块本地态（SideSlider 纯件可拖）——
const pwZoom = ref(100);
const pwLogoZoomText = computed(() => `${pwZoom.value}%`);

// —— 向导复刻：本地步骤态（帧 0 入口 → 一~四步；步骤件全真实，数据本地暂存）——
const logo = getIcon("LOGO"); // 临时 LOGO（同 SetupWizard 唯一引用）
const wizStep = ref(0);
const WIZ_STEP_TITLES = ["", "配置储存空间", "创建资源库", "安装浏览器插件", "安装桥接插件"];
const WIZ_STEP_NO = ["", "一", "二", "三", "四"];
const wizSpaceName = ref("");
const wizParent = ref("");
const wizLibName = ref("");
const wizAutoImport = ref(false);
const WIZ_BROWSER_ICONS: PlaceholderIcon[] = [
  { off: "chrome-off", on: "chrome-on" },
  { off: "firefox-off", on: "firefox-on" },
  { off: "edge-off", on: "edge-on" },
];
const WIZ_BRIDGE_ICONS: PlaceholderIcon[] = [
  { off: "animate-off", on: "animate-on" },
  { off: "photoshop-off", on: "photoshop-on" },
  { off: "ae-off", on: "ae-on" },
];

// —— 选择器面板复刻：两模板各一壳，item 真实嵌（关闭/编辑 = 本地数组操作）——
const pkRelationItems = ref([{ name: "主角-走01.fla", from: "父亲", to: "儿子" }]);
const pkFileItems = ref([{ name: "参考配乐.mp3" }, { name: "分镜脚本.txt" }]);
const pkRelationConfirm = computed(() =>
  pkRelationItems.value.length > 0 ? `确认 (${pkRelationItems.value.length})` : "确认",
);
const pkFileConfirm = computed(() =>
  pkFileItems.value.length > 0 ? `确认 (${pkFileItems.value.length})` : "确认",
);
</script>

<template>
  <section id="design-system-windows" class="pv-section">
    <h2 class="pv-sec-title">独立窗口 · 整页挂载件</h2>
    <p class="pv-sec-desc">
      预览子窗口 / 首启向导 / 选择器面板均为独立窗口或全屏挂载件，依赖 store 或窗口上下文，不便真实陈列——
      统一 pv-*-shell 静态复刻（口径变更与组件同步），壳内纯 props 驱动的子件嵌真实组件
      （IconButton / SideSlider / AppButton / 向导三步骤件 / Pick 两 item）。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">预览子窗口 PreviewWindow（PRD 4.3 新窗口打开；图示 = 图片素材 window 模式标题栏）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-pw-shell">
            <div class="pv-pw-titlebar">
              <div class="pv-pw-nav">
                <span class="pv-pw-name">主角-走01.png</span>
              </div>
              <div class="pv-pw-zoom">
                <SideSlider v-model="pwZoom" :min="2" :max="800" />
                <span class="pv-pw-zoom-text">{{ pwLogoZoomText }}</span>
              </div>
              <div class="pv-pw-btns">
                <IconButtonLegacy name="plugin" title="插件" />
                <IconButtonLegacy name="rotate-cw" title="旋转" />
                <IconButtonLegacy name="flip-h" title="水平翻转" />
                <IconButtonLegacy name="fit-screen" title="适合窗口" />
                <IconButtonLegacy name="original-size" title="原图大小" />
                <IconButtonLegacy name="open-default" title="默认应用打开" />
                <IconButtonLegacy name="angle-bracket" title="上一个" disabled />
                <IconButtonLegacy name="angle-bracket" title="下一个" class="pv-rot-180" />
                <IconButtonLegacy name="pin-on" title="窗口置顶" />
                <IconButtonLegacy name="window-minimize" title="最小化" />
                <IconButtonLegacy name="window-maximize" title="最大化" />
                <IconButtonLegacy name="window-close" title="关闭" danger />
              </div>
            </div>
            <div class="pv-pw-stage">
              <span class="pv-pw-stage-tip">预览画面区（PreviewPanel 依赖 preview store，不嵌真）</span>
              <!-- PreviewNavigator 静态复刻（真实件按 zoom/pan 计算视口框，依赖 store） -->
              <div class="pv-pw-navigator">
                <div class="pv-pw-nav-box">
                  <span class="pv-pw-nav-thumb" />
                  <span class="pv-pw-nav-viewport" />
                </div>
              </div>
            </div>
          </div>
          <span class="pv-cap">
            静态复刻（组件挂载即 content.refresh + preview.open，不能真实挂载），口径变更与组件同步；
            混合体系待预览族轮——标题栏文字/图标色已吃 --n-，但间距（--space-*）、旧 IconButton、
            窗口控制三钮（旧 WindowControls，§23「随预览族轮再改」）、window-minimize 等图标走旧版回退层；
            悬浮播放条 FloatBarHost（依赖 store）未复刻；壳尺寸 720×480 = 演示值（真实窗随 OS 窗口弹性）
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">首启向导 SetupWizard（600×559 卡片；步骤切换可点——步骤件全真实，本地态）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-wiz-shell">
            <div class="pv-wiz-titlebar">
              <IconButtonLegacy name="window-close" title="关闭" />
            </div>
            <div class="pv-wiz-logo-area">
              <div class="pv-wiz-logo" v-html="logo" />
              <div class="pv-wiz-app-text">
                <span class="pv-wiz-app-name">AniKit</span>
                <span class="pv-wiz-app-sub">动画资产管理工具</span>
              </div>
            </div>
            <div v-if="wizStep === 0" class="pv-wiz-entry">
              <AppButton label="新建储存空间" variant="primary" @click="wizStep = 1" />
              <AppButton label="打开储存空间" disabled />
            </div>
            <WizardStepSpace
              v-else-if="wizStep === 1"
              v-model:name="wizSpaceName"
              v-model:parent="wizParent"
              :busy="false"
              error=""
              @pick="wizParent = 'D:/AniKit/我的储存空间'"
            />
            <WizardStepLibrary
              v-else-if="wizStep === 2"
              v-model:name="wizLibName"
              v-model:auto-import="wizAutoImport"
              error=""
            />
            <WizardStepPlaceholder
              v-else
              :icons="wizStep === 3 ? WIZ_BROWSER_ICONS : WIZ_BRIDGE_ICONS"
              :text="wizStep === 3 ? '浏览器扩展安装将在后续版本提供' : '桥接插件安装将在后续版本提供'"
            />
            <div v-if="wizStep > 0" class="pv-wiz-step-area">
              <div class="pv-wiz-step-row">
                <AppButton
                  label="上一步"
                  size="small"
                  :class="{ 'pv-wiz-ghost': wizStep <= 1 }"
                  @click="wizStep -= 1"
                />
                <span class="pv-wiz-step-title">
                  <span class="no">第{{ WIZ_STEP_NO[wizStep] }}步：</span>
                  <span class="nm">{{ WIZ_STEP_TITLES[wizStep] }}</span>
                </span>
                <AppButton
                  :label="wizStep === 4 ? '完成' : '下一步'"
                  variant="primary"
                  size="small"
                  :disabled="wizStep === 4"
                  @click="wizStep += 1"
                />
              </div>
              <div class="pv-wiz-progress">
                <div class="fill" :style="{ width: `${(wizStep / 4) * 100}%` }" />
              </div>
            </div>
          </div>
          <span class="pv-cap">
            静态复刻壳 + 真实步骤件（WizardStepSpace/Library/Placeholder 纯 props 驱动嵌真），口径变更与组件同步；
            旧体系待换代——SetupWizard 15 处 legacy token（--c1/--c2/--text-*/--space-* 等），壳按 --n- 就近映射
            （真实件 = C2 底 / 亮-选中 1.5px 描边 / 投影，换代前以组件为准）；全屏遮罩层（absolute inset 0）复刻略；
            「打开储存空间」（接后端开空间流程）与第四步「完成」（落盘建库）演示禁用；
            第三/四步 on/off 双态图标 = 旧版回退层 SVG（嵌真件自带，hover 变色可验）
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">选择器面板 PickerPanel（右侧边栏选择模式备选区，§12/§23；两模板各一壳，item 真实嵌）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-pk-shell">
            <div class="pv-pk-body">
              <PickRelationItem
                v-if="pkRelationItems.length === 0"
                picked=""
                picked-placeholder="点击卡片选择"
                from-label=""
                to-label=""
                origin="主角-走01.fla"
                disabled
                :closable="false"
              />
              <PickRelationItem
                v-for="(p, i) in pkRelationItems"
                :key="p.name"
                :picked="p.name"
                v-model:from-label="p.from"
                v-model:to-label="p.to"
                origin="主角-走01.fla"
                @close="pkRelationItems.splice(i, 1)"
              />
            </div>
            <footer class="pv-pk-actions">
              <ActionButton label="取消" variant="solid" size="lg" />
              <ActionButton :label="pkRelationConfirm" variant="primary" size="lg" />
            </footer>
          </div>
          <span class="pv-cap">
            模板一 relation（关系模式，多选累积）：静态复刻壳 + 真实 PickRelationItem（双框称谓可编辑 / 关闭钮本地移除，
            清空后出占位组）；新体系（§23 贴稿重构）；确认/取消接 picker store 演示不响应；壳高 460 = 演示值（真实 flex:1 随右栏）
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-pk-shell">
            <div class="pv-pk-body">
              <PickFileItem
                v-if="pkFileItems.length === 0"
                name=""
                editing
                placeholder="点击卡片选择"
                readonly
                :closable="false"
              />
              <PickFileItem
                v-for="(p, i) in pkFileItems"
                :key="p.name"
                :name="p.name"
                editing
                @update:name="p.name = $event"
                @close="pkFileItems.splice(i, 1)"
              />
            </div>
            <footer class="pv-pk-actions">
              <ActionButton label="取消" variant="solid" size="lg" />
              <ActionButton :label="pkFileConfirm" variant="primary" size="lg" />
            </footer>
          </div>
          <span class="pv-cap">
            模板二 file（文件选择模式：link 添加文件链接 / voice 链接音频）：行卡 = 名称框恒输入态（默认源名可改），
            清空后出占位行；确认钮带数量「确认 (N)」（PRD 多选口径）；新体系；壳高 460 = 演示值
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* —— 预览子窗口复刻壳（与 PreviewWindow.vue + PreviewTitleBar.vue 口径同步；720×480 = 演示值） —— */
.pv-pw-shell {
  display: flex;
  flex-direction: column;
  width: 720px;
  height: 480px;
  box-sizing: border-box;
  padding: var(--n-space-4);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: var(--n-shadow-lg);
}
/* 标题栏（PreviewTitleBar window 模式）：padding 12 16 / space-between 三组；图标统一 亮色/三级 */
.pv-pw-titlebar {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
  padding: var(--n-space-12) var(--n-space-16);
}
.pv-pw-titlebar :deep(.icon-btn) {
  color: var(--n-l3);
}
.pv-pw-nav {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
}
.pv-pw-name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
  white-space: nowrap;
}
.pv-pw-zoom {
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
}
.pv-pw-zoom-text {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  min-width: 38px;
  text-align: right;
}
.pv-pw-btns {
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
}
.pv-rot-180 :deep(svg) {
  transform: rotate(180deg);
}
/* 画面区：静态占位（PreviewPanel 依赖 store 不嵌真） */
.pv-pw-stage {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--n-radius-4);
  background: var(--n-g2);
  overflow: hidden;
}
.pv-pw-stage-tip {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l4);
  user-select: none;
}
/* PreviewNavigator 静态复刻：右下 16/16 悬浮；g3 底 + 1px 半透明-1 内描边 + 大阴影 + 圆角 8 */
.pv-pw-navigator {
  position: absolute;
  right: 16px;
  bottom: 16px;
  padding: var(--n-space-4);
  background: var(--n-g3);
  border-radius: var(--n-radius-8);
  box-shadow:
    inset 0 0 0 1px var(--n-l-a1),
    var(--n-shadow-lg);
}
.pv-pw-nav-box {
  position: relative;
  width: 120px;
  height: 90px;
  border-radius: var(--n-radius-4);
  overflow: hidden;
}
.pv-pw-nav-thumb {
  position: absolute;
  inset: 0;
  background: var(--n-g5);
}
.pv-pw-nav-viewport {
  position: absolute;
  left: 18%;
  top: 12%;
  width: 46%;
  height: 42%;
  box-shadow: inset 0 0 0 1px var(--n-l1);
}

/* —— 首启向导复刻壳（与 SetupWizard.vue 口径同步；真实件旧体系，壳按 --n- 就近映射，换代前以组件为准） —— */
.pv-wiz-shell {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 600px;
  height: 559px;
  box-sizing: border-box;
  background: var(--n-g3);
  border-radius: var(--n-radius-8);
  box-shadow:
    var(--n-shadow-lg),
    inset 0 0 0 1px var(--n-l-a1);
}
.pv-wiz-titlebar {
  flex: none;
  display: flex;
  justify-content: flex-end;
  align-items: flex-start;
  width: 100%;
  height: 46px;
  box-sizing: border-box;
  padding: var(--n-space-8);
}
.pv-wiz-logo-area {
  flex: none;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: var(--n-space-12);
  width: 100%;
  height: 271px;
  box-sizing: border-box;
  padding: var(--n-space-24);
}
.pv-wiz-logo {
  width: 167px;
  height: 145px;
  flex: none;
}
.pv-wiz-logo :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
.pv-wiz-app-text {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-6);
}
.pv-wiz-app-name {
  font-size: var(--n-fs-display);
  line-height: var(--n-lh-display);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.pv-wiz-app-sub {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
/* 帧 0 入口：撑满剩余，按钮组居中 gap 12 */
.pv-wiz-entry {
  flex: 1;
  min-height: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  gap: var(--n-space-12);
  width: 100%;
  box-sizing: border-box;
  padding: var(--n-space-24);
}
/* 步骤区：146 高 / padding 48 / gap 12（上行 28 + 进度条 10；新体系无 48 档，取 24×2） */
.pv-wiz-step-area {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  width: 100%;
  height: 146px;
  box-sizing: border-box;
  padding: calc(var(--n-space-24) * 2);
}
.pv-wiz-step-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  height: 28px;
}
/* 第一步无上一步：透明占位保布局（38 稿 ghost 口径） */
.pv-wiz-ghost {
  opacity: 0;
  pointer-events: none;
}
.pv-wiz-step-title {
  display: flex;
  gap: var(--n-space-4);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.pv-wiz-step-title .no {
  color: var(--n-l3);
}
.pv-wiz-step-title .nm {
  color: var(--n-l1);
}
/* 进度条：通栏 × 10 / g1 轨道 / 品牌色填充，按 当前步/4 递增 */
.pv-wiz-progress {
  width: 100%;
  height: 10px;
  background: var(--n-g1);
  border-radius: 999px; /* 胶囊（新体系无 pill token，取旧 --radius-pill 同值） */
  overflow: hidden;
}
.pv-wiz-progress .fill {
  height: 100%;
  background: var(--n-brand);
  border-radius: 999px;
  transition: width var(--n-dur-250) var(--n-ease-in-out);
}

/* —— 选择器面板复刻壳（与 PickerPanel.vue 口径同步：264 宽 column；备选区 padding 12 / gap 12，
   按钮栏 62 padding 12 / gap 8 两钮 flex 均分；460 高 = 演示值，真实 flex:1 随右栏） —— */
.pv-pk-shell {
  display: flex;
  flex-direction: column;
  width: 264px;
  height: 460px;
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: var(--n-shadow-lg);
  overflow: hidden;
}
.pv-pk-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  overflow-y: auto;
  overflow-x: hidden;
}
/* item 宽随父容器（同真实件 :deep 口径） */
.pv-pk-body :deep(.ni-card) {
  width: 100%;
}
.pv-pk-actions {
  flex: none;
  display: flex;
  gap: var(--n-space-8);
  height: 62px;
  box-sizing: border-box;
  padding: var(--n-space-12);
}
.pv-pk-actions :deep(.act-btn) {
  flex: 1;
  min-width: 0;
}
</style>
