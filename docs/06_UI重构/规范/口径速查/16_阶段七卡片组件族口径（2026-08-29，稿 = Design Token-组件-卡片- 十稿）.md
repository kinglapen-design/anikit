# 16. 阶段七卡片组件族口径（2026-08-29，稿 = Design Token/组件/卡片/ 十稿）


- **组件位置 = `src/components/cards/`**（6 件 + 共享 `cards.css`）：FileCard（通用 / 视频 / 音频 / 字体 / txt-md 五稿合一，kind prop）/ FolderCard / ArchiveCard / ListRowCard / ListHeader / FormatBadge；与旧根目录同名卡片件并存，业务接入随列表容器重构轮替换。
- **媒体卡壳 `.kc-media`**（文件卡 / 档案卡）：g2 底 + 圆角 8；hover = 位移 Y -4px + 小阴影（250ms 小位移）。**线条 = 旧统一方案双层伪元素 inset 0**（2026-08-29 走查改回——首版「真 1px border + 选中环 inset -1px」在分数物理像素下渗线、粗细不均，文件夹卡无真 border 反而正常）：常态 1px = `.kc-bordered::after`（**2026-08-30 定案 = 半透明环**：常态 l-a4 / 媒体卡 hover l-a2，深底 ≈ g3/g6 等效亮度，与图片混合压锯齿，§7；列表行 hover 仍 g5 实色；瞬时换色不做过渡，踩坑 036）、选中 2px 品牌 = `.kc-sel::before` opacity 过渡；**禁真 border、禁负 inset**；内容圆角裁切走 `.kc-clip` 内层。**圆角平滑三件套（2026-08-30，§7 同款口径）**：壳不挂 overflow:hidden（单层裁剪）、铺满缩略图/封面自带 radius 8（`.fc-thumb` cover/contain、`.ar-cover`）、选中/命名态常态环 transparent 让位（cards.css `.kc-sel.selected::after` + FileCard/ArchiveCard/ListRowCard naming scoped）。
- **悬停控件 hover 才现（2026-08-29 用户定）**：视频/音频右上胶囊（`.kc-float`：半透明-2 + 立体感-通用 + 背景模糊-小 + 圆角 4；视频 = 帧数 + 播放/暂停 + 音量/静音，音频 = 时长 + 播放/暂停；图标二级、自身 hover 升一级）+ 视频底部 4px 播放条（轨道 亮色/半透明-2、填充 亮色/一级，贴底 4、左右 12）；**音频波形进度 = 品牌色同构波形层 clip-path 扫过**（与根数解耦，稿 prose；颜色 2026-08-29 用户定品牌色），hover / 播放中才显。波形竖线 4px gap 4 五级，圆头 99px 为稿值（Token.md 圆角无此档，非新造 token），根数恒定由业务保证。
- **格式角标统一样式 = FormatBadge**：`.kc-float` 铬 + 数字档 12/16 二级；文件卡左上 12/12、档案卡（属性文字如「主角」）、列表行扩展名列同一件。
- **文件夹卡**：g3 底 + 立体感-通用（**无 1px 描边**），hover 仅底升 g6（无位移无阴影——稿只给 background-color）；名称 标题档一级 + 文件夹-大 24 双色调**恒默认色不接 tint**（2026-08-29 用户定：卡片变色整屏噪声大，自定义色只留树容器）；简介统计两行数字档三级、段间 " / " gap 4；名称行与简介间距 36（稿值填满 122 高链）。
- **档案卡**：正方形（aspect-ratio 1，稿注）；封面 cover 充满；渐变罩三行 = 名称 标题档一级 / 属性行 " | " 分隔 / 数量行 " / " 分隔（数字档 二级）；右下 = linked-file-badge 14 二级（已转 currentColor）。
- **列表行**：57 高 padding 6/12/6/6 gap 12，g2 底 + g3 1px 描边；hover = 底升 g3 + 描边升 g5（瞬时）；缩略图 80×45（g3 底 + g5 描边 + 圆角 6，图 **contain**——沿用旧 ListRowCard 口径，与网格卡 cover 不同）；名称列 flex 1（正文-加粗 一级 + 原始文件徽章 14），数据列各 123 宽 数字-大档 三级。
- **列表表头**：g3 底 + 立体感-通用，高 34 padding 8/12 gap 12；列与行对齐（名称 flex 1，其余 123）；标签 正文-加粗 三级，hover 升二级 + **箭头 hover 现身 / 当前排序列恒显**；箭头 = chevron-small 单箭头旋转换向（朝上 = 升序原样、降序 rotate 180°，**300ms 形态切换**——稿注「上下箭头不同时出现」），常态 四级 / 自身 hover 三级。
- **通用卡 square 变体**：shell 缩略图 / 自动导入未导入 = 正方形 + 缩略图 50% 居中（稿注）；网格卡缩略图 cover 充满（沿用旧 AssetCard 口径）。
- **txt-md 卡**：卡内文本滚动预览（标题 标题档二级 / 正文 正文档四级 gap 8，滚动内容入渐变罩下）；内容溢出出 6px 滚动胶囊贴右缘（g3，**胶囊自身 hover 升五级**——稿「hover-滚动条」节），跟随滚动。
- **尺寸模型**：卡片组件宽高 100% 随父容器（稿值 = figma 画布语境），预览页按稿值演示；网格行列尺寸归列表容器布局轮。
- **业务接入登记**：素材网格 = AssetCard 壳接 FileCard（7.1 内容容器轮）；收件箱「工程文件」磁盘实况区 = InboxDiskSection 2026-09-08 换代 FileCard square 变体（旧 14 稿散装样式 C2 图标区 + C3 简介区 + 实色扩展名角标废除）——badge 恒 = 扩展名大写（目录不出）；目录条目 folder-closed 40 整卡居中保旧、被占用标记保旧视觉右上 4 壳层随 hover 位移（两处新稿未画，同 AssetCard 笔记角标「存量元素保旧」先例）。
- **动画时长**：hover / 颜色 150ms；位移 -4 + 小阴影 250ms；表头箭头旋转 300ms；悬停控件显隐 150ms。
