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
"$SPIKE_ROOT/rust/target/release/semantic-clip-spike.exe" --prepare-native
"$PYTHON" scripts/native_reference.py
"$SPIKE_ROOT/rust/target/release/semantic-clip-spike.exe" --backend cpu
"$SPIKE_ROOT/rust/target/release/semantic-clip-spike.exe" --backend directml
```

`scripts/env.sh` 显式设置本目录内的 `HF_HOME`、HF hub cache、`PIP_CACHE_DIR`、`UV_CACHE_DIR`、`CARGO_HOME`、`CARGO_TARGET_DIR`、`TORCH_HOME` 和临时目录，并禁用 Python 用户 site-packages。Python 使用 3.12.14，Torch 使用 2.5.1+cpu；Rust 使用 1.98.1 实测。

官方 `pytorch_to_onnx.py` 以 `ViT-B-16`、文本长度 52、opset 13 同时导出文本和视觉模型。当前 Torch 会将视觉侧 `MultiheadAttention` 追踪为 SDPA，因此 `scripts/official_export.py` 仅增加一层兼容入口：把 Torch 自带的 SDPA 数学展开符号函数注册到 opset 13，再原样调用官方脚本。本模型为非因果注意力，不使用 opset 14 的 `Trilu`。官方脚本也生成 FP16 文件，但本次 Rust 验证只使用重命名后的 FP32 `text.onnx` 和 `vision.onnx`。

`prepare_reference.py` 从与官方代码一致的 `vocab.txt` 生成 `tokenizer.json`，逐条确认其 52 个 token IDs 与官方 `clip.tokenize` 完全相同。Python ONNX Runtime 仅使用 `CPUExecutionProvider`，生成完整向量参考和 manifest。Rust 原生路径直接读取官方 `examples/pokemon.jpeg`，执行 JPEG 解码、直接 224×224 CatmullRom bicubic resize、RGB NCHW normalize，无 center crop。`--prepare-native` 写出的 FP32 文件只供 `native_reference.py` 生成独立的 Python-native 对照；Rust 模型输入始终来自自身 JPEG 预处理，不读取共享 FP32 文件。

## 原生预处理与 warmed smoke（2026-10-25）

两种后端均为 `status = passed`、`backend_verified = true`。CPU 路径只注册 CPU EP；DirectML 路径按高性能 GPU 的 DirectML→CPU 顺序注册，并启用注册失败即报错。程序解析 ORT profiling 的 `args.provider`，实际分别只有 `CPUExecutionProvider` 和 `DmlExecutionProvider`。

### 预处理与数值对照

原生张量与 Python/PIL 官方预处理张量共 150,528 个元素：

| 指标 | 结果 |
| --- | ---: |
| 逐位一致元素比例 | 89.08575% |
| 平均绝对差 | 0.0022076732 |
| RMS 差 | 0.0070585905 |
| 最大绝对差 | 0.0583937168 |

差异来自 JPEG 解码器，不能隐藏为“逐位一致”。因此报告同时保留两套对照：Python/PIL 跨解码器差异只作观察；严格 smoke 使用 Python 对 **Rust 原生张量** 独立跑出的 `native-python-reference.json`。Rust 与 Python-native 的图像输出差异为：CPU 原始向量最大差 `2.24e-6`、归一化向量最大差 `1.55e-7`；DirectML 分别为 `4.77e-6`、`1.49e-7`，均通过原容差。跨解码器图像归一化向量最大差约 `0.00152758`，最大 cosine 差约 `0.00051126`；皮卡丘仍为最高分类。

“机器人洗碗”及官方四候选的 token、形状、有限值、L2、相似度和 Python-native 对照均通过。原生张量 SHA256 为 `727925fbfbccb81515527c0928fb5f9b99564d869c378c8a0d4d15f8225007d4`，用于防止误用过期参考。

### 预热后重复计时

文本预热 3 次后测 20 次；图像预热 2 次后测 10 次。预处理与 ORT 推理分开计时，P50/P95 使用排序样本线性插值。

| 指标（ms） | Rust CPU P50 | Rust CPU P95 | DirectML P50 | DirectML P95 |
| --- | ---: | ---: | ---: | ---: |
| 原生 JPEG 预处理（10 次） | 0.5836 | 0.6123 | 0.4094 | 0.4262 |
| “机器人洗碗”文本推理（20 次） | 17.4510 | 22.4673 | 3.1843 | 3.4161 |
| 原生图像推理（10 次） | 45.2793 | 46.1930 | 3.6442 | 3.9173 |

峰值工作集分别为 CPU `1,141,727,232 B`、DirectML `986,963,968 B`，不含独立 GPU 显存统计。全部原始样本在报告中；这些仍是单台机器的 spike 数据，不是正式性能 benchmark。

## 产物与日志

- `outputs/model-manifest.json`：checkpoint、FP32 text/vision ONNX、tokenizer 的字节数与 SHA256，以及 opset、输入输出形状。
- `outputs/download-manifest.json`：固定 HF 文件及哈希。
- `outputs/runtime-manifest.json`：NuGet 包与私有 DLL 哈希。
- `outputs/reference.json`：Python token IDs、完整向量、相似度和参考耗时。
- `outputs/native_image_input.f32`、`outputs/native-python-reference.json`：仅供独立交叉验证的原生张量与 Python-native 参考，不作为 Rust 模型输入文件。
- `outputs/rust-cpu-native.json`、`outputs/rust-directml-native.json`：原生预处理误差、实际后端、数值断言、预热 P50/P95、RSS 和 profiling 路径；旧共享输入基线报告仍保留为 `outputs/rust-cpu.json`、`outputs/rust-directml.json`。
- `logs/`：环境、下载、导出、构建及两种后端的原始运行日志；`logs/prepare-native.log`、`logs/native-python-reference.log`、`logs/rust-cpu-native.log`、`logs/rust-directml-native.log` 对应本轮原生验证。

私有运行时文件共 35,877,968 字节：

| DLL | 字节数 |
| --- | ---: |
| `onnxruntime.dll` | 17,328,152 |
| `onnxruntime_providers_shared.dll` | 22,040 |
| `DirectML.dll` | 18,527,776 |

`.gitignore` 排除虚拟环境、所有缓存、第三方源码快照、weights、ONNX、tokenizer、runtime、target、日志和输出。保留脚本、`rust/Cargo.toml`、`rust/Cargo.lock` 与 Rust 源码即可按上节重新生成。

## 未覆盖与后续

50 轨迹/150 帧真实质量集及中文检索评估已在 `spike/semantic-eval/` 完成。仍未进行 1000 帧连续处理内存观察、RN50 对照或正式性能 benchmark。建议先解决权重许可，再依据质量集中的细粒度动作错误决定模型路线；预热计时与单例结果不外推到长期稳定性。
