# 091. 工具链升级后增量缓存混链——anon.*.llvm 符号 LNK2019（2026-09-26）

## 现象

`pnpm tauri dev` 链接 app_lib.dll 失败：`LNK2019 无法解析的外部符号 anon.<hash>.llvm.<数字>` × 24（gif 解码 / rmcp / serde / axum / preview_queue 等各模块内部匿名符号），`LNK1120` 收尾。同代码 `cargo test` / `clippy` 全绿——测试与检查走新产物，dev 链接复用旧 .o。

**当日复发**：`cargo clean -p app` + 全量重链验证绿，用户再跑 dev 又炸、膨胀到 231 个符号——清 deps 产物没动增量缓存，病根还在。

## 原因

rust 工具链 1.97 → 1.98 升级（clippy 报错提示 URL 已带 rust-1.98.0；09-02 待办记录的还是 1.97），`target/debug` 里混着旧编译器产出的目标文件：`anon.*.llvm.<hash>` 这类内部匿名符号的定义在各 codegen unit 间按编译器版本哈希互相引用，新旧对象文件混链即对不上号。典型的「环境漂移」非代码问题。

## 解法

~~`cargo clean -p app`~~ **不够**（首次定稿当日即复发）：-p 只清 deps 里的 app 产物，**病根在 `target/debug/incremental/`**——rustc 增量缓存按会话目录（app-*）留存，工具链换版后旧会话不自动作废，cargo test / clippy / cargo run 混跑时新旧会话对象交叉复用，anon 符号哈希即错位。

**定稿解法：`rm -rf target/debug/incremental` + 重编**（dev 链接即恢复；231 个符号一次清净）。

**三次复发与终局（2026-10-01）**：诱因 = rust 1.98.0 → **1.98.1 patch 升级**（patch 版本同样改变 anon 符号哈希，旧增量会话报废；用户 dev 链接 LNK2019×35）。清 incremental 救当次后，按下方预防节定案兑现——**`[profile.dev] incremental = false` 终局关闭**（patch 升级每隔几周来一次，清缓存只救当次）；全量重编 1m37s 链接绿。代价 = 每次改 Rust 全量重编本 crate；回退 = 删 Cargo.toml 该行。

## 预防

- **rust 工具链升级后第一件事 = 删 `target/debug/incremental`**（比 cargo clean -p app 便宜且对症）；版本漂移信号 = 报错 / clippy 提示 URL 里的版本号与上次记录不符（本项目 09-02 曾记 1.97 lint 漂移）。
- 同类「测试绿但 dev 链不上 / 反过来」先看产物新旧，不先怀疑代码。
- 增量缓存保命符：编译巨慢的机器可 `[profile.dev] incremental = false` 釜底抽薪（牺牲日常重编速度）；本项目 dev 循环依赖增量，暂不关——**同症状第三次复发时再关**（**2026-10-01 已触发并关闭**，见上节终局）。
