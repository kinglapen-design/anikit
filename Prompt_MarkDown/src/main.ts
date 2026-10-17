import { createApp } from "vue";
import App from "./App.vue";
import "./styles/tokens.css";
import "./styles/app.css";

document.documentElement.dataset.theme =
  localStorage.getItem("promptmd-theme") || "light";

createApp(App).mount("#app");
