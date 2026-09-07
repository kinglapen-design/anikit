## 046 回收站卡片按下交框选回调：pointerdown preventDefault 吞 click 合成，单击选中全灭（探针合成 click 绕过 pointerdown 链，掩盖真实缺陷）

- **现象：** 2026-08-21 用户实机报"回收站的单选选中、双击名称改名等基础操作被改坏了"。探针首跑（`dispatchEvent(new MouseEvent('click'))`）却全绿——合成 click 不经 pointerdown 链，量不出这条缺陷。
- **原因：** `gridPointerDown` 的回收站分支把卡片按下直交 `useMarquee.onPointerDown`（卡片不在 ignoreSelector 内——正常视口被 beginAssetDrag 截走从不到此），其 `e.preventDefault()` 抑制兼容性 mouse 事件合成（click 不派发 → onCardClick 永不触发），未过阈值松开又按"点击空白"语义 `selection.clear()`——单击选中、双击链路全灭。缺陷自 6.1 二轮（2026-07-29）埋下，回收站低频 + 探针合成事件长期掩盖。
- **解法：** 回收站视口卡片按下**直接 return 放行**（不起拖拽、不交框选）——click 正常合成选中；框选不受影响（空白处按下照常发起）。
- **预防：** ① **点击类探针一律用 `Input.dispatchMouseEvent` 真实事件**（pointerdown→pointerup→click 全链），`dispatchEvent(click)` 只在验证纯状态逻辑时用——凡涉 pointer 链路的交互（选择 / 拖拽 / 框选 / 重命名）合成事件会绕过真链路给出假 PASS；② 手势分发层（gridPointerDown）分流到"带 preventDefault 的回调"前必须想清楚 click 还要不要活。
