<script setup lang="ts">
// 选择模式横幅（阶段七稿：docs/06_UI重构/Design Token/组件/悬浮栏/通知栏/通知栏-选择模式）：
// 品牌/半透明底胶囊 + 立体感-背景模糊-大阴影 + blur 小 + 圆角 99px（稿值）；高 46 / padding 12 16 / gap 12。
// 结构 = 标题「选择模式」（按钮文字-大 一级）+ 目的文案（正文 一级，picker.bannerText）+「ESC 退出」（次要文字-加粗 二级）。
// 动画（稿 prose）：从上至下滑入 300ms → 随后 2.6s 循环上下浮动 ±6px（引起注目）。
// 字体抖动修复（稿备注「动画期间字体抖动」，2026-08-29）：滑入与循环分挂两层，循环层恒 composited
// （will-change: transform + translate3d 关键帧），文字随层位移动画、不逐帧重栅格化。
// 定位归 .notice-anchor（base.css）：与通知条统一锚点、横幅在上（2026-08-29 用户定）；纯展示不接收事件。
import { usePickerStore } from "../stores/picker";

const picker = usePickerStore();
</script>

<template>
  <!-- 三层分工：enter = 一次性滑入；float = 无限循环（恒 composited 防文字抖动）；banner = 视觉胶囊本体 -->
  <div class="banner-enter">
    <div class="banner-float">
      <div class="banner">
        <span class="title">选择模式</span>
        <span class="text">{{ picker.bannerText }}</span>
        <span class="esc">ESC 退出</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 一次性滑入（从上至下） */
.banner-enter {
  animation: banner-in var(--n-dur-300) var(--n-ease-in-out);
}
/* 无限循环浮动：独立合成层，文字只栅格化一次（抖动修复核心） */
.banner-float {
  will-change: transform;
  transform: translate3d(0, 0, 0);
  animation: banner-float 2.6s var(--n-ease-in-out) 0.35s infinite;
}
/* 胶囊本体：品牌/半透明 + 大阴影 + inset 高光 + blur + 圆角 99（稿无 border，旧 1px 亮-选中边框废除） */
.banner {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  height: 46px;
  padding: var(--n-space-12) var(--n-space-16);
  gap: var(--n-space-12);
  border-radius: 99px;
  background: var(--n-brand-50);
  box-shadow: var(--n-effect-raised-blur);
  backdrop-filter: blur(var(--n-blur-sm));
  white-space: nowrap;
  pointer-events: none;
}
/* 标题 = 按钮文字-大 一级 */
.title {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
/* 目的文案 = 正文 一级 */
.text {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
}
/* ESC 退出 = 次要文字-加粗 二级 */
.esc {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
@keyframes banner-in {
  from {
    transform: translate3d(0, -48px, 0);
    opacity: 0;
  }
  to {
    transform: translate3d(0, 0, 0);
    opacity: 1;
  }
}
@keyframes banner-float {
  0%,
  100% {
    transform: translate3d(0, 0, 0);
  }
  50% {
    transform: translate3d(0, -6px, 0);
  }
}
</style>
