# 117 WebView2 调试参数与共享数据目录

## 现象

真实 Tauri 验收需要 CDP 时，设置 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=…` 后端口始终不出现。显式构建器传入参数后，又出现 `0x8007139F`，WebView2 创建失败。应用与 MCP 服务本身可以运行，但无法用 CDP 验收真实界面。

## 原因

wry 会写入自己的默认浏览器参数，单设环境变量不等于窗口实际采用该参数。另一个隔离缺口是浏览器 profile：用户实例与验收实例共用 WebView2 用户数据目录，却要使用不同调试参数，浏览器环境无法兼容。仅隔离 `config.json` 和素材库并不足以覆盖这一层。

## 解法

验收路径用 `WebviewWindowBuilder.additional_browser_args` 显式传参，保留 wry 的默认禁用项与 sRGB；用 `data_directory` 指向 `ANIKIT_CONFIG_DIR/webview`，使浏览器 profile 也可重建。普通启动不开放 CDP，显式 `ANIKIT_E2E` 才允许并行验收。验收同时跳过单实例与 `anikit://` 注册，避免抢占用户窗口和协议路径；测试期间改写的协议路径已按用户实例恢复。

## 预防

真实实例验收必须同时隔离应用配置、素材库、浏览器 profile、MCP 端口文件和外部凭据。浏览器参数以正式构建器与启动日志为准，不能只看环境变量。启动测试前先核对现有实例，清理也只针对明确的测试 PID 与目录。
