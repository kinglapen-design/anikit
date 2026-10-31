import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri 等待 dev server 时按 127.0.0.1 探测，
// vite 默认绑 ::1 会导致一直 Waiting——显式绑定 IPv4 回环。
export default defineConfig({
  plugins: [vue()],
  server: {
    host: "127.0.0.1",
    // 端口 5373：原 5173 被 Windows NAT/Hyper-V 动态保留段（5144-5243）覆盖，
    // listen 报 EACCES（踩坑 020）；保留段随系统重启漂移，选段外端口
    port: 5373,
    strictPort: true,
    watch: {
      // cargo 构建产物会被 vite 监视并报 EBUSY，必须排除；离线模型、发布产物与隔离验收库也不是前端模块。
      // .pnpm-store = 项目内 pnpm 包存储（2026-09-13 迁入，pnpm-workspace.yaml storeDir），内容不变、无需监视
      ignored: ["**/src-tauri/**", "**/.pnpm-store/**", "**/assets/models/**", "**/release/**", "**/tools/style-preview/semantic-e2e/**"],
    },
  },
});
