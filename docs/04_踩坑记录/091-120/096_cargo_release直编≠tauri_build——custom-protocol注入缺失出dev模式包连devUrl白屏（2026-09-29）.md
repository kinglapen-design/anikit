# 096 cargo build --release ≠ tauri build——custom-protocol feature 决定 dev/prod，直编出 dev 模式包连 devUrl 白屏（2026-09-29）

- **现象：** 免安装包重打后用户实机打开 = 整窗 Edge 内核错误页「无法访问此页面 / 127.0.0.1 拒绝连接」（WebView2 导航 devUrl `127.0.0.1:5373`，vite 未在跑）。冒烟却「过」了——只验了进程起 + MCP 握手（后端 WS 与前端无关），**没看窗口 UI**。
- **根因：** tauri 2 的 dev/prod 判断**不看 Cargo PROFILE**——`tauri/build.rs`：`dev = !has_feature("custom-protocol")`，经 `cargo:dev` links 指令传给 tauri-build 的 `is_dev()`。项目 Cargo.toml 的 tauri features 只有 `["protocol-asset", "tray-icon"]`，`custom-protocol` 由 **tauri CLI 在 `tauri build` 时注入**（dev 不注入）。直接 `cargo build --release` = 无该 feature = **dev 模式 exe**：不嵌 frontendDist、运行时导航 devUrl。09-24/09-26 历轮重打均走 `pnpm tauri build --no-bundle` 故从未触发；本轮图省事直跑 cargo 中招。
- **解法：** 打包只用 `pnpm tauri build --no-bundle`（自动注入 custom-protocol + 跑 beforeBuildCommand）。重打后换正 portable 并**截图验证真 UI** 才算数。
- **预防：** ①release 产物验证必须包含「窗口真 UI 截图」——进程活 + 端口通只能证明后端活着，前端加载失败完全测不出；②「同样的编译命令换个入口」类捷径（cargo 直跑 vs CLI）在 tauri 体系不等价，feature 注入是隐式差异；③二进制粗验：prod exe 内嵌 dist（体积含压缩资产），dev exe 跑起来导航 devUrl——实机起窗一眼可辨。
