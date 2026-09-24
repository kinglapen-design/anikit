<script setup lang="ts">
// MarkDown 排版展区（稿 = Design Token/布局/主窗口/中间面板/中间面板-内容容器-文本编辑）：
// 全局类 styles/markdown.css（.md-body 族）的还原验收面——静态复刻稿全元素：
// H1（+ 2px 分割线）/ H2-H4 / 正文 / 引用（单行·多行）/ 代码块（+ 右上复制钮）/
// 任务列表（.kit-check-box 复选框，组件/按钮/复选框稿通用类）/ 普通列表 / 行内代码 / 分割线 / 表格。
// 结构与 lib/markdown.ts 渲染输出同形（md-task 悬挂缩进、md-code 包装），静态展示不接交互。
import AppIcon from "../../components/AppIcon.vue";
</script>

<template>
  <section id="design-system-markdown" class="pv-section">
    <h2 class="pv-sec-title">MarkDown 排版（文本编辑稿）</h2>
    <p class="pv-sec-desc">
      稿 = 布局/主窗口/中间面板-内容容器-文本编辑；样式 = styles/markdown.css 全局类（PreviewText md 渲染态 /
      PreviewDocx 共用）。版心 = 容器 padding 12 + 内容 72（左右各 84）、块间 gap 12；H1 带 2px 通用/三级分割线；
      任务列表 = .kit-check-box 复选框通用类（hover 描边升通用/六级、对勾 250ms 过冲弹出——本区可真实 hover 验收）。
    </p>
    <div class="md-stage">
      <div class="md-body">
        <h1>方舟前史</h1>
        <h2>1._创世纪</h2>
        <h3>1.1_亚当与夏娃</h3>
        <p>夏娃摘下一颗苹果，开启了人类的时代。</p>
        <blockquote>
          <p>夏娃摘下一颗苹果，开启了人类的时代。</p>
        </blockquote>
        <p>
          夏娃和亚当生下亚伯和该隐，该隐嫉妒亚伯，杀害亚伯，神降下诅咒，该隐一族无法通过劳作获取食物，流放至挪得之地，你的族人不会因饥饿而死，但会时刻感到饥饿，我在你眉心留下印记，让别人能认出你，防止他人杀你，杀该隐必遭七倍的反噬，该隐将永世存活下去，无休止接受饥饿的惩罚，但又无法劳作。
        </p>
        <blockquote>
          <p>
            该隐与他兄弟亚伯说话，二人正在田间，该隐起来打他兄弟亚伯，把他杀了。耶和华对该隐说：“你兄弟亚伯在哪里？”，他说：“我不知道！我岂是看守我兄弟的吗？”。耶和华说：“你做了什么事呢？你兄弟的血的声音通过地里向我哀告。地开了口，从你手里接受你兄弟的血。你必从这地受诅咒。你种地，地不再给你效力，你必流离飘荡在地上。”
          </p>
        </blockquote>
        <p>神对诺亚说：</p>
        <div class="md-code">
          <pre><code>// AniKit 前端入口（阶段 1：三栏主窗口骨架）
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";

const app = createApp(App);
app.use(createPinia());
app.mount("#app");</code></pre>
          <button type="button" class="md-copy"><AppIcon name="copy" :size="18" /></button>
        </div>
        <h4>2._任务列表</h4>
        <ul>
          <li class="md-task">
            <span class="kit-check-box on"><AppIcon name="check-tiny" :size="10" class="kit-check-mark" /></span>已完成的任务（对勾弹出 250ms 过冲）
          </li>
          <li class="md-task">
            <span class="kit-check-box"><AppIcon name="check-tiny" :size="10" class="kit-check-mark" /></span>未完成的任务（hover 框描边升档）
          </li>
          <li class="md-task">
            <span class="kit-check-box"><AppIcon name="check-tiny" :size="10" class="kit-check-mark" /></span>带行内代码的任务 <code>pnpm dev</code>
          </li>
        </ul>
        <h4>3._列表与其他元素</h4>
        <ul>
          <li>无序列表项一</li>
          <li>无序列表项二</li>
        </ul>
        <ol>
          <li>有序列表项一</li>
          <li>有序列表项二</li>
        </ol>
        <hr />
        <table>
          <thead>
            <tr>
              <th>名称</th>
              <th>数值</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>正文</td>
              <td>15px / 25px</td>
            </tr>
            <tr>
              <td>代码块</td>
              <td>14px / 24px</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 舞台 = 面板容器模拟（通用/一级底 + 细描边，排版由 md-body 自带 padding 供给） */
.md-stage {
  margin-top: var(--n-space-16);
  border: 1px solid var(--n-l-a3);
  border-radius: var(--n-radius-8);
  overflow: hidden;
}
</style>
