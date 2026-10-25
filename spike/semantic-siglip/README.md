# SigLIP 2 base patch16-512 选型 spike

在与 multilingual-v1 完全相同的 150 张图片、17 条常规中文查询和 1 条零正例诊断查询上，评估 `google/siglip2-base-patch16-512` 的质量与成本。结论：**质量下降且成本更高，不建议作为 AniKit 1.0 基础模型；继续保留 multilingual-v1。** 本目录独立，未更新 docs/主线，未提交 git。

## 官方依据与许可

- 模型卡 https://huggingface.co/google/siglip2-base-patch16-512 和 API https://huggingface.co/api/models/google/siglip2-base-patch16-512 均确认 **Apache-2.0**，revision 固定为 `a89f5c5093f902bf39d3cd4d81d2c09867f0724b`。
- 仓库包含 375,823,874 个 FP32 参数，没有官方 ONNX。模型卡、Apache 文本、OpenAI 样图 MIT、ORT/DirectML notices 均保存在 `notices/` 或 `runtime/licenses/`。
- 固定配置来自官方文件：`model_type: siglip`，所以 Transformers 4.56.2 实际加载 `SiglipModel`；没有擅自改用 `Siglip2Model` 或其它 checkpoint。
- 文本长度 64，最终向量是 final LayerNorm 后的最后 token 再经 Linear(768,768)，不是 `last_hidden_state`。图像为 512×512、patch16、attention pooling head。两塔最终均为 **[1,768]**，不是 CLIP 的 512 维。
- 官方预处理是直接双线性 resize 到 512×512（`resample: 2`）、1/255 rescale、mean/std 0.5；没有中心裁剪。Gemma fast tokenizer 不加 BOS、加 EOS、pad ID 0，固定 64；官方调用只返回 `input_ids`，所以文本 ONNX 不虚构 `attention_mask` 输入。

详细证据 URL 与通知见 `notices/THIRD_PARTY_NOTICES.md`。

## 复现

在 Git Bash 中进入本目录：

```bash
bash scripts/setup_python.sh
source scripts/env.sh
"$PYTHON" scripts/download_model.py
"$PYTHON" scripts/export_onnx.py
"$PYTHON" scripts/prepare_reference.py
"$PYTHON" scripts/prepare_runtime.py
cargo build --release --locked --manifest-path rust/Cargo.toml

"$SPIKE_ROOT/rust/target/release/semantic-siglip-spike.exe" --backend cpu
"$SPIKE_ROOT/rust/target/release/semantic-siglip-spike.exe" --backend directml
"$SPIKE_ROOT/rust/target/release/semantic-siglip-spike.exe" --backend cpu --eval --language zh
"$SPIKE_ROOT/rust/target/release/semantic-siglip-spike.exe" --backend directml --eval --language zh
"$PYTHON" scripts/compare_quality.py
```

Python 直接只读复用 `../semantic-multilingual/.venv`，`PYTHONDONTWRITEBYTECODE=1` 防止回写。HF/pip/Cargo/Torch/临时缓存、模型和输出均在本目录。私有 ORT 1.24.4/DirectML 1.15.4 从 `../semantic-clip/runtime` 只读复制并核验哈希。Rust 推理和质量评估不需要 Python；`compare_quality.py` 只做报告汇总。

`--manifest` 和 `--queries` 可覆盖默认的 `../semantic-eval/manifest/` 文件。质量计算只统计非诊断查询；“机器人洗碗”保留诊断 Top10 和典型错误，**不计入 Top1/5/10/MRR**。

## 导出与 smoke 验证

文本/图像 FP32 ONNX 都包含最终 pooling/projection，固定输入、opset 17、无动态轴。参考脚本同时断言导出的 wrapper 与官方 `get_text_features` / `get_image_features` 一致。第一次参考校验发现官方 tokenizer 不返回 `attention_mask`，随后一轮修正为官方单输入路径；DirectML 无需修复，没有不支持节点。

两种后端均 `status=passed`、`backend_verified=true`：

- 中文“机器人洗碗”固定 64 IDs，前缀 `[120110,236720,238756,1]`，其余 pad 0；Rust 与 Python 完全一致。
- 文本/图像输出 `[1,768]`、全部有限、归一化 L2 约 1。
- CPU 对 Python 参考数值差为 0。DirectML 中文文本原始/归一化最大差 `2.87e-6 / 1.38e-7`；同一 Python 图像 tensor 为 `9.42e-6 / 1.02e-6`。
- Rust 原生双线性预处理与 Python tensor 平均绝对差 `7.05e-5`；图像归一化向量最大差约 `0.000600`。官方 CLIP.png 的候选最高分均为 `a diagram`。
- CPU profiling 只有 CPU EP。DirectML smoke 的文本/图像 profiling 只有 DML EP（25/13 个事件），质量运行也只有 DML（18/150）；没有 CPU 回退或伪报。

### 冷加载、重复时延与 RSS

单次冷进程加 warmed-up 文本 20 次、图像 10 次，nearest-rank P50/P95；单位为 ms。小样本用于成本对比，不冒充稳定 benchmark。

| 项目 | Rust CPU | Rust DirectML |
| --- | ---: | ---: |
| 环境初始化 | 42.17 | 39.91 |
| 文本模型冷加载 | 705.22 | 1404.26 |
| 图像模型冷加载 | 314.48 | 584.03 |
| 冷启动至首次中文文本完成 | 1843.05 | 2788.96 |
| 首次中文文本推理 | 22.43 | 4.90 |
| 首次图像推理 | 307.58 | 17.90 |
| 文本 warmed P50 / P95 | 20.24 / 21.10 | 3.17 / 3.44 |
| 图像 warmed P50 / P95 | 476.29 / 484.26 | 14.10 / 15.40 |
| 峰值工作集 | 2,015,432,704 B | 2,092,122,112 B |

## 统一质量结果

CPU 与 DirectML 的 Top1/5/10/MRR 完全一致；50 条轨迹、150 张图、17 条常规查询、1 条诊断查询均实际运行。

| 指标 | multilingual-v1 | SigLIP 2 | 绝对差 | 结论 |
| --- | ---: | ---: | ---: | --- |
| Top1 | 0.352941 | 0.235294 | -0.117647（少 2/17） | 下降 |
| Top5 | 0.529412 | 0.294118 | -0.235294（少 4/17） | 下降 |
| Top10 | 0.647059 | 0.588235 | -0.058824（少 1/17） | 下降 |
| MRR | 0.423587 | 0.300860 | -0.122727（相对 -28.97%） | 下降 |

这不是“显著提高”：在当前任务分布上四项全部下降。17 条查询中，5 条首个正例排名改善、9 条退化、3 条不变；未做统计显著性检验。对照 Chinese-CLIP 的 Top1/Top5 也仅为持平，Top10 高 1/17，但 MRR 低 0.016028，且 Chinese-CLIP 的权重许可仍未确认。

典型退化包括 `open_microwave` 排名 26（multilingual 为 1）、`close_microwave` 42（10）、`put_spoon_in_sink` 55（37）、`put_mushroom_in_pot` 46（26）。改善例包括 `open_drawer` 1（28）、`wipe_burner` 30（69）、`wipe_pot_with_sponge` 9（38），但不足以抵消整体退化。

Hard-negative 的 `positive_outscore_rate`：subject-match/action-mismatch 为 0.500（multilingual 0.583），action-match/subject-mismatch 为 0.500（持平）。完整 margin、best negative、逐查询 Top10、正例排名和 25 条典型错误已输出，不只是四个聚合数字。

## 成本对比

部署体积按两个 ONNX 加 tokenizer 计算，不把 runtime 重复算入模型：

| 项目 | SigLIP 2 | multilingual-v1 | 倍率 |
| --- | ---: | ---: | ---: |
| 文本 ONNX | 1,129,352,212 B | 540,591,040 B | 2.089× |
| 图像 ONNX | 374,244,754 B | 351,517,281 B | 1.065× |
| tokenizer | 34,363,136 B | 2,919,627 B | 11.770× |
| 两塔 ONNX 合计 | 1,503,596,966 B | 892,108,321 B | 1.685× |
| 部署合计 | **1,537,960,102 B** | **895,027,948 B** | **1.718×（+642,932,154 B）** |
| 单条向量维度 | 768 | 512 | 1.5× |

相对当前 multilingual-v1 复跑数据：CPU 文本 P50 为 1.25×、CPU 图像 P50 为 **27.47×**；DirectML 文本 P50 为 2.25×、图像 P50 为 **6.92×**。冷启动至首次文本分别为 2.52×/2.27×，峰值 RSS 分别为 1.94×/2.05×。质量损失的同时还增加模型、索引向量、内存和推理成本。

## 选型结论与未覆盖

**不建议作为 1.0 基础模型。** Apache-2.0 许可清晰、CPU/DirectML 可用是优点，但统一任务上的质量没有提升，且图像成本增加约一个数量级（CPU）或数倍（DirectML）。当前没有用它替换 multilingual-v1 的合理收益；保留它只作为后续更大数据集复核的备选。

未覆盖：英文和其它语言质量、公开检索 benchmark、统计显著性、批量/动态分辨率、FP16/量化、1000 帧耐久、显存与 GPU 型号、生产索引端到端内存。原生双线性 resize 与 PIL 不逐位一致，已量化。量化可能降低成本，但其质量和 DirectML 支持未验证，不能用来推翻本次结论。

## 文件

- 复现脚本：`scripts/{env.sh,setup_python.sh,requirements.txt,download_model.py,modeling.py,export_onnx.py,prepare_reference.py,prepare_runtime.py,compare_quality.py}`。
- Rust：`rust/Cargo.toml`、`rust/Cargo.lock`、`rust/src/main.rs`。
- 清单：`outputs/{download-manifest,model-manifest,runtime-manifest}.json`。
- 验证：`outputs/{rust-cpu,rust-directml}.json`；质量：`quality-{cpu,directml}.json`。
- 对比：`outputs/comparison.json`、`outputs/typical-errors.json`；原始日志在 `logs/`。
- `.gitignore` 排除 venv/cache/weights/models/runtime/outputs/logs/target 和其它大文件；源码、锁文件、README、notices 保留。
