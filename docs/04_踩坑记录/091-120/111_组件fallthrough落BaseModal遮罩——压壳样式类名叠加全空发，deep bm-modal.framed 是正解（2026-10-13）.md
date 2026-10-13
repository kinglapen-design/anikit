# 组件 fallthrough class 落 BaseModal 遮罩——压壳样式类名叠加全空发，:deep(.bm-modal.framed) 是正解（2026-10-13）

## 现象

设置面板（SettingsDialog）要压 BaseModal framed 壳的列 gap 与内容段 padding（让深色区贴标题线）。给组件根元素挂 `class="settings-dialog"` 再写 `.settings-dialog.settings-dialog { gap: 0 }` 这类叠加选择器，改完「看着生效了」（一轮圆角还「通过」了目验），四轮后用户截图指出标题线下仍有壳色横带——**样式从未打到弹窗本体**。

## 原因

Vue 的 fallthrough attribute 把组件上挂的 class 落到 BaseModal 的**根元素 = 遮罩 `.bm-mask`**，而不是内部弹窗 `.bm-modal`。所以 `.settings-dialog.xxx` 压的全是遮罩（无 gap/padding 可压），弹窗本体上的 framed 样式分毫未动。叠加类名抬优先级只是更高地打空。

隐蔽点：遮罩是全屏层，挂它上面的 `overflow: hidden` 之类属性**偶尔会产生「生效了」的错觉**（裁剪遮罩 ≈ 裁剪视口），目验截图不放大对比边缘根本分不出；DOM 探针量错元素也会报「属性正确」。

## 解法

实例级压壳样式走 `:deep()` 直指出口：

```css
.settings-dialog :deep(.bm-modal.framed) {
  gap: 0;
  padding: 0;
  overflow: hidden;
}
```

优先级 (0,4,0) 稳压共享件 `.bm-modal.framed` (0,3,0)，**与样式表加载序无关**；BaseModal 共享件本身零改动。

## 预防

- **给弹窗类组件挂 class 时先想它落在哪**——BaseModal / ConfirmModal 这类「根 = 遮罩」的组件，fallthrough class 永远落遮罩；要压弹窗本体必须 `:deep(.bm-modal)` / `:deep(.bm-modal.framed)`。
- 「样式好像改了又好像没改」时第一步 = DevTools 确认选择器命中的到底是哪个元素，别靠截图观感判定生效。
- 存量同构：WsManageDialog 的 `class="ws-manage"` 也落在遮罩上（当前无样式消费、无害登记在此）。
