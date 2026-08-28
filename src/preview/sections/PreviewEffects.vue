<script setup lang="ts">
// Token.md 3_效果 全量：立体感三态 / 大小阴影 / 背景模糊两档（模糊变量配 backdrop-filter 演示）
import TokenVar from "../TokenVar.vue";

interface ShadowDemo {
  name: string;
  note: string;
  cardClass: string;
}

const shadows: ShadowDemo[] = [
  { name: "立体感-通用", note: "内阴影 0 1 1 白20% + 投影 0 2 1 黑20%", cardClass: "raised" },
  { name: "立体感-主要按钮", note: "内阴影 0 1 1 白40% + 投影 0 2 1 黑20%", cardClass: "raised-primary" },
  { name: "大阴影", note: "投影 0 4 40 黑60%", cardClass: "shadow-lg" },
  { name: "小阴影", note: "投影 0 4 20 黑40%", cardClass: "shadow-sm" },
];

interface BlurDemo {
  name: string;
  note: string;
  panelClass: string;
}

const blurs: BlurDemo[] = [
  { name: "立体感-背景模糊", note: "通用立体感 + blur(--n-blur-sm)", panelClass: "blur-raised" },
  { name: "立体感-背景模糊-大阴影", note: "大阴影立体感 + blur(--n-blur-sm)", panelClass: "blur-raised-lg" },
  { name: "背景模糊-小", note: "仅 backdrop blur(--n-blur-sm)（figma 80 减半）", panelClass: "blur-sm" },
  { name: "背景模糊-大", note: "仅 backdrop blur(--n-blur-lg)（figma 250 减半）", panelClass: "blur-lg" },
];
</script>

<template>
  <section id="design-system-effects" class="pv-section">
    <h2 class="pv-sec-title">效果</h2>
    <p class="pv-sec-desc">
      Token.md 第 3 节；背景模糊 = figma 值减半落地。主窗口级模糊正式方案为 Windows 云母，此处仅演示 CSS 口径。
    </p>
    <div class="pv-group">
      <h3 class="pv-group-title">立体感 / 阴影</h3>
      <div class="cards">
        <div v-for="d in shadows" :key="d.name" class="demo">
          <div class="card" :class="d.cardClass">
            <span class="card-label">{{ d.cardClass === 'raised-primary' ? '主按钮' : '卡片' }}</span>
          </div>
          <span class="nm">{{ d.name }}</span>
          <span class="note">{{ d.note }}</span>
        </div>
      </div>
    </div>
    <div class="pv-group">
      <h3 class="pv-group-title">背景模糊</h3>
      <div class="cards">
        <div v-for="d in blurs" :key="d.name" class="demo">
          <div class="backdrop">
            <div class="panel" :class="d.panelClass" />
          </div>
          <span class="nm">{{ d.name }}</span>
          <span class="note">{{ d.note }}</span>
          <TokenVar :token="d.panelClass === 'blur-lg' ? '--n-blur-lg' : '--n-blur-sm'" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: var(--n-space-24);
  margin-top: var(--n-space-12);
}
.demo {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
.card {
  height: 96px;
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  display: flex;
  align-items: center;
  justify-content: center;
}
.card.raised {
  box-shadow: var(--n-effect-raised);
}
.card.raised-primary {
  background: var(--n-brand);
  box-shadow: var(--n-effect-raised-primary);
}
.card.shadow-lg {
  box-shadow: var(--n-shadow-lg);
}
.card.shadow-sm {
  box-shadow: var(--n-shadow-sm);
}
.card-label {
  color: var(--n-l1);
  font-size: var(--n-fs-btn);
  line-height: var(--n-lh-btn);
}
.backdrop {
  height: 96px;
  border-radius: var(--n-radius-8);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  /* 彩色底：借自定义色环验证 backdrop 模糊 */
  background: linear-gradient(
    135deg,
    var(--n-custom-red-1) 0%,
    var(--n-custom-yellow-1) 20%,
    var(--n-custom-green-1) 40%,
    var(--n-custom-cyan-1) 60%,
    var(--n-custom-blue-1) 80%,
    var(--n-custom-purple-1) 100%
  );
}
.panel {
  width: 70%;
  height: 56px;
  border-radius: var(--n-radius-6);
  background: var(--n-g-a2);
}
.panel.blur-raised {
  box-shadow: var(--n-effect-raised);
  backdrop-filter: blur(var(--n-blur-sm));
}
.panel.blur-raised-lg {
  box-shadow: var(--n-effect-raised-blur);
  backdrop-filter: blur(var(--n-blur-sm));
}
.panel.blur-sm {
  backdrop-filter: blur(var(--n-blur-sm));
}
.panel.blur-lg {
  backdrop-filter: blur(var(--n-blur-lg));
}
.nm {
  color: var(--n-l1);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.note {
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
</style>
