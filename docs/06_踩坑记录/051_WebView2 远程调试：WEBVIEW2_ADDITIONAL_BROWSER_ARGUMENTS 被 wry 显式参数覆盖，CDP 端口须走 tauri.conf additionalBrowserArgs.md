## 051 WebView2 远程调试：WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS 被 wry 显式参数覆盖，CDP 端口须走 tauri.conf additionalBrowserArgs

- **现象：** 2026-08-22 真机 spike 前置：tauri dev / 直跑 app.exe 带 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223`（bash 前缀 / cmd set 两法皆试），9223 恒不 LISTEN，msedgewebview2 browser 进程命令行无该 flag。
- **原因：** wry 创建 WebView2 Environment 时**显式传了** AdditionalBrowserArguments（browser 命令行可见 `--autoplay-policy=no-user-gesture-required --disable-features=msWebOOUI,...` 等 wry 默认）——loader 以 API 显式值为准，同名环境变量被忽略。
- **解法：** tauri.conf.json 窗口配置加 `"additionalBrowserArgs": "--remote-debugging-port=9223"`（改后 cargo build 重嵌 conf 生效），`http://localhost:9223/json` 出 page target 即通；spike 完删除该行（勿把调试端口带入口常配置）。另：9222 在本机被 Adobe Animate（CEP 调试）占用——端口被占先 netstat 排查换号。
- **预防：** WebView2 行为调整（CDP / Chromium 旗标）先查 wry 是否已显式传参；环境变量失效时看 browser 进程命令行即可定位参数来源。
