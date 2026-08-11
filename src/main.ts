// AniKit 前端入口（阶段 1：三栏主窗口骨架）
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { vTip } from "./lib/tip";
import "./styles/tokens.css";
import "./styles/base.css";
import "dockview-core/dist/styles/dockview.css";
import "./styles/dockview-theme.css";

// 禁用 Chromium 原生右键菜单（1.3，2026-08-11 用户定：应用内右键一律出自有菜单，输入框同禁）
window.addEventListener("contextmenu", (e) => e.preventDefault());

const app = createApp(App);
app.use(createPinia());
app.directive("tip", vTip);
app.mount("#app");
