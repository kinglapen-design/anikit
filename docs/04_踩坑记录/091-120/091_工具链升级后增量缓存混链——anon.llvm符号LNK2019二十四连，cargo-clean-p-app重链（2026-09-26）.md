# 091. 工具链升级后增量缓存混链——anon.*.llvm 符号 LNK2019 二十四连（2026-09-26）

## 现象

`pnpm tauri dev` 链接 app_lib.dll 失败：`LNK2019 无法解析的外部符号 anon.<hash>.llvm.<数字>` × 24（gif 解码 / rmcp / serde / axum / preview_queue 等各模块内部匿名符号），`LNK1120` 收尾。同代码 `cargo test` / `clippy` 全绿——测试与检查走新产物，dev 链接复用旧 .o。

## 原因

rust 工具链 1.97 → 1.98 升级（clippy 报错提示 URL 已带 rust-1.98.0；09-02 待办记录的还是 1.97），`target/debug` 里混着旧编译器产出的目标文件：`anon.*.llvm.<hash>` 这类内部匿名符号的定义在各 codegen unit 间按编译器版本哈希互相引用，新旧对象文件混链即对不上号。典型的「环境漂移」非代码问题。

## 解法

`cargo clean -p app`（清 6828 文件 / 15.5 GiB）后 `cargo build --no-default-features` 全量重链，1m49s 通过。不必 full clean（依赖 rlib 不含 app 的 anon 符号）。

## 预防

- **rust 工具链升级后第一件事 = `cargo clean -p app`**（或 full clean），再跑 dev / 打包；版本漂移信号 = 报错 / clippy 提示 URL 里的版本号与上次记录不符（本项目 09-02 曾记 1.97 lint 漂移）。
- 同类「测试绿但 dev 链不上 / 反过来」先看产物新旧，不先怀疑代码。
