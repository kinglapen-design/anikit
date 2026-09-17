<script setup lang="ts">
// 组件 悬浮栏展区——docs/06_UI重构/Design Token/组件/悬浮栏/多功能悬浮栏/ 八稿的展示与验收面。
// 搜索栏 / 搜索栏展开成筛选 / 播放条 / MarkDown 工具栏 = 同一容器（FloatBar）：
// 切换宽度平滑伸缩 + 功能按钮 Y 轴出场入场 = 300ms 形态切换 + 位移 4px（2026-08-29 用户两轮定）；hover / 颜色 150ms。
// 无需悬浮栏 = 宽度 + 透明度退场（稿 prose，visible prop）。
// 播放/暂停、声音/静音 = 单钮按态切换（稿两态并列是全量画法）；**进度条已播放填充段 = 亮色/三级**
//（2026-09-09 四轮用户定稿，原「无填充段」口径作废）；帧条选中帧 = 亮色/一级 1px 边框（同轮定稿）；
// 笔记圆点、循环/笔记开启态稿仍未画记 09 待办；图标 hover 升二级 = 按钮族 IconButton bare 口径（按钮展区已验收，此不重复 force 列）。
import { ref } from "vue";
import FloatBar from "../../components/bars/FloatBar.vue";
import FrameStrip from "../../components/bars/FrameStrip.vue";
import BarSlider from "../../components/bars/BarSlider.vue";
import RatePop from "../../components/bars/RatePop.vue";
import VolumePop from "../../components/bars/VolumePop.vue";
import NotesPop from "../../components/bars/NotesPop.vue";
import ToastStack from "../../components/ToastStack.vue";
import PickerBanner from "../../components/PickerBanner.vue";
import LinkListPopover from "../../components/LinkListPopover.vue";
import { useToastStore } from "../../stores/toast";
import { usePickerStore } from "../../stores/picker";
import { useContentStore, type PlacementView } from "../../stores/content";

// 模式切换 morph 演示
const morphMode = ref<"search" | "markdown" | "play">("search");
const morphVisible = ref(true);
const morphQuery = ref("");
const morphFilters = ref<string[]>([]);
function morphToggleFilter(f: string): void {
  const i = morphFilters.value.indexOf(f);
  if (i >= 0) morphFilters.value.splice(i, 1);
  else morphFilters.value.push(f);
}

// 播放条交互实例
const livePlaying = ref(false);
const liveMuted = ref(false);
const liveProgress = ref(0.2);

// 帧条 + 播放条同宽组合（帧条自然宽 777 → 播放条 width 覆盖同步）
const COMBO_W = 777;

// 弹层交互实例
const liveRate = ref("1x");
const liveVolume = ref(0.2);

// 条形滑块三档 tone 交互实例（宽度 = 各自稿值语境：进度 327 / 音量 100 / 尺寸 100）
const sliderLight = ref(0.35);
const sliderDark = ref(0.2);
const sliderSize = ref(0.5);

// 笔记弹层
const noteList = ref([
  { frame: "0123:", text: "这是第一篇笔记…" },
  { frame: "0456:", text: "起身动作再夸张一点" },
  { frame: "0789:", text: "结尾定格延长 12 帧" },
]);
const noteSel = ref(0);
const noteDraft = ref("");
function noteConfirm(): void {
  const t = noteDraft.value.trim();
  if (t) noteList.value.push({ frame: "0000:", text: t.length > 12 ? `${t.slice(0, 12)}…` : t });
  noteDraft.value = "";
}
function noteRemove(i: number): void {
  noteList.value.splice(i, 1);
  if (noteSel.value >= noteList.value.length) noteSel.value = Math.max(0, noteList.value.length - 1);
}

// —— 通知栏演示（真实例走 toast store；duration 拉长便于目验，刷新页面复位）——
const toastDemo = useToastStore();
function demoToast(kind: "success" | "fail" | "undo"): void {
  if (kind === "fail") {
    toastDemo.notify({ message: "移动失败：目标文件夹不存在", icon: "fail", duration: 60_000 });
  } else if (kind === "undo") {
    toastDemo.notify({ message: "已删除 3 个文件", icon: "success", actionLabel: "撤销", onAction: () => {}, duration: 60_000 });
  } else {
    toastDemo.notify({ message: "已复制 2 个文件至剪切版", icon: "success", duration: 60_000 });
  }
}

// —— 选择模式横幅演示：只 seed bannerText 依赖的两个字段（不进入模式、无副作用）——
const pickerDemo = usePickerStore();
pickerDemo.purpose = "relation";
pickerDemo.origin = { kind: "asset", id: "demo-asset", name: "01_人物" };

// —— 悬停-链接列表演示：seed content store 两条活位置（同 asset 两文件夹；幂等防 HMR 重复）——
// f-renwu 故意套一层父级「角色」：实证路径两级截止口径（只显 区块 / 所在目录，完整父级链不放，2026-08-29 用户定）
const contentDemo = useContentStore();
if (contentDemo.blocks.length === 0) {
  contentDemo.blocks = [{ id: "dir", name: "文件夹", icon: null }];
  contentDemo.folders = [
    { id: "f-juese", name: "角色", count: 1, totalSize: 1_258_291, children: [
      { id: "f-renwu", name: "01_人物", count: 1, totalSize: 1_258_291, children: [] },
    ] },
    { id: "f-changjing", name: "02_场景", count: 1, totalSize: 1_258_291, children: [] },
  ];
  const base = {
    w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null,
    frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
    size: 1_258_291, createdAt: null, hidden: false, tags: [] as string[],
    modifiedAt: "2026-08-29T00:00:00Z", importedAt: "2026-08-29T00:00:00Z",
  };
  contentDemo.placements = [
    { ...base, assetId: "demo-asset", folderId: "f-renwu", name: "雪牧_自动.fla", origin: true },
    { ...base, assetId: "demo-asset", folderId: "f-changjing", name: "雪牧_自动.fla", origin: false },
  ];
}
const demoAsset = contentDemo.placements.find((p) => p.assetId === "demo-asset") as PlacementView;
const popAnchor = ref<HTMLElement>();
const showPop = ref(false); // 浮层按当前锚点位置落位——滚动后需重开（实机悬停重新触发同理）
</script>

<template>
  <section id="design-system-bars" class="pv-section">
    <h2 class="pv-sec-title">组件 · 悬浮栏</h2>
    <p class="pv-sec-desc">
      Design Token/组件/悬浮栏/多功能悬浮栏/ 八稿：搜索栏 / 展开成筛选 / MarkDown 工具栏 / 播放条 + 帧条 / 倍率 / 音量 / 笔记。
      四模式同一容器切换 = 宽度伸缩 + Y 轴出入 <b>300ms</b> 形态切换（位移 4px，2026-08-29 用户两轮定）；hover / 颜色 <b>150ms</b>；
      壳 = 通用/半透明-2 + 立体感-背景模糊 + 圆角 21（稿值），弹层 = 通用/半透明-1 + 大阴影 + 圆角 8。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">多功能悬浮栏 · 模式切换（同一容器，宽度伸缩 + 按钮 Y 轴出入 300ms）</h3>
      <p class="pv-group-note">
        点按钮切换模式 / 显隐；搜索态可输入、聚焦扩 450、点筛选图标展开筛选钮组（反向折叠同路径）——宽度全程壳动画驱动，右端筛选图标随壳边平滑移动（十五轮走查修复）。
        演示舞台居中陈列 = 实际布局口径（中间面板底部居中）：壳宽过渡逐帧重排，展开自动以中心向两侧生长。
      </p>
      <div class="pv-demo-row">
        <button class="pv-switch" @click="morphMode = 'search'">搜索栏</button>
        <button class="pv-switch" @click="morphMode = 'markdown'">MarkDown 工具栏</button>
        <button class="pv-switch" @click="morphMode = 'play'">播放条</button>
        <button class="pv-switch" @click="morphVisible = !morphVisible">
          {{ morphVisible ? "退场（宽度+透明度）" : "入场" }}
        </button>
      </div>
      <div class="pv-demo-row pv-morph-stage">
        <FloatBar
          :mode="morphMode"
          :visible="morphVisible"
          v-model:query="morphQuery"
          :active="morphFilters"
          @filter-toggle="morphToggleFilter"
        />
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">搜索栏（常态 250 / 聚焦 450 / 展开 max-content；占位符 半透明-2）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <FloatBar mode="search" />
          <span class="pv-cap">常态 · 宽 250</span>
        </div>
        <div class="pv-demo-cell">
          <FloatBar mode="search" force-state="focus" />
          <span class="pv-cap">输入聚焦 · 宽 450（force-focus 静态）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <FloatBar mode="search" initial-expanded :active="['标签']" force-hover-filter="时长" />
          <span class="pv-cap">展开成筛选 · 选中「标签」（半透明-1 + 一级）/ hover「时长」（底升半透明-2）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">MarkDown 工具栏（段落 / 列表 / 插入 / 格式 四组，组间 1px 半透明-3 分隔线）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <FloatBar mode="markdown" />
          <span class="pv-cap">常态（图标 三级，hover 升二级 = IconButton bare 口径）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">播放条（全量陈列；时间码 数字-大：当前·分隔 二级 / 总计 三级；进度条已播放填充 亮色/三级）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <FloatBar mode="play" :progress="0.2" />
          <span class="pv-cap">常态 · 暂停态（播放钮显 play）</span>
        </div>
        <div class="pv-demo-cell">
          <FloatBar mode="play" playing muted :progress="0.65" />
          <span class="pv-cap">播放中 + 静音（单钮按态切换）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <FloatBar mode="play" :show="['volume', 'rate', 'loop']" :progress="0.4" />
          <span class="pv-cap">按上下文删减（音频语境示例：无传输 / 步进 / 笔记 / 封面 / 全屏）</span>
        </div>
        <div class="pv-demo-cell">
          <FloatBar
            mode="play"
            :playing="livePlaying"
            :muted="liveMuted"
            v-model:progress="liveProgress"
            @toggle-play="livePlaying = !livePlaying"
            @volume-click="liveMuted = !liveMuted"
          />
          <span class="pv-cap">真实交互（播放暂停 / 静音切换 / 进度可拖）</span>
        </div>
        <div class="pv-demo-cell">
          <FloatBar mode="play" disabled />
          <span class="pv-cap">转码态（disabled：控件禁用 + 进度区「转码中…」，2026-09-09 接入轮补）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">倍率播放弹层（选中 = 品牌底 + 一级；hover = 通用/六级）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <RatePop model-value="1x" force-hover="4x" />
          <span class="pv-cap">常态 · 选中 1x / hover 4x（force 静态）</span>
        </div>
        <div class="pv-demo-cell">
          <RatePop v-model="liveRate" />
          <span class="pv-cap">真实交互（当前 {{ liveRate }}）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">音量滑块弹层（宽 124 稿值；轨道 通用/四级，拖钮 二级 + 1px 四级描边）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <VolumePop v-model="liveVolume" />
          <span class="pv-cap">真实交互（点击跳值 / 拖动，{{ Math.round(liveVolume * 100) }}%）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">条形滑块（BarSlider 三稿合一：播放条进度 light / 音量 dark / 工具栏尺寸 size——轨道 通用/三级 + 拖钮无描边，7.1 内容容器轮加）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <BarSlider v-model="sliderLight" tone="light" />
          <span class="pv-cap">light · 播放条进度（轨道 亮色/半透明-3 + 拖钮同档描边）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <BarSlider v-model="sliderDark" tone="dark" />
          <span class="pv-cap">dark · 音量（轨道 通用/四级 + 拖钮同档描边）</span>
        </div>
        <div class="pv-demo-cell" style="width: 100px">
          <BarSlider v-model="sliderSize" tone="size" />
          <span class="pv-cap">size · 工具栏尺寸（轨道 通用/三级，拖钮无描边；稿宽 100）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">帧条 + 播放条（帧条宽与播放条一致、间距 12——稿 prose；选中帧 = 亮色/一级 1px 边框，2026-09-09 四轮定稿）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-combo" :style="{ width: `${COMBO_W}px` }">
            <FrameStrip :current="1" />
            <FloatBar mode="play" :width="COMBO_W" :progress="0.2" />
          </div>
          <span class="pv-cap">帧图 129×76（二级底 + 三级描边）；上/下帧 = 尖括号（下一帧 flip-h）；第 2 格 = 选中帧静态陈列</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">笔记弹层（533×378 稿值；打开即聚焦输入；选中 item = 半透明-2 + 文字二级）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <NotesPop :notes="noteList" :selected="noteSel" v-model:draft="noteDraft" @select="noteSel = $event" @remove="noteRemove" @confirm="noteConfirm" @cancel="noteDraft = ''" />
          <span class="pv-cap">真实交互（点选 / × 删 / 确认追加到列表）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">通知栏 + 选择模式横幅（通知栏稿两枚：同族胶囊同锚点——列表容器顶部居中，横幅在上、通知在下）</h3>
      <p class="pv-group-note">
        通知栏 = 通用/半透明-2 + 大阴影 + 圆角 99：状态圆点 22（成功 ✓ / 失败 ✗ 单图标按需）+ 正文 一级 + 可选动作（撤销，按钮文字-大）+ 关闭 ✕ 二级；
        出入场 = 上→下位移 <b>250ms</b> 缓入缓出。横幅 = 品牌/半透明 + 圆角 99：滑入 <b>300ms</b> 后 2.6s 循环浮动（独立合成层修文字抖动）。
        下方为真实例（演示通知 duration 60s 不自动消失，刷新页面复位；失败通知带提示音）。
      </p>
      <div class="pv-demo-row">
        <button class="pv-switch" @click="demoToast('success')">成功通知</button>
        <button class="pv-switch" @click="demoToast('fail')">失败通知</button>
        <button class="pv-switch" @click="demoToast('undo')">带「撤销」动作</button>
      </div>
      <div class="pv-notice-stage">
        <!-- 复用全局 .notice-anchor（实机挂 DockHost / 预览窗口 App 层）；舞台内锚点下沉从简（实机 top 64 贴工具栏下） -->
        <div class="notice-anchor">
          <PickerBanner />
          <ToastStack />
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">悬停 tip（壳 = 半透明-1 + 1px 半透明-3 + 小阴影 + blur + 圆角 6；快捷键/副行 = 数字档 三级原文直显，kbd 徽章废除）</h3>
      <p class="pv-group-note">
        悬停 400ms 出、元素下方居中（空间不足翻上方、水平钳制窗口内）、点击即隐藏；标题 = 次要文字-加粗 二级。
        下方按钮为真实交互实例，静态验收由探针驱动截图。
      </p>
      <div class="pv-demo-row">
        <button class="pv-switch" v-tip="{ text: '添加文件/档案链接', keys: 'Ctrl+L' }">悬停 · 横排（标题 + 快捷键）</button>
        <button class="pv-switch" v-tip="{ text: '雪牧_自动.fla', keys: '1.2 MB', column: true }">悬停 · 竖排（卡片名称 + 简介）</button>
        <button class="pv-switch" v-tip="'纯文字'">悬停 · 纯文字</button>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">悬停-链接列表（LinkListPopover 真实例；宽随内容上限 360；行 = 图标+全路径 / 显示名+徽章 双行，hover 底 半透明-3）</h3>
      <p class="pv-group-note">
        行为与实机一致：150ms 延迟出、徽章上方右对齐（不足翻下）；行间 1px 半透明-3 顶线（首行去线）。
        演示数据 = 同一资产两条活位置（origin ★ / 链接 icon）；行跳转接真实业务，演示页内勿点行。
      </p>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-demo-row">
            <button ref="popAnchor" class="pv-switch">★ 徽章锚点</button>
            <button class="pv-switch" @click="showPop = !showPop">{{ showPop ? "关闭浮层" : "显示浮层" }}</button>
          </div>
          <LinkListPopover v-if="showPop" :asset="demoAsset" :anchor="popAnchor" />
          <span class="pv-cap">浮层在锚点上方右对齐（fixed body 级，滚动后请重开）；双行：图标 18 + 路径（首段 二级 / 后续 三级）｜显示名 三级 + 徽章</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* morph 演示的切换钮（预览页自身控件，dogfood 新体系 token） */
.pv-switch {
  padding: var(--n-space-4) var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g3);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
.pv-switch:hover {
  background: var(--n-g4);
  color: var(--n-l1);
}

/* morph 演示舞台居中——实机布局口径（中间面板底部居中）：壳宽过渡逐帧重排，展开即中心向两侧 */
.pv-morph-stage {
  justify-content: center;
  width: 100%;
}

/* 帧条 + 播放条同宽组合：列排 gap 12（稿 prose 间距），两者同宽 777 */
.pv-combo {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}

/* 通知栏 / 横幅演示舞台：relative 供 .notice-anchor 锚定；锚点下沉从简（实机 top 64 贴工具栏下） */
.pv-notice-stage {
  position: relative;
  width: 100%;
  height: 260px;
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
.pv-notice-stage .notice-anchor {
  top: 12px;
}
</style>
