# 114 库单测缺 Windows 资源：rfd 装载 `TaskDialogIndirect` 失败

## 现象

接入 `tauri::test::mock_app` 后，库单元测试在执行任何 Rust 语句前就以 `0xc0000139 (STATUS_ENTRYPOINT_NOT_FOUND)` 退出。临时定位日志一行也未打印，FFmpeg、模型和 mock 初始化都尚未运行，容易误判为模型或原生 FFmpeg 故障。

## 原因

原生调试记录实际加载模块与异常线程，最终在读出 `TaskDialogIndirect` 时才定位：测试程序激活的是 `comctl32` v5，而 rfd 需要 v6。Tauri 的默认 Windows manifest 虽声明 v6，`tauri-winres` 只通过 `rustc-link-arg-bins` 链接资源，库内单元测试没有该 manifest。

`cargo:rustc-link-arg-tests` 只接受独立测试目标，不能覆盖 lib 内单测；改用通用 `rustc-link-arg` 又会让正式 bins 重复嵌入 VERSION 资源，出现 `CVT1100 资源重复`。

## 解法

`build.rs` 只向 MSVC 输出 `cargo:rustc-link-search=native={OUT_DIR}`，正式 bins 继续走 Tauri 原资源链。`src/lib.rs` 仅在 `cfg(test)` 下用 `#[link(name = "resource", kind = "static")]` 引入同一份 manifest。库单测能加载 v6，正式 bins 不重复嵌资源。

修正后真实「分析 → 分段落库 → 中文检索」通过，随后完整 `cargo test` 也通过，证明不是以假数据绕过原生加载故障。

## 预防

Windows 下新增 `mock_app`、系统对话框或 rfd 测试时，先验证测试进程实际加载的 `comctl32` 版本和嵌入 manifest，不要把进程装载期错误当成业务错误。通用链接参数需同时检查 lib 单测、正式 bins 与其他 bin，避免只修一个目标又造成另一个目标资源重复。

诊断 native 装载故障时，不能只靠主进程日志；要按异常线程取证，Python 自带 manifest 可能激活不同版本的系统 DLL，静态导入检查通过也不能证明测试进程加载结果一致。
