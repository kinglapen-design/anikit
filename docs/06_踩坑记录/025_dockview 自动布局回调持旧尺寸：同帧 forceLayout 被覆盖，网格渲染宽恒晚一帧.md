## 025 dockview 自动布局回调持旧尺寸：同帧 forceLayout 被覆盖，网格渲染宽恒晚一帧

- **现象：** 侧栏收展动画过程中：① 顶栏（标签栏 46 C3）右端断开，露出 dock 宿主 C1；② 中间面板右缘锚点失效——收左栏时右缘应钉死不动，实际整个网格（含滚动条）被往左拽、动画结束再回弹；③ 收展末尾因重排闪烁一帧。三现象同根。
- **原因：** App.vue 的收展驱动在同一 rAF 回调里写完侧栏宽度后 `forceLayout(当前宽)` 灌给 dockview——但 dockview 自身的自动布局走 `ResizeObserver → requestAnimationFrame`（dockview-core `dom.ts watchElementResize`），回调注册在同一帧的 rAF 队列里且**排在我们之后、携带的是触发时的旧尺寸**：我们先灌当前宽，它紧随其后把网格宽度覆盖回上一帧。CDP 逐帧实测：网格渲染宽恒等于 dock 宿主**上一帧**宽，滞后最多 ~75px。
- **解法：** 灌宽时机从 rAF 阶段改到 **ResizeObserver 阶段**——App.vue 挂 `dockResizeObs` 监听 dock 宿主，RO 回调在当帧全部 rAF 之后、绘制之前执行，此刻读到的就是当帧最终宽，`tabs.forceLayout` 同步写网格；dockview 的陈旧回调此前已执行完毕，再也覆盖不到。侧栏收展、窗口拖改尺寸统一走此路径。逐帧复测：每一帧 `dockHost == grid == tabs` 精确到 0.1px，收左栏时网格右缘恒定钉在窗口右缘。
- **预防：** 与第三方组件库做"逐帧尺寸同步"时，先搞清对方自动布局的事件源与时机（RO / rAF / 微任务），在**比它更晚、仍在绘制前**的时机同步（RO 阶段是浏览器渲染流水线里绘制前最后一站）；同帧内后执行的旧尺寸回调会覆盖先执行的同步写入——顺序错误时"同步灌值"形同虚设。排查这类逐帧问题用 CDP 劫持 rAF 逐帧步进 + `getBoundingClientRect` 探针，比截图猜因快得多。
