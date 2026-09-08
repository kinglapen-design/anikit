## 073 dockview 布局不吃 Emulation 视口收窄——探针模拟侧栏挤压要直接钉 inline 宽（2026-09-09）

- **现象：** 搜索栏挤压修复轮的回归探针（cdp-search-probe S20）用 `Emulation.setDeviceMetricsOverride` 把视口收到 560，断言中间面板变窄——`window.innerWidth` / `body.offsetWidth` 都到了 560，但 `.content-area` 纹丝不动（panelW 恒 891），挤压场景根本不成立。
- **原因：** dockview panel 宽度由 dock split 布局自身管理（split 百分比 × dock 容器，内部 ResizeObserver 驱动），不随视口 override 即时重排；主窗口三栏的宽度链不在文档流响应式上。
- **解法：** 探针里直接给 `.content-area` 钉 `style.width = '500px'`（等价侧栏挤压后的窄态），验证目标本就是「壳在窄容器内的裁剪 / 钉住 / 横滚行为」，不关心容器变窄的力学。断言壳 `max-width:100%` 钳制、收起钮在壳内、筛选组 scrollWidth > clientWidth、wheel → scrollLeft 生效，全绿。
- **预防：** CDP 探针要模拟「布局挤压 / 容器变窄」时，先确认目标容器的宽度由谁管——**视口收窄只对文档流响应式布局有效**；dockview / split 管理的容器直接钉 inline 宽或调 dock API，别在 Emulation 上空耗（本轮白跑两轮）。
