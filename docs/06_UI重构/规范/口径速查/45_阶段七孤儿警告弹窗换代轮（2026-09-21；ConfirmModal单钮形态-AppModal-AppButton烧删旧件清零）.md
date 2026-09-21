# 45 阶段七孤儿警告弹窗换代轮（2026-09-21；ConfirmModal 单钮形态 + AppModal/AppButton 烧删，旧件清零）

> 用户实机走查供稿定案：OrphanInfosAlert（无主资产警告）换新弹窗，列表行「用类似 MCP 接入配置这种代码块包裹起来」，行尾放「在资源管理器中打开」图标。§40/§44 遗留的 alert 系缺口至此全关闭，**AppModal/AppButton 烧删，LEGACY 表清空，旧件清零**。

## OrphanInfosAlert 换代口径

壳 = **modals/ConfirmModal kind="warning"**（alert warning → warning，§24 映射的 alert 系第二件落地）：48 warning 徽章废，标题 18/26 Bold 一级、正文 13/18 三级随组件层。

- **ConfirmModal 单钮形态（本轮扩展，用户定）**：`cancelLabel` 传空串隐藏取消钮，仅留确认——孤儿警告「知道了」即此形态。稿未画单钮弹窗，按既有件扩展未自创样式。
- **列表行 = md 代码块口径**（引自 MCP 接入配置行，§37 同款）：圆角 6 / 底色与描边 亮色/半透明-4（描边 inside）/ padding 6·8 / 图标钮贴右；行内文本 = `<b>名称</b>：路径`（加粗二级 + 正文三级），长串 `overflow-wrap: anywhere` 折行；图标钮与首行文字基线对齐（margin-top 1px，14 盒对 18 行高）。
- **行尾入口 = 在资源管理器中打开**：IconButton `open-in-explorer` bare sm（既有库件，首批已转 currentColor）→ `lib/libraryOps.revealOrphanInfo(o.path)`（o.path = info 目录完整路径直传 `reveal_path` 通用命令，失败 toast）。删除按钮用户未提，仍不做。
- **提示音不丢**：ConfirmModal 不带音，组件内 `watch(orphanInfosState.list) → playAlert()`（GroupManage 同先例）。
- 沿旧用户定：行间距 8 / 清单超高自滚上限 160 / kit-scroll + ScrollThumb 负 margin 对贴右缘。

## AppModal / AppButton 烧删

OrphanInfosAlert 迁出后消费方清零（preview 不计入台账口径），两件随轮删除；`gen-component-inventory.mjs` LEGACY 表清空。**台账：146 件 = 新 72 / 业务 72 / 旧件 0 / 休眠 2（DetailNotes / FolderPickNode）**。

- 顺手烧减：HoverAudioLayer `var(--brand-1)` → `var(--n-brand)`（旧兼容层别名同值），**组件内 legacy token 同步清零**。
- 图标 `error-large` / `warning` / `window-close` 等随 AppModal alert/standard 变体失去组件内使用处——图标是设计资产留库不删，后续随用随转。
- dev 验证：删件后 vite HMR 正常（踩坑 081 的动态 import 挂页未复发——两件删前已无引用）。

## preview 同步

- PreviewModals：新增**单钮格**（warning「知道了」）；复刻钮口径滞后顺手修正——`.pv-modal-btn` 120 定宽废，改宽自适应 + 左右 48（与 ConfirmModal 组件层 2026-09-13 定稿值同步），展区 desc 同改。
- PreviewDialogs：OrphanInfosAlert 复刻格换新（pv-cm-shell = ConfirmModal 口径复刻 + md 代码块行 + 真实 IconButton），② 档「仍旧 AppModal alert 待稿」标注清零；pv-old-alert 系列复刻样式烧删（pv-old-shell 留 RecycleConfirm 用）；AppButton import 清除。

## 探针账

孤儿警告无专属探针，无新增失效；cdp-import/export-failed 旧壳断言失效账已记 §44（随用随修）。
