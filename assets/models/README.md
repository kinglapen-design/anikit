# 语义模型离线包

本目录保存 AniKit 内置基础画面分析所需的受管模型资产。默认模型为 `multilingual-v1`：多语文本塔与其训练时对齐的 CLIP ViT-B/32 图像塔，共同输出 512 维向量。CPU 默认可离线运行，DirectML 由用户主动选择；不内置 LLM、Ollama、Python、PyTorch 或 CUDA 推理环境。

## 固定版本

- 文本塔：`sentence-transformers/clip-ViT-B-32-multilingual-v1`，revision `58edf8cada9e398793dca955574a48cbb7f18be2`，模型卡声明 Apache-2.0。
- 图像塔：`openai/clip-vit-base-patch32`，revision `3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268`；上游 OpenAI CLIP 的官方权重与代码采用 MIT 路径。
- 推理文件：FP32 ONNX，文本塔、图像塔及 tokenizer 必要文件合计 895,027,948 bytes（约 853.6 MiB）。

`multilingual-v1/model-manifest.json` 记录固定 revision、权重 SHA-256 和运行所需文件。`notices/` 保留许可全文、模型卡与来源证据；`runtime/licenses/` 和 `outputs/runtime-manifest.json` 保留私有 ORT / DirectML 运行时的许可与版本信息。公开下载入口尚未定稿，不能将预研目录当成正式发布地址。

## 生成与校验

`python tools/prepare-semantic-model.py` 从已经验证的本地转换结果整理文件，逐个核对 revision、大小和 SHA-256，再复制到本目录；不下载、不移动或修改源文件。只收运行时需要的模型与许可，不携带 spike 原始权重、评估集、缓存或其他实验产物。模型二进制和示例图由 `.gitignore` 排除，可用同一脚本重建。

`tools/pack-portable.py` 在组装前再次校验模型，并将本目录复制到便携包的 `models/multilingual-v1/{固定 revision}/`。应用优先使用用户明确指定的模型目录；未指定时读取包内模型，再回退到受管数据目录。离线模型与 `vectors/semantic.sqlite3` 派生索引分开管理，索引可以删除重建，模型不是用户素材的备份对象。
