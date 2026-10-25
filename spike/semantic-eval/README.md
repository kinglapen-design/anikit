# BridgeData V2 中文图文检索质量 spike

本目录构建一个可重复的小型真实质量集，并用 `spike/semantic-clip` 已导出的 Chinese-CLIP FP32 ONNX 做同一批图像的检索评估。未下载 411 GB 整包，未做 1000 帧耐久或 RN50 对照。

## 数据来源与许可

- 数据：BridgeData V2，官方页面 <https://rail-berkeley.github.io/bridgedata/>。
- CSV：<https://rail-berkeley.github.io/bridgedata/traj_links.csv>，SHA256 `a9c971ba2dbd3b5f55fa091bf1873f26d1ded6fd06d3808f9497f68bb6729567`。
- 帧 prefix：`https://rail.eecs.berkeley.edu/datasets/bridge_release/`。
- 数据许可：**CC BY 4.0**，署名 BridgeData V2、Homer Walke et al.、RAIL Berkeley。
- Chinese-CLIP 权重许可仍是独立闸门；本数据集的 CC BY 许可不构成模型权重许可。

CSV 实际无表头：前两列是首/末 JPEG 相对路径，后续列是原始自然语言标注。固定选择 50 条 CSV 轨迹，每条下载 `images0` 的首帧、`floor(last/2)` 中帧和末帧，共 **150 张、13,857,614 字节**。选择覆盖水槽、餐具、锅、海绵/刷子、厨房设备、擦拭、布料/洗衣机、扫颗粒及其他取放动作；固定 CSV 行号，不使用随机抽样。

## 标注核验与“洗碗”边界

`manifest/selection.json` 保存选择规则和来源；`manifest/review.json` 保存逐轨迹看图结论；`manifest/quality-manifest.json` 是最终可复用 manifest，逐帧保留原始 CSV 行、原始 annotation、轨迹 URL prefix、帧 URL、帧索引、尺寸、字节数、SHA256、来源和许可。

全部 150 张图均通过 PIL 真实 JPEG 解码验证，并通过 9 张首/中/末联系表逐条查看；洗衣机样本另做了原尺寸复核。语义标签不是仅凭文件名生成。动作查询通常只用中/末帧作为正例，首帧不因属于同一轨迹就自动算动作已完成。

**本质量集没有直接“机器人洗碗”正例。** 全 CSV 的 wash/rinse/clean 关键词核查也没有找到可支持的明确洗碗标注，但这不是对全部 60k 轨迹的穷尽语义审阅。`wipe pot with sponge` 只算“用海绵擦锅”；干水槽中的放锅/放餐具只算取放，均不冒充洗碗。“机器人洗碗”仅作为零正例诊断查询，不进入 Top-k、MRR 或 hard-negative 聚合。

T035/T036 的原标注写着把布放进洗衣机，但图片只能确认布位于圆形门表面，不能证明进入内筒。因此查询写成“把布放到洗衣机门上”，并在 review 中把标注支持级别记为 `partial`。

## 复现与格式

在 Git Bash 中进入本目录：

```bash
bash scripts/fetch_source.sh
source scripts/env.sh
"$PYTHON" scripts/build_dataset.py
"$PYTHON" scripts/evaluate.py
```

`env.sh` 复用 `semantic-clip` 的隔离 Python、ONNX、tokenizer 和官方 `image_transform`。评估仅注册 `CPUExecutionProvider`。主要文件：

- `data/images/`：原始 JPEG，忽略提交。
- `outputs/contact-sheets/`：看图核验联系表，忽略提交。
- `manifest/quality-manifest.json`：模型无关的图像、标注与 review 数据源。
- `manifest/queries.json`：17 个有正例查询和 1 个零正例诊断，含中英文、正例选择器和两类 hard negatives。
- `outputs/chinese-clip/metrics.json`：总体与逐查询指标、图片数、标注依据、诊断结果。
- `outputs/chinese-clip/rankings.json`：每个查询的 Top10、正例 rank、hard-negative 最佳 rank/相似度/margin。
- `outputs/chinese-clip/typical-errors.json`：高分非正例及 hard-negative 失败。
- `outputs/chinese-clip/embeddings.npz`：归一化图像/查询向量，便于复核。

后续 `semantic-multilingual` 可复用同一 manifest：

```bash
"$PYTHON" scripts/evaluate.py \
  --model-dir ../semantic-multilingual/models \
  --model-manifest ../semantic-multilingual/outputs/model-manifest.json \
  --output outputs/semantic-multilingual \
  --language zh
```

可用参数覆盖 `text.onnx`、`vision.onnx`、tokenizer、文本/图像输入输出名、context length 和中英文查询。选择器支持 `image_ids`、`samples + frames` 和 `tags_any + frames`；所有 ID 都在 manifest 中固定。Top1/5/10 定义为“首个正例是否位于前 k”，MRR 为首个正例倒数排名均值。Hard-negative margin 为“最佳正例 cosine − 最佳负例 cosine”，越大越好。

## Chinese-CLIP 实测指标

评估库为上述 **150 张图**，常规查询 **17 个**，诊断查询 **1 个**。

| 指标 | 结果 |
| --- | ---: |
| Top1 hit rate | 4/17 = 23.53% |
| Top5 hit rate | 5/17 = 29.41% |
| Top10 hit rate | 9/17 = 52.94% |
| MRR | 0.31688794 |
| 主体对、动作错：正例胜出率 | 5/12 = 41.67% |
| 主体对、动作错：hard negative 为 Top1 | 0/12 = 0% |
| 主体对、动作错：平均 margin | +0.00163333 |
| 动作对、主体错：正例胜出率 | 4/10 = 40.00% |
| 动作对、主体错：hard negative 为 Top1 | 1/10 = 10.00% |
| 动作对、主体错：平均 margin | +0.00197401 |

平均 margin 接近零，不能掩盖多数配对失败；应同时看逐查询 margin 和排名。

### 查询清单与最佳正例排名

| 查询 | 最佳正例 rank | 主体对动作错 margin | 动作对主体错 margin |
| --- | ---: | ---: | ---: |
| 厨房里的水槽 | 1 | — | — |
| 锅或平底锅 | 1 | — | — |
| 勺子、叉子或刀具 | 1 | — | — |
| 海绵或刷子 | 18 | — | — |
| 机械臂用海绵擦拭锅 | 7 | +0.030579 | +0.002570 |
| 机械臂把锅放进水槽 | 81 | −0.017634 | −0.022007 |
| 机械臂把勺子放进水槽 | 9 | −0.003317 | −0.000613 |
| 机械臂打开微波炉 | 2 | +0.036310 | +0.026895 |
| 机械臂关闭微波炉 | 16 | −0.033354 | +0.023238 |
| 机械臂打开盒子的抽屉 | 16 | +0.035465 | −0.037716 |
| 机械臂关闭盒子的抽屉 | 42 | −0.031074 | −0.012263 |
| 机械臂擦拭灶台 | 11 | +0.027990 | −0.002763 |
| 机械臂擦拭洗衣机 | 7 | −0.009972 | +0.049451 |
| 机械臂把布放到洗衣机门上 | 8 | −0.014968 | — |
| 机械臂折叠布料 | 22 | −0.032702 | — |
| 机械臂把颗粒扫成一堆 | 1 | — | — |
| 机械臂把蘑菇放进锅里 | 82 | +0.032278 | −0.007053 |
| 机器人洗碗（零正例诊断，不聚合） | 不适用 | 不适用 | 不适用 |

每个查询的原始标注依据见 `manifest/queries.json` 和 `outputs/chinese-clip/rankings.json`。

## 典型错误

- 场景检索并不等同于动作检索：水槽、锅、餐具和扫颗粒最佳正例为第 1，但“海绵或刷子”仅第 18。
- `机械臂关闭微波炉`：开微波炉的 `T024_last` 排第 2，真正关闭动作最佳正例仅第 16，margin `−0.033354`。
- `机械臂打开盒子的抽屉`：开微波炉的 `T024_mid` 排第 1，真正开抽屉最佳正例第 16；这是明确的“动作对、主体错”。
- `机械臂关闭盒子的抽屉`：开抽屉和关微波炉均排在真正正例之前，最佳正例第 42。
- `机械臂把锅放进水槽`：最佳正例第 81；移动/取出锅和把叉勺放入水槽都更像模型的高分答案，不能据此声称它理解了锅或放置动作。
- `机械臂把蘑菇放进锅里`：最佳正例第 82，海绵入锅等配对也得分更高，物体与动作组合区分较弱。
- `机械臂折叠布料`：把布放在洗衣机门上的样本排第 2，折叠正例仅第 22。
- 多个动作查询中，同一轨迹的首帧高于中/末帧，显示静态场景偏置；这也是评估按帧限定动作正例而不是按整条轨迹粗放打标的原因。
- 零正例“机器人洗碗”的 Top1 是 `T004_first`（把叉子放进水槽里的银锅，cosine `0.481252`）。它只反映水槽/餐具场景相似，**不是洗碗证据**。

当前结果更支持“静态场景与物体相似度可用，但细粒度动作、主体和状态组合仍不可靠”的结论。未进行 1000 帧连续处理内存观察、RN50 对照或正式性能 benchmark。
