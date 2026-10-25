# 2026-10-24_09 Chinese-CLIP 双后端 spike

真实 Chinese-CLIP + Rust ORT 已跑通，主代理独立复跑一致：CPU 仅 `CPUExecutionProvider`，DirectML 仅 `DmlExecutionProvider`，本机无回退。文本首次推理 18.52ms → 4.92ms，图像 45.36ms → 5.69ms；峰值工作集约 1.14GB / 1.01GB。两塔输出 `[1,512]`、L2 约 1，与 Python 参考一致，官方样例分类正确。

运行时 = `ort rc.15` + ORT DirectML 1.24.4 / DirectML 1.15.4，私有 DLL 约 35.9MB；复现脚本与 Rust 源码已提交 `484816eb`，`cargo check --release --locked` 通过。

**仍未关闭**：权重许可未声明，40–60 动作片段质量、1000 帧耐久、RN50 和 Rust 原生图像预处理未完成。后续 `spike/semantic-eval` 做真实质量与 hard negatives，`spike/semantic-multilingual` 验许可清晰备选。路线图第 2 项未勾。详见 [开发日志](../../../02_开发日志/2026-10/2026-10-24_09_ChineseCLIP_CPU_DirectML_spike验证.md)。
