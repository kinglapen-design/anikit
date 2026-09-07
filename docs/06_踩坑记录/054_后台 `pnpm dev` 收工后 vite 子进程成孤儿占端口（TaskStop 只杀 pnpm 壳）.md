## 054 后台 `pnpm dev` 收工后 vite 子进程成孤儿占端口（TaskStop 只杀 pnpm 壳）

- **现象：** 2026-08-28（阶段七预览页截图）。后台任务 `pnpm dev` 用 TaskStop 收工显示 killed、netstat 查端口也已释放，过几分钟再起 `pnpm dev` 却撞 `EADDRINUSE`——vite 子进程（独立 node PID）在 TaskStop 后才脱离壳进程继续监听，首次查端口时它还没完全起来/暂未 LISTEN，漏检。
- **解法：** 按端口清：`netstat -ano | findstr :5373` 找 LISTENING 的 PID（验明 node.exe 再杀），`Stop-Process -Id <PID> -Force`。本轮两次孤儿均此处置。
- **预防：** 后台起 dev server 收工时，TaskStop 后必须 netstat 复验端口真释放（最好等 2-3s 再查一次，孤儿绑定有延迟）；再起服务前同样先查端口，别信"刚杀过"。截图/探针类一次性用途优先用 `timeout` 短的后台任务或直接前台跑完即走。
