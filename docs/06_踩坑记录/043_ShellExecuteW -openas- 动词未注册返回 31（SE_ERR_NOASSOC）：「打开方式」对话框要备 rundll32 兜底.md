## 043 ShellExecuteW "openas" 动词未注册返回 31（SE_ERR_NOASSOC）：「打开方式」对话框要备 rundll32 兜底

- **现象：** 2026-08-20 用户实机：素材右键「在其他应用打开」报"打开方式对话框调用失败，错误码 31"。
- **原因：** ShellExecuteW 的 "openas" 是**注册表动词**——该文件类型（或其 ProgID）没注册 openas 动词时系统直接返回 SE_ERR_NOASSOC（31），"打开方式"对话框根本调不起；与文件是否存在、路径是否合法无关。
- **解法：** 两级方案——openas 优先（有动词时最贴近资源管理器行为），失败回退 `rundll32 shell32.dll,OpenAs_RunDLL <path>`（shell32 导出入口，不依赖动词注册，弹同一个对话框）。rundll32 是正常按 CommandLineToArgvW 解析参数的程序，`std::process::Command` 可用（含空格路径 Rust 自动加引号）——与 explorer /select 的踩坑（吃原始命令行、不能 Command）不同，勿混。
- **预防：** ShellExecuteW 动词调用都按"动词可能未注册"设计（open / openas / edit 同理），错误码 ≤32 是旧 HINSTANCE 语义枚举（2 文件不存在 / 5 拒绝访问 / 31 无关联……），报错文案带上错误码方便实机定性。
