<script setup lang="ts">
// Token.md 2_字体 全量：通用十档 + MarkDown 六档；加粗档与同组常规档共用字号/行高
import TokenVar from "../TokenVar.vue";

interface TypeRow {
  name: string;
  size: string; // 字号变量
  line: string; // 行高变量
  weight: number;
  mono?: boolean;
  expect: string; // Token.md 原文规格
  sample: string;
}

const SAMPLE_TEXT = "动画师的素材管理软件 AniKit 0123456789";
const SAMPLE_NUM = "0123456789 12:34:56 108f";

const common: TypeRow[] = [
  { name: "正文", size: "--n-fs-body", line: "--n-lh-body", weight: 400, expect: "13px / 18px / Regular", sample: SAMPLE_TEXT },
  { name: "正文-加粗", size: "--n-fs-body", line: "--n-lh-body", weight: 700, expect: "13px / 18px / Bold", sample: SAMPLE_TEXT },
  { name: "次要文字", size: "--n-fs-secondary", line: "--n-lh-secondary", weight: 400, expect: "12px / 16px / Regular", sample: SAMPLE_TEXT },
  { name: "次要文字-加粗", size: "--n-fs-secondary", line: "--n-lh-secondary", weight: 700, expect: "12px / 16px / Bold", sample: SAMPLE_TEXT },
  { name: "数字", size: "--n-fs-num", line: "--n-lh-num", weight: 500, mono: true, expect: "12px / 16px / Fira Mono Medium", sample: SAMPLE_NUM },
  { name: "数字-大", size: "--n-fs-num-lg", line: "--n-lh-num-lg", weight: 500, mono: true, expect: "13px / 18px / Fira Mono Medium", sample: SAMPLE_NUM },
  { name: "按钮文字", size: "--n-fs-btn", line: "--n-lh-btn", weight: 400, expect: "15px / 20px / Regular", sample: "确认 取消 上一步" },
  { name: "按钮文字-大", size: "--n-fs-btn-lg", line: "--n-lh-btn-lg", weight: 700, expect: "16px / 22px / Bold", sample: "确认 取消" },
  { name: "标题", size: "--n-fs-title", line: "--n-lh-title", weight: 700, expect: "18px / 26px / Bold", sample: SAMPLE_TEXT },
  { name: "超大标题", size: "--n-fs-display", line: "--n-lh-display", weight: 700, expect: "36px / 46px / Bold", sample: "AniKit 素材库" },
];

const markdown: TypeRow[] = [
  { name: "H1", size: "--n-fs-md-h1", line: "--n-lh-md-h1", weight: 700, expect: "30px / 40px / Bold", sample: "一级标题" },
  { name: "H2", size: "--n-fs-md-h2", line: "--n-lh-md-h2", weight: 700, expect: "26px / 36px / Bold", sample: "二级标题" },
  { name: "H3", size: "--n-fs-md-h3", line: "--n-lh-md-h3", weight: 700, expect: "22px / 32px / Bold", sample: "三级标题" },
  { name: "H4-H6", size: "--n-fs-md-h4", line: "--n-lh-md-h4", weight: 700, expect: "18px / 28px / Bold", sample: "四级标题" },
  { name: "正文", size: "--n-fs-md-body", line: "--n-lh-md-body", weight: 400, expect: "15px / 25px / Regular", sample: SAMPLE_TEXT },
  { name: "代码块", size: "--n-fs-md-code", line: "--n-lh-md-code", weight: 400, mono: true, expect: "14px / 24px / Fira Mono Regular", sample: "const fps = 24;" },
];
</script>

<template>
  <section id="design-system-type" class="pv-section">
    <h2 class="pv-sec-title">字体</h2>
    <p class="pv-sec-desc">
      Token.md 第 2 节；Fira Mono 未内嵌（非 Windows 系统字体），当前走 Consolas 兜底，待补字体文件。
    </p>
    <div class="pv-group">
      <h3 class="pv-group-title">2.1 通用</h3>
      <div class="rows">
        <div v-for="r in common" :key="r.name" class="row">
          <div class="meta">
            <span class="nm">{{ r.name }}</span>
            <span class="spec">{{ r.expect }}</span>
            <TokenVar :token="r.size" />
            <TokenVar :token="r.line" />
          </div>
          <div
            class="sample"
            :style="{
              fontSize: `var(${r.size})`,
              lineHeight: `var(${r.line})`,
              fontWeight: r.weight,
              fontFamily: r.mono ? 'var(--n-font-mono)' : 'var(--n-font-family)',
            }"
          >
            {{ r.sample }}
          </div>
        </div>
      </div>
    </div>
    <div class="pv-group">
      <h3 class="pv-group-title">2.2 MarkDown</h3>
      <div class="rows">
        <div v-for="r in markdown" :key="r.name" class="row">
          <div class="meta">
            <span class="nm">{{ r.name }}</span>
            <span class="spec">{{ r.expect }}</span>
            <TokenVar :token="r.size" />
            <TokenVar :token="r.line" />
          </div>
          <div
            class="sample"
            :style="{
              fontSize: `var(${r.size})`,
              lineHeight: `var(${r.line})`,
              fontWeight: r.weight,
              fontFamily: r.mono ? 'var(--n-font-mono)' : 'var(--n-font-family)',
            }"
          >
            {{ r.sample }}
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.rows {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
  margin-top: var(--n-space-12);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--n-space-24);
  padding: var(--n-space-12) var(--n-space-16);
  border-radius: var(--n-radius-6);
  background: var(--n-g2);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
.meta {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  width: 240px;
  flex: none;
}
.nm {
  color: var(--n-l1);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
}
.spec {
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
.sample {
  color: var(--n-l1);
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
