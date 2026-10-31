# 116 Vite 监听大模型权重触发 EBUSY

## 现象

离线 CLIP 模型整理完成后，设计预览服务退出，日志为 `Error: EBUSY: resource busy or locked, watch '…/assets/models/multilingual-v1/models/text.onnx'`。

## 原因

Vite 默认监听项目文件，包含数百 MB 的 ONNX 权重。Windows 下模型复制、杀毒扫描或读取占用与 watcher 竞争，导致进程直接退出。权重只供 Rust worker 读取，不是前端模块，监听它没有价值。

## 解法

在 `vite.config.ts` 的 `server.watch.ignored` 排除 `assets/models/`、`release/` 和隔离验收目录，保留源码监听。模型目录继续由 `prepare-semantic-model.py` 校验、打包脚本复制，不进入前端构建图。

## 预防

模型、发布产物和验收数据库都不要放进前端 watcher。新增大文件目录时先明确它是源码、静态资源还是后端数据；后端数据同时排除 Git 大文件提交与前端监听，不能只靠 `.gitignore` 代替 watcher 配置。
