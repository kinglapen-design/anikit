## 002 `tauri dev` 一直 Waiting for frontend dev server

- **现象：** vite 显示 ready，Tauri 却反复 "Waiting for your frontend dev server to start"。
- **原因：** vite 默认把 dev server 绑到 IPv6 的 `::1`，而 Tauri 按 `127.0.0.1` 探测，localhost 解析不一致。
- **解法：** vite.config.ts 显式 `server.host = "127.0.0.1"` + `strictPort`，tauri.conf.json 的 devUrl 同步改为 `http://127.0.0.1:5173`。
- **预防：** 该项目配置已固化在 vite.config.ts / tauri.conf.json，新机器克隆即用。
