## 058 flex/grid 容器的 trailing gap 被 Chromium 计入 scrollHeight——高度展开动画终值偏大，切回 auto 瞬间兄弟回弹

- **现象：** 2026-08-30（阶段七十八轮，树容器展开动画）。最外层父级展开时，被推下去的兄弟层级在动画收尾"回弹一下"。cdp-tree-expand-probe 逐帧采样实证：`.ft-children`（flex column + gap 2，3 子项 26 高）`getBoundingClientRect().height` = 82，而 `scrollHeight` = **84**——多出一个 trailing gap。展开动画 0→84px 补间，transitionend 后清 inline height 切回 auto（82），兄弟从 +86 跳回 +84 = 2px 回弹。
- **根因：** Chromium 的 scrollable overflow 计算把 flex（grid 同）容器的**尾部 gap** 计入内容高度，scrollHeight 比真实 auto 高度大一个 gap；嵌套 flex-gap 容器每层各贡献一个，层级越深误差越大（所以最外层父级回弹最明显）。
- **解法：** 高度测量一律 `getBoundingClientRect().height`（元素正常布局时 = auto 高，含小数精度），`lib/expandAnim.ts` enter/leave 两处全改；scrollHeight 在此类动画测量场景禁用。
- **预防：** 凡"实测元素高度驱动 JS 补间"的代码，先想测量值是否等价于 auto 布局高——scrollHeight 在 flex/grid gap 容器上恒偏大；写动画探针断言兄弟位移时把"超调量 ≤1px"列进去（本坑靠 overshoot 断言直接显形）。
