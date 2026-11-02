# 2026-11-02 preview 可视化 token 调参面板

用户反馈：token 烧是烧了，但布局还是偏旧版，文字描述调间距太麻烦——preview 界面能否像 opendesign 那样直接调。本轮落地 dev 专用调参面板：数值实时改、效果实时看、定稿一键导出。

## 定案

在 `#design-system` preview 页加右下角「调参」浮动钮 + 340px 右侧抽屉。**数据源 = `src/styles/tokens.css?raw` 全量解析**（新体系 + legacy 旧体系变量都在列，旧布局还在吃旧 token，正好覆盖「布局偏旧版」的调参面）；每变量显示名称 / 行内注释 / 分组，长度（px）与时长（ms）给滑杆 + 数字框，颜色给色块 + 文本（#hex 加原生取色器），字重等纯数字给数字框，字体族 / 阴影 / 渐变等给文本框。

改动实时 `document.documentElement.style.setProperty` 写 `:root` inline——preview 与主 App 同文档，**主界面同吃**；localStorage（`anikit.devTokenOverrides.v1`）持久化，`main.ts` dev 启动时经 `applyDevTokenOverrides()` 恢复（动态 import，生产零进入）。改动行品牌色高亮 + 左缘色条 + 「原值」小字 + 单项 ↺ 重置；工具行 = 搜索（名称 / 注释）/ 分组过滤 / 仅已改 / 全部重置；**「复制覆盖 CSS」导出 `:root { --n-space-8: 10px; /* 原 8px */ }` 形态**，贴回 tokens.css 或发 AI 即定稿依据，另有「复制 JSON」。

## 验证与一处环境边界

内置浏览器目验（vite dev + #design-system）：space-8 改 10px 后滑杆 / 数字 / 高亮 / 原值 / 重置钮全就位；localStorage 在 HMR 与新 tab 下均恢复；「仅已改」过滤与单项重置闭环；typecheck 0 错。测试覆盖已全数重置清零，不留污染。

**复制按钮在纯内置浏览器被 clipboardWrite 站点权限弹窗拦截**（该弹窗仅用户能回答，探针纪律不硬修）——已加兜底：复制失败时面板底部展示导出原文 textarea，全选手动 Ctrl+C，不丢产物。Tauri WebView2 内剪贴板行为待实机确认。

## 待实机走查

`pnpm tauri dev` 实例里：主 App 各布局调 token 的实时观感（尤其 legacy 变量对旧布局的作用面）、重启后恢复、复制按钮在 WebView2 的表现、导出 CSS 回写 tokens.css 的定稿流程。调出来的满意数值发我或直接改 tokens.css 均可——本面板只产「覆盖层」，不自动改源文件，定稿动作仍在人。

文件：`src/preview/devTokens.ts`（解析 / 应用 / 存储 / 导出）、`src/preview/DevTokenPanel.vue`（UI）、`src/main.ts`（dev 启动恢复）、`DesignSystemPreview.vue`（挂载）。零后端、零新依赖、零新组件入生产。
