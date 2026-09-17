# 077. Smart App Control 评估期满自动转「开」，本地 cargo 新编 exe 被 WDAC 拦截（os error 4551）

日期：2026-09-17。环境：Windows 11 + rust-msvc 工具链，本机开发。

## 现象

`pnpm tauri dev` 编译成功（Finished dev profile），起 `target\debug\app.exe` 即报：

```
error: could not execute process `target\debug\app.exe` (never executed)
Caused by: 应用程序控制策略已阻止此文件。 (os error 4551)
```

与代码无关——前一 commit 还能正常实机走查，本次仅新增了工作空间功能。同日上午第三方软件（Topaz Video 的 dll）也被同一策略拦截。

## 原因

**Smart App Control（SAC）从「评估」模式被系统自动切换为「开」**。判定链：

- 事件查看器 `Microsoft-Windows-CodeIntegrity/Operational`：Event 3077「cargo.exe 试图加载 app.exe，未达到 Enterprise 签名级别要求（Policy ID:{0283ac0f-...}）」+ Event 3118「Smart App Control Block Details」
- `HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy\VerifiedAndReputablePolicyState = 1`（0 关 / 1 开 / 2 评估）；`Win32_DeviceGuard` 的 CI 与用户态策略 EnforcementStatus 均 = 2（强制）
- SAC 规则 = 无声誉二进制一律拦；本地 cargo 每次重编产生新 exe，天然无声誉 → 每个新构建都被拦，开发循环直接断

SAC 的「评估」期只观察不拦，期满自动转「开」——表现为「某天起突然不能跑自己编的程序」。注册表无组策略接管（`SOFTWARE\Policies\Microsoft\Windows Defender` 无相关键），本机可自行关闭。

## 解法

设置 → 隐私和安全性 → Windows 安全中心 → 应用和浏览器控制 → Smart App Control 设置 → **关**。

代价（微软设计成单向）：关闭后想再开启**只能重置 Windows**；且 SAC 关闭会降低对未知应用的自动拦截，自行权衡。企业域控机器上开关可能为灰，需找 IT。

## 预防

无法从代码侧预防（签名也满足不了 SAC 的声誉要求；它是机器级策略）。沉淀两条排查口径：

1. os error 4551「应用程序控制策略已阻止此文件」≠ 代码/构建问题，先查 `CodeIntegrity/Operational` 日志定拦截者与 Policy ID，别在构建链上找原因（本会话曾为此绕 clippy 走 wrapper，方向全错）。
2. SAC 状态三值记牢：评估期不拦只观察，转「开」即拦一切无声誉 exe——开发机若要保持 SAC，需接受每次重编被拦的风险。
