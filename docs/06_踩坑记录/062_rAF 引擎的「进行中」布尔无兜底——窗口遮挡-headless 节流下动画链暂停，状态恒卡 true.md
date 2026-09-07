## 062 rAF 引擎的「进行中」布尔无兜底——窗口遮挡/headless 节流下动画链暂停，状态恒卡 true

- **现象：** 2026-08-30（阶段七 7.1 十九轮六轮，用户实机 + 探针 flake 双实证）。`sidebarAnimating` 一旦置 true 只能靠 rAF 回调链跑到终点才清——headless 探针里 T13c 间歇 FAIL（pad-win 恒 160），用户实机同类：窗口遮挡/失焦时 rAF 暂停，若回调链断掉（或暂停长于预期），布尔恒 true，pad-win 让位永不解除（标签栏右端恒 160 空白）。
- **根因：** rAF 不是可靠定时器——页面隐藏/遮挡时浏览器可完全暂停派发；「动画完成才复位状态」的隐式假设在真实桌面环境不成立。
- **解法：** `useSidebarAnim` 两引擎（左右栏 + 档案面板）各加 **watchdog**：置位时同步 `setTimeout(动画时长 + 200)`，超时若动画仍在册 = 链断，强制写终值（showWidths + el.style.width）+ 清状态 + cancelAnimationFrame；正常完成路径 clearTimeout。探针侧同类断言一律**轮询等待**（deadline + 间隔复查），不用固定 sleep（动画 + RO 灌宽时序不定）。
- **预防：** 凡「rAF 驱动 + 布尔进行中标志」的结构必须配 watchdog 兜底；评审动画引擎代码时把「隐藏/遮挡时这个布尔会怎样」列入检查项。关联：054/056（headless 环境时序与实机差异同族）。
