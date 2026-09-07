## 020 vite 起不来：listen EACCES——端口被 Windows NAT 动态保留段覆盖

- **现象：** `pnpm tauri dev` 失败：`beforeDevCommand terminated with non-zero status`；手动 `pnpm dev` 报 `Error: listen EACCES: permission denied 127.0.0.1:5173`——不是 EADDRINUSE（无进程占用 5173），昨天还能正常 dev。
- **原因：** Windows NAT（Hyper-V / WSL / Docker 的 winnat 服务）会动态保留 TCP 端口段，`netsh interface ipv4 show excludedportrange protocol=tcp` 可见 5173 落在 5144-5243 保留段内；保留段随系统重启 / winnat 重启漂移，所以"昨天好的今天不行"。
- **解法：** 换保留段外端口：vite.config.ts `server.port` 5173 → **5373** + tauri.conf.json `devUrl` 同步（两文件必须一起改，否则 Tauri 一直 Waiting for frontend dev server，同踩坑 002 的联动）。
- **预防：** dev 报 EACCES（非 EADDRINUSE）先查 excludedportrange 再查占用；端口变更必须 vite.config.ts 与 tauri.conf.json 同步；若保留段漂移再次覆盖 5373，同法再换（也可用 `net stop winnat` 重置保留段，需管理员且重启后可能复发，不作首选）。
