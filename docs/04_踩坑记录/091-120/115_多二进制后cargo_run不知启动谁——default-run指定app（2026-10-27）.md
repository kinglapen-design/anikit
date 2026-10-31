# 115 多二进制后 cargo run 不知启动谁

## 现象

新增独立 `semantic-worker` 后，标准 `pnpm tauri dev` 在真正启动前端与后端前就退出。Cargo 报告候选二进制有 `anikit-paste`、`app`、`semantic-worker`，要求显式指定 `--bin`。

## 原因

项目原来只有一个应用二进制，未配置 `default-run`。增加 worker 后，`cargo run` 不再能推断默认目标。语义测试此前只编译指定 bin，掩盖了标准开发入口的缺口。

## 解法

在 `src-tauri/Cargo.toml` 的 package 段增加 `default-run = "app"`。worker 继续作为独立进程由应用按需拉起，不改变其职责，也不用手工绕过标准开发命令。

## 预防

新增任何 bin 时同时检查 `cargo run` 和 `pnpm tauri dev`，不要只验证 `cargo build --bin …`。默认应用和辅助进程的边界必须在 manifest 中明确，不能让命令行为随候选数量变化。
