# 093. 仅改图标 tauri build 不重嵌 exe 图标——build.rs rerun-if-changed 不含 icons/（2026-09-26）

## 现象

图标源换新（`pnpm tauri icon` 重生成 `src-tauri/icons/` 全套）后 `pnpm tauri build --no-bundle` 重编，构建日志有「Compiling app」且耗时正常——但打包出的 exe 资源管理器图标**还是旧版**。

## 原因

exe 图标由 tauri-build 的 build.rs 在构建脚本阶段经 tauri-winres 编译进 `resource.lib` 再链接入 exe。cargo 是否重跑 build.rs 取决于其声明的 `cargo:rerun-if-changed` 清单——查 `target/release/build/app-*/output` 实证 13 条 = `tauri.conf.json` / capabilities / binaries 两个 sidecar / plugins 若干文件，**不含 `icons/` 目录**。仅改图标时 build.rs 被 cargo 判定无变化跳过，`resource.lib` 停在旧时间戳不 regenerate，链接出来的 exe 自然带旧图标。同日第一轮能嵌上新图标纯属巧合——那轮同时有源码改动逼 build.rs 整跑，图标顺带重嵌。

判别手法（不依赖资源管理器观感）：pefile 解 exe 的 RT_GROUP_ICON/RT_ICON 逐帧解码量 alpha 覆盖率，与 `icons/32x32.png` 期望值对账——本轮旧帧 78.1%×81.2% vs 新源应有 87.5%×93.8%，一量即穿。`ExtractAssociatedIcon`（PowerShell）提取目验也可，但小尺寸下肉眼难分，覆盖率数值最稳。

## 解法

**改完图标后 `touch src-tauri/tauri.conf.json` 再 `pnpm tauri build --no-bundle`**——conf 在 rerun-if-changed 清单里，mtime 一变 build.rs 必重跑，图标随之重嵌。重嵌判据：`target/release/build/app-*/out/resource.lib` 时间戳刷新 + 体积跟新 icon.ico 走。

## 预防

- **凡「只改 icons/ 后的打包」，流程钉死两步**：tauri icon 重生成 → touch tauri.conf.json → build；打包后冒烟加一道 exe 图标帧覆盖率对账（数值而非肉眼）。
- 同类泛化：tauri 侧「不进 rerun-if-changed 的构建输入」改动后都要给构建系统一个重跑理由（touch conf 是最轻的扳机），不能只看「Compiling」字样就认定新输入已生效——重编译 ≠ 构建脚本重跑。
