## 034 std::process::Command 自动引号转义 vs explorer 原始命令行解析（/select 回退"文档"）

- **现象：** 2026-08-14 序列帧"在资源管理器打开"（文件名含空格）恒跳"文档"文件夹——explorer 对解析失败的路径静默回退，无报错。
- **原因：** 两层叠加。① 裸路径含空格时 explorer 自解析截断；② 手写 `format!('/select,"{path}"')` 经 `std::process::Command::arg` 传递时，**Rust 自动给含空格参数加外层引号、内层引号转义为 `\"`**——而 explorer.exe 不走 CommandLineToArgvW（吃原始命令行），转义后的 `\"` 直接断路径。PowerShell 直传同字符串能成功（原样传递），对比实验才定位到这一层。
- **解法：** 走 `ShellExecuteW`（与 open / open_as 同通道），lpParameters = `/select,"路径"` 原样传递，绕开 Command 的转义层。
- **预防：** Windows 下调系统程序传**预格式化参数字符串**（含引号的开关参数）时，不要用 `Command::arg`（它按 argv 语义再加工）；用 ShellExecuteW 或 `CommandExt::raw_arg`（原样拼接）。诊断注意：git bash 传中文参数有编码失真，验证此类问题用 PowerShell 起进程才可信。
