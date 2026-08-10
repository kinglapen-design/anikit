// AniKit 前端入口（阶段 1：三栏主窗口骨架）
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { vTip } from "./lib/tip";
import "./styles/tokens.css";
import "./styles/base.css";
import "dockview-core/dist/styles/dockview.css";
import "./styles/dockview-theme.css";

const app = createApp(App);
app.use(createPinia());
app.directive("tip", vTip);
app.mount("#app");
