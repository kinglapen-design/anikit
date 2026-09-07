## 056 CDP 探针连上历史僵尸 headless Edge——固定调试端口 + terminate 只杀父进程（旧页面旧模块鬼状态）

- **现象：** 2026-08-29（阶段七十五轮，悬浮栏走查修复验证）。探针连跑四轮全 FAIL 且证据互相矛盾：静态宽度新旧值混杂（force-focus 实例一会儿 450 一会儿 250）、console 异常时间戳逐轮**完全雷同**、组件 setup 报 `watch is not defined`（磁盘文件明明已导入）。同代码用一次性 debug 脚本（随机端口）全正常。
- **根因：** `subprocess.Popen` + `proc.terminate()` 在 Windows 只杀 Edge 父进程，**headless 子进程树整体残留**（数轮攒了上百个僵尸）；新探针用固定端口 9394——新 Edge 绑端口失败静默退出，`/json` 落到**旧僵尸实例**上：旧页面 + 新旧混合模块 + 陈旧 console 事件，断言全跑在鬼状态上。
- **解法：** 清杀按命令行特征匹配（`Win32_Process` 过滤 `user-data-dir` 含 `edge-cdp`）；探针收尾改 `taskkill /F /T /PID` 整树；调试端口与 profile 按 PID 唯一化 + `Network.setCacheDisabled`。
- **预防：** CDP 探针一律整树杀 + 唯一端口；**探针结果与手写 debug 脚本矛盾时，先查连上的是不是自己起的实例**（时间戳雷同 = 旧实例铁证）。关联 054（后台进程残留同族）；headless 无窗口焦点下 `focus()` 不发 focus 事件，交互驱动用合成事件（速查表 §18 末条）。
