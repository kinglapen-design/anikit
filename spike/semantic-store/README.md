# Semantic Store Spike

验证 1.0 语义分析的模型无关数据层，不接入主业务。

## 覆盖

- `semantic_models`：模型 id / revision / 维度 / 权重 SHA-256。
- `semantic_assets`：源指纹、媒体类型、当前模型与分析版本。
- `semantic_segments`：起止毫秒、可选起止帧索引、类型、文本与归一化向量。
- `analysis_tasks` / `analysis_task_items`：库 root、assets_dir、设备、状态和逐项检查点。

同一资产重跑分析会在事务内替换旧段，不混用模型 revision。向量查询使用归一化点积并做确定性 topK；后续主业务还需在查询前接入 Placement 范围、hidden / 回收站和陈旧索引过滤。

## 验证

```bash
cargo test --locked --manifest-path Cargo.toml
cargo clippy --locked --manifest-path Cargo.toml -- -D warnings
```

2026-10-24 实测：3/3 测试通过，clippy 零告警。

- 重复替换不留下重复段，查询返回正确时间与帧索引；维度不匹配拒绝。
- 零范数向量和倒置时间区间拒绝。
- 任务 ID 去重、逐项完成、暂停 / 恢复、重开后中断恢复与最终完成均通过。
