# 2026-10-24 Chinese-CLIP CPU / DirectML spike 验证

## 结论

在 `spike/semantic-clip` 完成真实 Chinese-CLIP + Rust ORT 双后端验证，主代理随后独立复跑，结果一致。**CPU 与 DirectML 都能运行中文文本 / 图像向量，DirectML 本机未发生 CPU 回退。**

本轮证明的是模型导出、Rust 推理和私有运行时路线可行，不代表“机器人洗碗”语义检索已达标，也不代表权重可以发布。

## 实测

模型为 `OFA-Sys/chinese-clip-vit-base-patch16`，固定 HF 提交 `36e679e65c2a2fead755ae21162091293ad37834`，官方导出源码提交 `31863c707501bf1605d36842f43deb78793dbc5d`。FP32 / opset 13 双塔：文本 `text.onnx` 413,318,654 B，图像 `vision.onnx` 354,538,989 B，均输出 `[1,512]`。

运行时采用 `ort 2.0.0-rc.15`（`api-24` / `directml` / `load-dynamic`）+ ORT DirectML 1.24.4 + DirectML 1.15.4，私有 DLL 共 35,877,968 B，经 `SetDllDirectoryW` 与 `ort::init_from` 显式加载，不依赖系统 ORT。

| 项目 | Rust CPU | Rust DirectML |
| --- | ---: | ---: |
| profiling 实际后端 | 仅 `CPUExecutionProvider` | 仅 `DmlExecutionProvider` |
| 文本模型冷加载 | 304.02ms | 1088.72ms |
| 图像模型冷加载 | 448.41ms | 905.55ms |
| 首次“机器人洗碗”推理 | 18.52ms | 4.92ms |
| 图像推理 | 45.36ms | 5.69ms |
| 峰值工作集 | 约 1.14GB | 约 1.01GB |

独立复跑同样通过：DirectML 文本约 2.9–4.8ms、图像约 5.5ms；`cargo check --release --locked` 通过。上述均为单次冷启动 spike 与一次复跑，不作为稳定 benchmark。

中文“机器人洗碗”token IDs 与官方 `clip.tokenize` 完全一致；两塔输出有限且归一化 L2 约 1。该文本归一化向量与 Python 参考的最大绝对差为 CPU `1.34e-7`、DirectML `3.58e-7`。官方宝可梦样例四候选均以皮卡丘为最高项，CPU / DirectML 概率与 Python 一致。

## 过程问题

官方导出补装并固定 `onnxconverter-common==1.14.0`；Torch 2.5.1 的 SDPA 不支持 opset 13，通过兼容入口注册 Torch 自带数学展开符号函数后导出，未修改模型参数或提高 opset。Git Bash 的 Windows 路径打包使用 `tar --force-local`。`ort` 注册失败不隐藏，profiling 无 DML 节点也判失败，避免把静默回退写成 GPU 成功。

复现脚本、Rust 源码、Cargo.lock 与 README 已提交（`484816eb`）；权重、ONNX、tokenizer、运行时、缓存、日志和输出均由 `.gitignore` 排除。

## 未关闭的闸门

**权重许可仍未确认。** HF 模型提交、API 与模型卡未声明 license，代码许可证不能代替权重许可证；当前权重和导出物只用于内部 spike，不进入产品或发布包。

尚未完成 40–60 动作片段质量库、1000 帧耐久、RN50 对照和 Rust 原生 JPEG / bicubic 预处理；本轮 Rust 与 Python 共享官方预处理后的 FP32 图像张量。后续以 `spike/semantic-eval` 的真实动作样本与 hard negatives 验证质量，并以 `spike/semantic-multilingual` 的许可清晰候选做备选。路线图第 2 项保持未勾。
