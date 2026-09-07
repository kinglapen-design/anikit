## 059 trailing gap 横向同款：dockview 自绘横向滚动条因 scrollWidth 假溢出 4px 横贯标签组底部

- **现象：** 2026-08-30（阶段七 7.1 十九轮，中间面板-标签栏轮）。标签栏贴稿后截图目验：一条 4px 灰色条带（rgba(255,255,255,0.06)）横贯标签组正下方，选中标签与内容容器的无缝衔接被截断。elementFromPoint 定位 = `.dv-scrollbar.dv-scrollbar-horizontal`（dockview 7 自绘滚动条，`.dv-scrollable` 包装层内）。
- **根因：** 058 同族的横向切面——Chromium 把 flex 容器的 **trailing gap 计入 scrollable overflow**（规范行为）：两枚 180 标签 + gap 4 = 内容 364 = clientWidth，但 scrollWidth = 368（末尾多算一个 gap）。恒 4px 假溢出 → 横向滚动条恒显。第一误判成 `.dv-tab-group-underline`（分组指示线）——display:none 后 DOM 查无此元素仍见线，才回元素定位正主（**先 elementFromPoint 再动手**，别按样式特征猜归属）。
- **解法：** `.anikit-dock-theme .dv-scrollbar { display: none }`（标签页 shrink 平分策略下本就无滚动场景；触控板/滚轮滚 tabs-container 原生 overflow 不受影响）。分组指示线另加防御性 display:none（分组着色启用时会冒出）。
- **预防：** flex/grid + gap 容器的滚动/测量一律把 trailing gap 算进去（058 高度、059 宽度）；dockview 7 的 `.dv-scrollable` + 自绘 `.dv-scrollbar` 接管了 tabs 区滚动——**贴稿排查装饰线先列 dockview 7 新增件清单**（scrollbar / tab-group-underline / tab-group-chip），再怀疑自己的代码。
