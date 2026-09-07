## 029 vite 热更新"假生效"：文件已改、页面跑旧码，重启 app 窗口也没用

- **现象：** 2026-08-11 连环菜单走查中，多次"修复已提交但用户端看不到"——typical 路径：Edit 工具改文件 → vite dev 看似正常 → 用户窗口（甚至重启 tauri dev 后）仍是旧代码。
- **原因：** 两层。① **vite 的 chokidar watch 会漏编辑器的原子写入**（临时文件 + rename，Windows 上偶发）——模块图不失效，`?vue&type=style` 子请求一直返回**转换缓存**；② 用户"重启 dev"往往只重启了 app 窗口（webview 重新加载 URL），**vite server 进程没重启**，陈旧的转换结果继续被服务。验证手法：curl `localhost:5373/src/components/X.vue?vue&type=style...` 看服务内容是否含新改动；`touch` 文件强制失效后立刻出现（本次实测 min-width 案例）。
- **解法：** 改动后走查前，先 curl 对应模块的 vite 转换结果确认新代码已被服务；没有就 `touch` 文件或重启 vite server 进程（不是只重启 app 窗口）。
- **预防：** 同一问题的早期形态曾误判为"HMR websocket 断开"（回收站右键三轮误诊原因之一）——以后"用户看不到修复"先查 vite 服务的实际内容，再怀疑前端逻辑。
