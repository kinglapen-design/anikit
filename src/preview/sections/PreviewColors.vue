<script setup lang="ts">
// Token.md 1_颜色 全量：品牌 / 通用 / 亮色 / 状态 / MarkDown / 自定义 / 黑白
import SwatchCell from "../SwatchCell.vue";

interface Group {
  title: string;
  note?: string;
  items: { name: string; token: string }[];
}

const groups: Group[] = [
  {
    title: "1.1 品牌",
    items: [
      { name: "常态", token: "--n-brand" },
      { name: "提亮", token: "--n-brand-bright" },
      { name: "半透明", token: "--n-brand-50" },
    ],
  },
  {
    title: "1.2 通用（暗色阶：窗口 / 容器底色）",
    items: [
      { name: "一级", token: "--n-g1" },
      { name: "二级", token: "--n-g2" },
      { name: "三级", token: "--n-g3" },
      { name: "四级", token: "--n-g4" },
      { name: "五级", token: "--n-g5" },
      { name: "六级", token: "--n-g6" },
      { name: "半透明-1", token: "--n-g-a1" },
      { name: "半透明-2", token: "--n-g-a2" },
      { name: "渐变-1", token: "--n-g-grad-1" },
      { name: "渐变-2（渐隐罩）", token: "--n-g-grad-2" },
    ],
  },
  {
    title: "1.3 亮色（文字 / 图标 / 线条叠加）",
    items: [
      { name: "一级", token: "--n-l1" },
      { name: "二级", token: "--n-l2" },
      { name: "三级", token: "--n-l3" },
      { name: "四级", token: "--n-l4" },
      { name: "禁用", token: "--n-l-disabled" },
      { name: "半透明-1", token: "--n-l-a1" },
      { name: "半透明-2", token: "--n-l-a2" },
      { name: "半透明-3", token: "--n-l-a3" },
      { name: "渐变-1", token: "--n-l-grad-1" },
    ],
  },
  {
    title: "1.4 状态",
    items: [
      { name: "成功", token: "--n-success" },
      { name: "警告", token: "--n-warning" },
      { name: "禁止", token: "--n-danger" },
    ],
  },
  {
    title: "1.5 MarkDown",
    items: [
      { name: "标题", token: "--n-md-title" },
      { name: "正文", token: "--n-md-body" },
      { name: "其他", token: "--n-md-other" },
    ],
  },
  {
    title: "1.6 自定义（-1 亮档 / -2 暗档）",
    items: [
      { name: "常态-1", token: "--n-custom-1" },
      { name: "常态-2", token: "--n-custom-2" },
      { name: "红色-1", token: "--n-custom-red-1" },
      { name: "红色-2", token: "--n-custom-red-2" },
      { name: "蓝色-1", token: "--n-custom-blue-1" },
      { name: "蓝色-2", token: "--n-custom-blue-2" },
      { name: "黄色-1", token: "--n-custom-yellow-1" },
      { name: "黄色-2", token: "--n-custom-yellow-2" },
      { name: "黄绿-1", token: "--n-custom-yellowgreen-1" },
      { name: "黄绿-2", token: "--n-custom-yellowgreen-2" },
      { name: "绿色-1", token: "--n-custom-green-1" },
      { name: "绿色-2", token: "--n-custom-green-2" },
      { name: "青色-1", token: "--n-custom-cyan-1" },
      { name: "青色-2", token: "--n-custom-cyan-2" },
      { name: "紫色-1", token: "--n-custom-purple-1" },
      { name: "紫色-2", token: "--n-custom-purple-2" },
      { name: "粉色-1", token: "--n-custom-pink-1" },
      { name: "粉色-2", token: "--n-custom-pink-2" },
      { name: "桃红-1", token: "--n-custom-peach-1" },
      { name: "桃红-2", token: "--n-custom-peach-2" },
    ],
  },
  {
    title: "1.7 黑白",
    items: [
      { name: "白", token: "--n-white" },
      { name: "黑", token: "--n-black" },
    ],
  },
];
</script>

<template>
  <section id="design-system-colors" class="pv-section">
    <h2 class="pv-sec-title">颜色</h2>
    <p class="pv-sec-desc">Token.md 第 1 节；渐变角度稿未注明，暂按 180deg（上→下）。</p>
    <div v-for="g in groups" :key="g.title" class="pv-group">
      <h3 class="pv-group-title">{{ g.title }}</h3>
      <div class="pv-swatch-grid">
        <SwatchCell v-for="it in g.items" :key="it.token" :name="it.name" :token="it.token" />
      </div>
    </div>
  </section>
</template>
