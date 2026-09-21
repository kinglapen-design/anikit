<script setup lang="ts">
// 组件 输入框展区——docs/06_UI重构/Design Token/组件/输入框/ 大 / 小两稿的展示与验收面。
// force-focus = 聚焦态静态验收；每档附真实可点实例（focus-within 出聚焦描边）。
// 聚焦描边 = 亮色/半透明-1，opacity 过渡 150ms；hover 稿无不做；禁用态 = 30% 透明度（2026-09-20 控件族口径，§39②）。
// 占位符 亮色/四级（2026-08-29 用户定）。
import { ref } from "vue";
import TextInput from "../../components/inputs/TextInput.vue";

const liveLg = ref("");
const liveSm = ref("");
const liveClear = ref("雪牧_自动");
</script>

<template>
  <section id="design-system-inputs" class="pv-section">
    <h2 class="pv-sec-title">组件 · 输入框</h2>
    <p class="pv-sec-desc">
      Design Token/组件/输入框/ 大 / 小两稿合一 TextInput：通用二级底 + 圆角 8，聚焦描边 亮色/半透明-1（<b>150ms</b> opacity，
      双层伪元素 inset 0 同卡片线条方案）；占位符 亮色/四级（2026-08-29 用户定）；hover 稿无不做；禁用态 30% 透明度（§39②）。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">输入框-大（高 34 / padding 8/12 / 常态描边 通用三级）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput placeholder="名称" />
          <span class="pv-cap">常态 · 占位符</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput model-value="雪牧_自动" />
          <span class="pv-cap">常态 · 已填</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput model-value="雪牧_自动" force-state="focus" />
          <span class="pv-cap">聚焦（亮色/半透明-1）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput v-model="liveLg" placeholder="点击聚焦试试" />
          <span class="pv-cap">真实交互（可输入）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput model-value="" placeholder="http://" trailing-icon="link" readonly />
          <span class="pv-cap">trailingIcon 右置装饰（详情面板链接框）+ 只读（7.1 右栏轮扩）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput model-value="" placeholder="搜索文件夹..." leading-icon="search" />
          <span class="pv-cap">leadingIcon 左置装饰 18 亮色/三级（移动弹窗搜索框稿，2026-09-08 扩）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput v-model="liveClear" placeholder="输入后出清除钮" leading-icon="search" clearable />
          <span class="pv-cap">clearable 清除钮（clear 12 亮色/四级 hover 升三级；烧 AppInput/AppSearchBar 补，2026-09-20）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <TextInput model-value="全部" readonly :focusable="false" />
          <span class="pv-cap">focusable=false：聚焦描边不显（快速访问固定入口框，2026-08-31 用户定）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">输入框-小（高 26 / padding 4/12 / 描边 亮色/半透明-3 / 文字左对齐 2026-09-21 用户定，稿居中特化废）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 55px">
          <TextInput size="sm" placeholder="年" />
          <span class="pv-cap">常态 · 占位（稿 55 宽）</span>
        </div>
        <div class="pv-demo-cell" style="width: 55px">
          <TextInput size="sm" model-value="2026" />
          <span class="pv-cap">常态 · 已填</span>
        </div>
        <div class="pv-demo-cell" style="width: 55px">
          <TextInput size="sm" model-value="2026" force-state="focus" />
          <span class="pv-cap">聚焦</span>
        </div>
        <div class="pv-demo-cell" style="width: 120px">
          <TextInput size="sm" v-model="liveSm" placeholder="左对齐" />
          <span class="pv-cap">交互 · 宽随父容器</span>
        </div>
      </div>
    </div>
  </section>
</template>
