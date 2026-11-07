# AniKit

面向动画师的本地素材管理软件。AniKit 用封闭式素材库集中收管文件，也能通过索引式工作空间直接整理现有磁盘目录。

**当前版本：`0.1.0`（早期版本）。** 主要功能链路已经可用，但格式覆盖、界面细节和长期稳定性仍在完善，不应把它当作功能完整、无数据风险的稳定版。

## 下载与运行

前往 [GitHub Releases](https://github.com/kinglapen-design/anikit/releases) 下载最新的 Windows 免安装包。

1. 完整解压 ZIP，不要只解压或移动 `anikit.exe`。
2. 运行 `anikit.exe`，按向导创建或打开储存空间与资源库。
3. 需要反馈问题时，使用应用内「帮助 → 意见反馈」，并附版本、复现步骤和相关日志。

运行环境为 Windows 10 / 11 x64，需要系统 WebView2 Runtime。免安装包包含 FFmpeg、语义模型与运行时，因此体积较大；外部详细分析不是基础检索的前置条件。

## 已具备的能力

- 素材库与多资源库：导入、复制或剪切入库、文件夹、档案、链接、回收站、备份与恢复。
- 浏览与整理：网格和列表、分组、筛选、批量重命名、属性、描述、提示词和自定义数据。
- 搜索与预览：分词搜索、中文语义检索、图片 / 视频 / 动图 / 序列帧 / 音频 / 字体 / 文档预览。
- 语义片段：本地 CLIP 基础分析、可选外部详细分析、命中定位、虚拟片段与物理截取。
- 工作空间：直接浏览磁盘目录、链接外部文件夹、与素材库互通、Everything 加速搜索。
- 编辑与联动：Markdown 阅读 / 编辑 / 分屏、MD 打包导出、Animate CEP、Typora / Obsidian 贴图直链。
- AI 外接：MCP 只读与受控写入、可撤销操作、`anikit-cli.exe` 命令行入口。

## 使用与开发文档

- [使用教程](docs/使用教程.md)
- [v0.1.0 发布说明](docs/发布/v0.1.0.md)
- [产品需求与功能总览](docs/01_项目-PRD/00_功能总览.md)
- [接口文档](docs/05_接口文档/Index.md)
- [MCP 使用指南](docs/05_接口文档/MCP使用指南.md)
- [项目文档总索引](docs/Index.md)

应用内也可通过「帮助 → 使用教程」阅读同一份教程。

## 贡献者

- [kinglapen-design](https://github.com/kinglapen-design) —— 项目创建者与维护者。
- **Kimi（Moonshot AI / 月之暗面）** —— AI 辅助设计、编码、测试、文档与发布协作。

详见 [CONTRIBUTORS.md](CONTRIBUTORS.md)。

## 本地开发

需要 Node.js、pnpm、Rust 与 Tauri 的 Windows 构建环境。依赖版本以 `package.json`、`pnpm-lock.yaml` 和 `src-tauri/Cargo.lock` 为准。

```bash
pnpm install
python tools/fetch-ffmpeg.py
python tools/fetch-ffmpeg.py --dev-lib
tools/.venv/Scripts/python.exe -m pip install --target tools/libclang libclang
pnpm tauri dev
```

常用检查：

```bash
pnpm typecheck
pnpm build
cd src-tauri
cargo test --locked
```

完整免安装包还需要准备语义运行时与离线模型：

```bash
python tools/fetch-semantic-runtime.py
python tools/prepare-semantic-model.py
pnpm tauri build --no-bundle
python tools/pack-portable.py YYYYMMDD
```

打包脚本输出到本地 `release/`；该目录及大文件、模型权重和构建产物不进入 Git。

## 数据与隐私

素材库数据、缓存与工作空间默认保存在用户选择的储存空间内。本地基础语义分析和检索不调用外部服务。

只有主动配置并启动外部详细分析时，AniKit 才向该服务发送带时间戳的采样帧。API Key 写入 Windows 钥匙串，不回显，也不进入普通配置文件。反馈问题时不要上传密钥、个人素材或其他敏感信息。

## 授权

本项目当前**尚未选择开源许可证**。公开仓库不等于授予复制、修改或再分发授权；在补充明确许可证前，默认保留相应权利。

FFmpeg、模型、字体及其他第三方组件分别遵循各自的许可证与声明；相关文件随对应源码目录或发布包附带。
