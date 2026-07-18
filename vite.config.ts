import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri 等待 dev server 时按 127.0.0.1 探测，
// vite 默认绑 ::1 会导致一直 Waiting——显式绑定 IPv4 回环。
export default defineConfig({
  plugins: [vue()],
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    watch: {
      // cargo 构建产物会被 vite 监视并报 EBUSY，必须排除
      ignored: ["**/src-tauri/**"],
    },
  },
});
