# Chinese-CLIP + Rust ORT Windows spike

验证 `OFA-Sys/chinese-clip-vit-base-patch16` 的中文文本/图像 512 维向量、Rust `ort 2.0.0-rc.15`、CPU 与 DirectML，以及不依赖系统 ORT 的 Windows x64 私有运行时。所有依赖、缓存、权重、模型、虚拟环境、构建目录和输出均限制在本目录。

## 许可闸门

截至 2026-10-24，固定的 HF 模型提交及模型卡均未声明 `license` 字段，**权重许可仍未确认**。官方代码仓库的许可证不能替代权重许可。权重、导出的 ONNX、tokenizer 生成物和官方示例图只用于内部 spike，禁止提交、再分发或直接进入产品；产品化前必须先取得明确许可结论。NuGet 运行时的许可证和第三方声明由获取脚本放在 `runtime/licenses/`。

固定输入：

- 模型：`OFA-Sys/chinese-clip-vit-base-patch16`，HF 提交 `36e679e65c2a2fead755ae21162091293ad37834`。
- 官方导出源码：`OFA-Sys/Chinese-CLIP`，提交 `31863c707501bf1605d36842f43deb78793dbc5d`。
- 运行时：`Microsoft.ML.OnnxRuntime.DirectML 1.24.4` 与 `Microsoft.AI.DirectML 1.15.4`。
- Rust：`ort = 2.0.0-rc.15`，`default-features = false`，仅启用 `std`、`ndarray`、`api-24`、`directml`、`load-dynamic`。

## 复现

在 Git Bash 中进入本目录，依次执行：

```bash
bash scripts/fetch_source.sh
bash scripts/setup_python.sh
source scripts/env.sh
"$PYTHON" scripts/download_model.py
"$PYTHON" scripts/fetch_runtime.py
bash scripts/export_onnx.sh
"$PYTHON" scripts/prepare_reference.py
cargo build --release --locked --manifest-path rust/Cargo.toml
"$SPIKE_ROOT/rust/target/release/semantic-clip-spike.exe" --backend cpu
"$SPIKE_ROOT/rust/target/release/semantic-clip-spike.exe" --backend directml
```

`scripts/env.sh` 显式设置本目录内的 `HF_HOME`、HF hub cache、`PIP_CACHE_DIR`、`UV_CACHE_DIR`、`CARGO_HOME`、`CARGO_TARGET_DIR`、`TORCH_HOME` 和临时目录，并禁用 Python 用户 site-packages。Python 使用 3.12.14，Torch 使用 2.5.1+cpu；Rust 使用 1.98.1 实测。

官方 `pytorch_to_onnx.py` 以 `ViT-B-16`、文本长度 52、opset 13 同时导出文本和视觉模型。当前 Torch 会将视觉侧 `MultiheadAttention` 追踪为 SDPA，因此 `scripts/official_export.py` 仅增加一层兼容入口：把 Torch 自带的 SDPA 数学展开符号函数注册到 opset 13，再原样调用官方脚本。本模型为非因果注意力，不使用 opset 14 的 `Trilu`。官方脚本也生成 FP16 文件，但本次 Rust 验证只使用重命名后的 FP32 `text.onnx` 和 `vision.onnx`。

`prepare_reference.py` 从与官方代码一致的 `vocab.txt` 生成 `tokenizer.json`，逐条确认其 52 个 token IDs 与官方 `clip.tokenize` 完全相同。Python ONNX Runtime 仅使用 `CPUExecutionProvider`，生成完整向量参考和 manifest。图像使用官方 `examples/pokemon.jpeg` 与官方 `image_transform`；Python 和 Rust 共享同一个预处理后的 little-endian FP32 NCHW 文件，避免把图像缩放差异混入 ORT 对照。**本 spike 尚未实现 Rust 原生 JPEG/ bicubic 预处理。**

## 2026-10-24 实测结果

两种后端均为 `status = passed`、`backend_verified = true`。CPU 路径只注册 CPU EP；DirectML 路径按高性能 GPU 的 DirectML→CPU 顺序注册，并启用注册失败即报错。程序还解析 ORT profiling 的 `args.provider`，而不是以注册成功冒充实际 GPU 执行。

| 项目 | Rust CPU | Rust DirectML |
| --- | ---: | ---: |
| 实际 provider（profiling） | 仅 `CPUExecutionProvider` | 仅 `DmlExecutionProvider` |
| 文本/图像 provider 计数 | 1800 / 455 | 5 / 1 |
| 环境初始化 | 42.27 ms | 40.44 ms |
| 文本模型冷加载 | 304.02 ms | 1088.72 ms |
| 图像模型冷加载 | 448.41 ms | 905.55 ms |
| 首次“机器人洗碗”推理 | 18.52 ms | 4.92 ms |
| 图像推理 | 45.36 ms | 5.69 ms |
| 峰值工作集 | 1,142,136,832 B | 1,013,096,448 B |
| 结束前两模型仍驻留的工作集 | 834,158,592 B | 380,862,464 B |

这些是单次冷启动 spike 数据，不是稳定性能基准；工作集来自 Windows `GetProcessMemoryInfo`，不含独立 GPU 显存统计。DirectML 的 `actual_backend` 证明的是 EP 节点放置，不额外冒充已识别具体显卡型号。

共同数值结果：

- “机器人洗碗”token IDs 前缀为 `[101, 3322, 1690, 782, 3819, 4813, 102]`，其余补零；输出形状为 `[1,512]`，全部有限，归一化 L2 约为 1。
- CPU 的“机器人洗碗”归一化向量与 Python 最大绝对差为 `1.34e-7`，DirectML 为 `3.58e-7`。
- 图像与“机器人洗碗”的 cosine：Python `0.28803691`，CPU `0.28803709`，DirectML `0.28803694`。
- 官方四候选分类均以“皮卡丘”为最高项；Python 概率 `0.94367826`，CPU `0.94367701`，DirectML `0.94367665`。
- Python 参考使用 ORT 1.20.1，Rust 私有运行时为 ORT 1.24.4；报告保留了逐向量差异和容差，未将不同版本视为必须逐位一致。

## 产物与日志

- `outputs/model-manifest.json`：checkpoint、FP32 text/vision ONNX、tokenizer 的字节数与 SHA256，以及 opset、输入输出形状。
- `outputs/download-manifest.json`：固定 HF 文件及哈希。
- `outputs/runtime-manifest.json`：NuGet 包与私有 DLL 哈希。
- `outputs/reference.json`：Python token IDs、完整向量、相似度和参考耗时。
- `outputs/rust-cpu.json`、`outputs/rust-directml.json`：实际后端、数值断言、耗时、RSS 和 profiling 路径。
- `logs/`：环境、下载、导出、构建及两种后端的原始运行日志；`logs/python-reference-utf8.log` 是原 Python 日志的 UTF-8 转码副本。

私有运行时文件共 35,877,968 字节：

| DLL | 字节数 |
| --- | ---: |
| `onnxruntime.dll` | 17,328,152 |
| `onnxruntime_providers_shared.dll` | 22,040 |
| `DirectML.dll` | 18,527,776 |

`.gitignore` 排除虚拟环境、所有缓存、第三方源码快照、weights、ONNX、tokenizer、runtime、target、日志和输出。保留脚本、`rust/Cargo.toml`、`rust/Cargo.lock` 与 Rust 源码即可按上节重新生成。

## 未覆盖与后续

未进行 40–60 片段质量库、1000 帧耐久、RN50 对照或正式性能 benchmark，也未验证 Rust 原生图像预处理。建议先解决权重许可，再补原生预处理一致性、 warmed-up 重复计时和显存指标；质量库与耐久验证应使用独立的真实数据集，不由本次单例结果外推。
