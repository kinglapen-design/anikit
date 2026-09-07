## 060 容器 overflow 裁到内容盒——标签页反弧肩膀（外溢伪元素）首尾枚外侧恒被裁掉

- **现象：** 2026-08-30（阶段七 7.1 十九轮三轮，用户实机）。选中标签页的反弧肩膀（`.tab::before/::after`，4×4 位于本体左右 -4px）只有中间标签可见：单标签时双肩全无，多标签时最左缺左肩、最右缺右肩。computed 样式一切正常（伪元素尺寸/位置/渐变全对）——**在 ≠ 可见**。
- **根因：** dockview 7 的 `.dv-scrollable`（overflow hidden 包装层）与 `.dv-tabs-container`（overflow auto/hidden）都裁到**内容盒**边缘：tab 是 flex item 贴容器内容缘，外侧肩膀伸出内容盒即被裁；中间标签的肩膀落在 gap 4px 内（gap 属容器内部）所以正常。预览页没暴露是因为 demo 栏 padding 36/24 把首枚 tab 推离了容器缘。
- **解法：** 两层各加 `padding: 0 4px + margin: 0 -4px`（负 margin 抵消，tab 视觉位置不变，padding 区供外溢肩膀绘制）——overflow 裁剪边界 = padding box 缘。不改 overflow 机制（滚动测量/动画依赖保留）。
- **预防：** 伪元素/子元素外溢绘制（负位偏移、反弧、光环）先查祖先链 overflow——**dockview 7 包装层多（scrollable/tabs-container/tabs-and-actions-container）逐层查**；断言此类视觉效果必须截图读像素（本坑探针 computed 断言全绿、实机全无），像素采样点取纯色区（radial-gradient 反弧取贴角 1px 处，距弧心 > 弧半径）。
