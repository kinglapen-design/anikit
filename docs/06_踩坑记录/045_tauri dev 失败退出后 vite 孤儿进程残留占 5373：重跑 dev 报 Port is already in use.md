## 045 tauri dev 失败退出后 vite 孤儿进程残留占 5373：重跑 dev 报 Port is already in use

- **现象：** 2026-08-20 tauri dev 因后端编译失败（踩坑 033 那次）退出后，重跑报 `Error: Port 5373 is already in use`，beforeDevCommand 阶段即败。
- **原因：** beforeDevCommand 起的 vite 不随 tauri CLI 退出而被清理——DevCommand（cargo run）失败让主进程退出时，先行的 vite 变孤儿常驻，端口不放。
- **解法：** `netstat -ano | grep ":5373.*LISTENING"` 拿 PID → `tasklist //FI "PID eq <pid>"` 确认是 node.exe → `taskkill //PID <pid> //F`；`curl -s -o /dev/null -w "%{http_code}" http://localhost:5373` 回 000 即释放，重跑 dev 正常。
- **预防：** dev 失败退出后重跑前先看一眼 5373；与"后台 dev server 交付即停"规约（e812b15）同族——凡经手起停 dev，收尾确认无残留。
