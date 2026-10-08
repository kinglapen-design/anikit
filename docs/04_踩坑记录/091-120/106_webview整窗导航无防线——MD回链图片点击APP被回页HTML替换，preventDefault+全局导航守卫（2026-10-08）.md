# webview 整窗导航无防线——MD 回链图片点击 APP 被回页 HTML 替换（2026-10-08）

## 现象

anikit 协议消费端落地首日实机走查：内部 MD 编辑器里点一张图片，整个 APP 窗口内容变成一行「已在 AniKit 中定位，可关闭本页」——AniKit 界面消失，关也关不掉，只能重启。

## 原因

带跳转形态的 MD 图片 `[![](http://127.0.0.1:17701/item/x)](http://127.0.0.1:17701/open/item/x)` 渲染为 `<a href="open端点"><img class="md-asset"></a>`。PreviewText 点击代理的 img 分支命中后做了内部跳转就 return，**漏了 `e.preventDefault()`**——包裹它的 `<a>` 默认导航照常触发，而 **webview 不是浏览器**：http 链接导航 = 当前窗口整页被 open 端点返回的 HTML 替换，APP 本体没了（跳转动作其实成功了，只是窗口同时被换走）。同代理的 a 分支本来就有 preventDefault，唯独 img 分支（先命中先 return）漏了。

深层是防线缺失：整个 APP 此前没有「webview 永不导航到外部 URL」的兜底——任何组件渲染出一个 http `<a>` 且忘了 preventDefault，就是同款事故。

## 解法

两层：①img 分支补 `preventDefault`（本 bug 直接修复）；②App.vue 立全局导航守卫——document 捕获阶段拦截一切 `<a href="http(s):// / anikit://">` 的默认导航并 stopPropagation 全权接管（anikit → `jumpToAnikitRef` 内部跳转，http(s) → `open_external_url` 默认浏览器），组件内冒泡处理不再跑（行为等价）。顺带补：分屏右栏渲染体此前没挂点击代理（复制钮 / 复选框 / 链接全点不动），同轮挂上 `onMdClick`。

## 预防

- **webview 里渲染 `<a href>` 必须配 preventDefault**；包在 a 里的可点子元素（img 等）分支先命中时同样要拦——默认导航在祖先 a 上，不在被点元素上。
- **webview 应用要立「整窗不导航」全局守卫**（document 捕获层拦 a 默认行为）：组件级 preventDefault 总会漏，防线必须在根上。
- 点击代理各分支自查顺序：先命中的分支（img）不能把拦截责任留给后分支（a）——谁处理谁拦截。
