<script setup lang="ts">
// 组件 悬浮栏展区——docs/UI 重构/Design Token/组件/悬浮栏/多功能悬浮栏/ 八稿的展示与验收面。
// 搜索栏 / 搜索栏展开成筛选 / 播放条 / MarkDown 工具栏 = 同一容器（FloatBar）：
// 切换宽度平滑伸缩 + 功能按钮 Y 轴出场入场 = 250ms 小位移 + 位移 4px（2026-08-29 用户定）；hover / 颜色 150ms。
// 无需悬浮栏 = 宽度 + 透明度退场（稿 prose，visible prop）。
// 播放/暂停、声音/静音 = 单钮按态切换（稿两态并列是全量画法）；进度条无填充段（稿未画，进度靠拖钮位置）；
// 循环/笔记开启态稿未画记 09 待办；图标 hover 升二级 = 按钮族 IconButton bare 口径（按钮展区已验收，此不重复 force 列）。
import { ref } from "vue";
import FloatBar from "../../components/bars/FloatBar.vue";
import FrameStrip from "../../components/bars/FrameStrip.vue";
import RatePop from "../../components/bars/RatePop.vue";
import VolumePop from "../../components/bars/VolumePop.vue";
import NotesPop from "../../components/bars/NotesPop.vue";

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
      </p>
      <div class="pv-demo-row">
        <button class="pv-switch" @click="morphMode = 'search'">搜索栏</button>
        <button class="pv-switch" @click="morphMode = 'markdown'">MarkDown 工具栏</button>
        <button class="pv-switch" @click="morphMode = 'play'">播放条</button>
        <button class="pv-switch" @click="morphVisible = !morphVisible">
          {{ morphVisible ? "退场（宽度+透明度）" : "入场" }}
        </button>
      </div>
      <div class="pv-demo-row">
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
      <h3 class="pv-group-title">播放条（全量陈列；时间码 数字-大：当前·分隔 二级 / 总计 三级；进度条无填充段）</h3>
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
      <h3 class="pv-group-title">帧条 + 播放条（帧条宽与播放条一致、间距 12——稿 prose）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-combo" :style="{ width: `${COMBO_W}px` }">
            <FrameStrip />
            <FloatBar mode="play" :width="COMBO_W" :progress="0.2" />
          </div>
          <span class="pv-cap">帧图 129×76（二级底 + 三级描边）；上/下帧 = 尖括号（下一帧 flip-h）</span>
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

/* 帧条 + 播放条同宽组合：列排 gap 12（稿 prose 间距），两者同宽 777 */
.pv-combo {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
</style>
