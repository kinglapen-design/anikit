## 041 dev server 两坑连环：vite watcher 丢文件变更发旧版 SFC + HMR 整页刷新把 CDP 探针挂死

- **现象：** 2026-08-20 解包废除轮，CDP 探针两处诡异失败：① 预览探针断言 `.frame-strip` 不存在——组件明明已改（`v-if="show"`），curl vite 取 SFC 编译产物发现返回的是**上一版**（旧 `v-if="preview.seq"`）；② 卡片探针两次 600s 超时卡死——顶部检查已通过，滚轮段后 `Runtime.evaluate` 永远等不到响应。
- **原因：** ① 用户长跑的 `pnpm dev`（vite）的 chokidar watcher 在 Windows 上**偶发丢失文件变更事件**（本轮批量改了十余个前端文件），模块编译缓存不失效，新浏览器拿到的仍是旧编译产物——与踩坑 029"热更新假生效"同族但更隐蔽（连全新加载都是旧码）；② 大批文件变更后首次加载触发 vite **整页刷新**（HMR full-reload），探针 CDP 会话附着在旧文档上，`evaluate` 挂起无响应，探针 `send()` 死等 recv。
- **解法：** ① 探针前批量 `touch` 本轮改过的前端文件强制失效（touch 后 curl 验证 SFC 为新版再跑）；更稳的是探针前重启 dev。② 卡死探针 TaskStop + 连根杀 Edge（terminate 常杀不透，936x 端口 netstat 验证），touch 预热编译后重跑即过。
- **预防：** CDP 探针与用户长跑 dev server 共用 5373 时，**改完代码先 touch + curl 验证编译产物版本，再跑探针**；探针 send 宜加超时（现状死等）；凡探针拿到"逻辑上不可能"的断言失败，先怀疑拿到的是旧版页面而非代码 bug。
