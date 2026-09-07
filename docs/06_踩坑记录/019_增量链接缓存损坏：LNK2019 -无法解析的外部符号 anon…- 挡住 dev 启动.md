## 019 增量链接缓存损坏：LNK2019 "无法解析的外部符号 anon…" 挡住 dev 启动

- **现象：** `cargo test` / `pnpm tauri dev` 编译末期链接失败：`error LNK2019: 无法解析的外部符号 anon.{hash}.llvm.{hash}`（serde Serialize/Deserialize 等泛型符号），报 N 个无法解析后 `fatal error LNK1120`、`could not compile 'app' (lib)`；代码本身无任何问题，且错误符号与本次改动无关。
- **原因：** Rust 增量编译的代码生成单元缓存（`target/debug/incremental/`）与前一次编译状态不一致（本机 2026-07-28 一天内出现 2 次；opt-level=1 的 dev profile + cdylib 组合下偶发）。
- **解法：** `rm -rf src-tauri/target/debug/incremental` 后重跑即可（只清增量缓存，依赖与 crate 产物保留，重编约 1 分钟）；不要 `cargo clean`（全量重编 10 分钟级）。
- **预防：** 遇到"符号与改动无关"的 LNK2019 先清 incremental 再怀疑代码；若日后频繁复发，再考虑 `[profile.dev] incremental = false`（代价是每次改代码重编 app crate 从秒级变分钟级，暂不关）。
