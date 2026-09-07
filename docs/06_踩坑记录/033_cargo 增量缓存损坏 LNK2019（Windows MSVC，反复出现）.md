## 033 cargo 增量缓存损坏 LNK2019（Windows MSVC，反复出现）

- **现象：** 2026-08-13 起多次 `cargo test` / `build` 链接失败：`LNK2019 无法解析的外部符号`（serde_json / image 等匿名 anon.* 符号），符号与当次改动毫无关系。
- **原因：** target 增量编译缓存损坏（诱因未查明，疑似中断编译 / 高频小改叠加）；特征是报错的匿名符号属于第三方 crate 内部泛型展开。
- **解法：** `cargo clean -p app` 后重跑即过（只清本包，依赖缓存保留，重编译 ~1-2 分钟）。**更轻替代（2026-08-20 实证）**：`rm -rf target/debug/incremental` 仅清增量缓存，重编 ~45s 即过。
- **预防：** LNK2019 报匿名符号先 clean -p app / 清 incremental 再怀疑代码；别按符号名去查业务代码。诱因补一记：cargo test 与 tauri dev 的 cargo watcher **并发踩踏同一 target** 也会打出这种缓存错乱——跑全量测试前留意用户 dev 是否在跑。
