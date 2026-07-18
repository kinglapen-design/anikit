// AniKit 前端入口（阶段 1：三栏主窗口骨架）
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/tokens.css";
import "./styles/base.css";

createApp(App).use(createPinia()).mount("#app");
