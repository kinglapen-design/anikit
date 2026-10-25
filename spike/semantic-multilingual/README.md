# Multilingual CLIP text + original OpenAI vision spike

验证中文文本塔 `sentence-transformers/clip-ViT-B-32-multilingual-v1` 与其训练时明确对齐的原始图像塔 `openai/clip-vit-base-patch32`。两塔输出同一 512 维空间；Rust `ort 2.0.0-rc.15` 在 Windows 上真实运行 CPU 和 DirectML，推理与质量评估均不需要 Python。没有修改 `src/`、`src-tauri/`、`docs/`、根配置或其它 spike，也没有提交 git。

## 许可与官方依据

详细通知和原文见 `notices/THIRD_PARTY_NOTICES.md`。

- 文本模型卡 https://huggingface.co/sentence-transformers/clip-ViT-B-32-multilingual-v1 和官方 API https://huggingface.co/api/models/sentence-transformers/clip-ViT-B-32-multilingual-v1 明示 `apache-2.0`。卡片明确写道：以原始 `clip-ViT-B-32` 为教师进行多语知识蒸馏，图像编码器保持不变；结构为 DistilBERT、mean pooling、768→512 Dense。
- OpenAI 官方 CLIP 仓库在固定提交 `d05afc436d78f1c48dc0dbf8e5980a9d471f35f6` 的根 LICENSE 明示 MIT：https://github.com/openai/CLIP/blob/d05afc436d78f1c48dc0dbf8e5980a9d471f35f6/LICENSE 。同一提交的官方加载器直接列出 ViT-B/32 权重地址：https://github.com/openai/CLIP/blob/d05afc436d78f1c48dc0dbf8e5980a9d471f35f6/clip/clip.py#L36 。本 spike 以此作为 OpenAI CLIP 代码及官方权重的 MIT 证据。
- HF 的 `openai/clip-vit-base-patch32` 是转换镜像，其页面/API **没有 license 标签**，所以不把镜像元数据冒充 MIT 证据。镜像固定为 `3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268`；再分发时应在合规清单中保留这一转换件元数据缺口。
- Apache-2.0、OpenAI MIT、双方模型卡、OpenAI README/model-card/loader，以及 ORT/DirectML 的许可与第三方通知均已保留。官方样图 `assets/CLIP.png` 来自同一 OpenAI MIT 仓库固定提交。

固定输入与软件版本：文本 revision `58edf8cada9e398793dca955574a48cbb7f18be2`，长度 128（`sentence_bert_config.json` 覆盖 tokenizer 的 512 默认值）；图像 1×3×224×224；FP32、opset 17、无动态轴。Python 3.12.14、Torch 2.6.0+cpu、Transformers 4.56.2、ONNX Runtime 1.24.4；Rust 1.98.1；DirectML 1.15.4。

## 模型导出与预处理

文本仓库确实自带 FP32 `onnx/model.onnx`，但实际检查表明它只有动态输入和 `last_hidden_state [batch,sequence,768]`，没有 mean pooling 和 Dense，不能直接得到对齐的 512 维文本向量。因此保留官方文件作审查依据，并有理由地重导出完整文本塔，而不是无意义重导出。图像侧只使用指定 OpenAI ViT-B/32 的 `vision_model + visual_projection`，未混入文本 CLIP 塔或其它图像模型。

`models/tokenizer.json` 由官方 fast tokenizer 加上固定 128 padding/truncation 生成。Python 官方 tokenizer 与 Rust `tokenizers` 逐条核对 token IDs 和 attention mask。图像预处理按官方 `preprocessor_config.json`：RGB、短边 224 的 bicubic resize、224 中心裁剪、1/255 rescale、OpenAI mean/std normalization。Rust 原生实现使用 `image` 的 CatmullRom bicubic；另用 Python 官方 processor 的共享 FP32 tensor 隔离验证模型数值。

导出兼容性只修了两轮：Transformers 对 `.bin` 的安全闸门要求 Torch≥2.6，故升级隔离环境；Transformers 4.56 的 `CLIPVisionTransformer.forward` 不接受 `return_dict`，故删除该关键字。ORT API 未需兼容性修复。

## 复现命令

在 Git Bash 中进入本目录：

```bash
bash scripts/setup_python.sh
source scripts/env.sh
"$PYTHON" scripts/download_model.py
"$PYTHON" scripts/export_onnx.py
"$PYTHON" scripts/prepare_reference.py
"$PYTHON" scripts/prepare_runtime.py
cargo build --release --locked --manifest-path rust/Cargo.toml
"$SPIKE_ROOT/rust/target/release/semantic-multilingual-spike.exe" --backend cpu
"$SPIKE_ROOT/rust/target/release/semantic-multilingual-spike.exe" --backend directml
```

`setup_python.sh` 若发现 `../semantic-clip/.venv`，只读复制为本目录的独立 `.venv` 后升级；否则新建 venv。所有 HF/pip/Cargo/Torch/临时缓存均在当前目录。`prepare_runtime.py` 优先只读复制已有私有 DLL，缺失时从 NuGet 获取并核对哈希。

质量评估使用统一数据集的 `quality-manifest.json + queries.json + 150 张图片`：

```bash
"$SPIKE_ROOT/rust/target/release/semantic-multilingual-spike.exe" --backend cpu --eval --language zh
"$SPIKE_ROOT/rust/target/release/semantic-multilingual-spike.exe" --backend directml --eval --language zh
```

默认读取 `../semantic-eval/manifest/`；可用 `--manifest PATH --queries PATH` 覆盖。输出分别为 `outputs/quality-{cpu,directml}.json`。TopK 是首个正例不超过 K 的常规查询比例，MRR 取首个正例倒数排名均值；`diagnostic_only` 查询不计入聚合。还支持 `--language en`，但本次未跑英文质量集。

## 最终实测

两种后端均为 `status=passed`、`backend_verified=true`。CPU profiling 只有 `CPUExecutionProvider`（文本/图像事件计数 5475/4849）；DirectML profiling 只有 `DmlExecutionProvider`（25/13），没有可见 CPU 回退。计数来自多次推理的 profiling 事件，不冒充静态图节点数或具体显卡型号。

共同数值结果：

- “机器人洗碗”固定为 128 IDs，非 padding 前缀 `[101,4482,2970,2179,4965,5866,102]`；Rust 与 Python 完全一致。
- 文本/图像输出均为 `[1,512]`、全部有限，归一化 L2 在 `0.99999976–1.00000024`。
- 中文文本相对 Python 的最大绝对差：CPU `0`，DirectML 原始 `9.54e-7`、归一化 `1.19e-7`。
- 对同一个 Python 图像 tensor：CPU 差异 `0`；DirectML 原始最大差 `4.29e-6`、归一化 `4.40e-7`。
- Rust 原生预处理与 Python tensor 的平均绝对差 `0.00043149`；由此产生的图像归一化向量最大差约 `0.001283`，不是逐位一致。官方样图三分类均以 `a diagram` 为最高项，Rust 概率 `0.99822360`。

性能为 2026-10-25 本机最终一次冷进程记录；文本 warmed-up 20 次、图像 10 次，nearest-rank P50/P95，单位 ms。warmed-up 计时包含 `session.run` 与输出提取，不含 tokenizer、tensor 克隆和图像预处理；小样本不是稳定 benchmark。

| 项目 | Rust CPU | Rust DirectML |
| --- | ---: | ---: |
| 环境初始化 | 42.12 | 39.41 |
| 文本模型冷加载 | 328.33 | 733.54 |
| 图像模型冷加载 | 272.96 | 347.69 |
| 程序内冷启动至首次中文文本完成 | 722.12 | 1185.02 |
| 首次中文文本推理 | 18.21 | 3.80 |
| 首次图像推理 | 25.91 | 4.11 |
| 文本 warmed P50 / P95 | 16.12 / 16.38 | 1.33 / 1.64 |
| 图像 warmed P50 / P95 | 17.49 / 17.94 | 1.94 / 2.30 |

质量评估已实际执行，不是预留空结果：50 条轨迹、150 张图、17 条常规中文查询、1 条诊断查询。CPU 与 DirectML 的下列数值完全一致；hard-negative 的边界余量仅有浮点尾差。

| Top1 hit | Top5 hit | Top10 hit | MRR |
| ---: | ---: | ---: | ---: |
| 0.352941 (6/17) | 0.529412 (9/17) | 0.647059 (11/17) | 0.423587 |

“机器人洗碗”在数据集中无直接正例，只保留诊断 Top10，未计入上述指标。质量结果只适用于这个已审查的小集合，不能外推为普遍中文检索能力。

## 文件与校验

保留的复现源文件：

- `scripts/`：`env.sh`、`setup_python.sh`、`requirements.txt`、`download_model.py`、`modeling.py`、`export_onnx.py`、`prepare_reference.py`、`prepare_runtime.py`。
- `rust/`：`Cargo.toml`、`Cargo.lock`、`src/main.rs`（验证、benchmark、原生预处理、质量评估）。
- `notices/` 与 `assets/CLIP.png`；本 README 和 `.gitignore`。

主要生成物（均被 `.gitignore` 排除）：

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| `models/text.onnx` | 540,591,040 | `d8cc5dcab1e6b37048b93e14dfaf90ca4a13d84704b8c161f06e4e5a97b07355` |
| `models/vision.onnx` | 351,517,281 | `91b93a912b0b8d6e355a9605bbce66415eb7366295819f2883dc66d9e266f5ca` |
| `models/tokenizer.json` | 2,919,627 | `672146ee6867dc02a01c474090e237789f8a066ee7247bb2cb6c8688a27536a8` |
| `runtime/onnxruntime.dll` | 17,328,152 | `e7eedec6a6f26dc39dc948276a75ef6d2bee3fff944d874ceed0bbd3b97bff40` |
| `runtime/onnxruntime_providers_shared.dll` | 22,040 | `265c8daf29637cb259cac8be9f08f2cd45f3883f0f0e4949cbfddd5b4cbec3b6` |
| `runtime/DirectML.dll` | 18,527,776 | `9c9e6d822561c6c41b90e6994b3e8857cf1d66dbfb1e0c4c799c7c89b4e92da1` |

全部下载权重/配置/通知的 URL、revision、字节数和哈希在 `outputs/download-manifest.json`；模型与 checkpoint 哈希在 `outputs/model-manifest.json`；运行时哈希在 `outputs/runtime-manifest.json`。数值参考为 `outputs/reference.json`，验证报告为 `rust-{cpu,directml}.json`，质量报告为 `quality-{cpu,directml}.json`，原始日志在 `logs/`。venv/cache/weights/models/runtime/outputs/logs/target 均已忽略。

## 未覆盖

未跑英文质量集、更多语言或公开检索 benchmark，未做 1000 帧耐久、batch/动态形状、显存统计、GPU 型号识别、线程 sweep 或正式性能 benchmark。原生 bicubic 与 PIL 不逐位一致，已量化而未隐瞒；产品若要求逐位预处理一致需另做实现。OpenAI HF 转换镜像缺 license 标签的事实需要在发布合规复核中保留。Python/Rust 第三方依赖的完整产品级 SBOM 与许可审计不在本 spike 范围内。
