---
link: "[[00_组件目录]]"
---
## 01. 基础 item

``` css
/* 按钮容器：图标 + 标题 */
/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 0px;
gap: 6px;

margin: 0 auto;
width: 图标 + gap + 标题;
height: 子项;

/* Inside auto layout */
flex: none;
flex-grow: 1;

/* 图标 */
width: 16px;
height: 16px;

color: border font-icon-2 fill var(--font-icon-4);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;

/* 标题 */
font: [[01_样式 token#02. 文字]] / 正文;

color: font-icon-1;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;
```

## 02. 文件树 item

展开按钮作为空间占位，要一直保持，无论需不需要展开按钮的文件树 item，都需要保留展开按钮的空间占位，以保证 item 之间的装饰图标（如文件夹图标 / 标签图标）可以保持列对齐。

``` css
/* 主容器：展开按钮容器 + item 容器 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 2px;

min-width: 150px;
height: 子项;

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 展开按钮容器：展开按钮图标 */
/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 0px;

width: 子项; /* = 展开按钮图标 8px：item 组左 4 + 容器 8 + gap 2 = 14，与汉堡菜单 / 副标题按钮左缘对齐 */
height: 主容器高度; /* 增加鼠标判定范围，方便点选 */

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;

/* 展开按钮图标 */
width: 9px;
height: 9px;

background: font-icon-3;

hover: background font-icon-2;

/* Inside auto layout */
flex: none;
flex-grow: 0;

/* item 容器：基础 item + 数量标签 */
/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 4px 6px;
gap: 6px;

min-width: 140px;
height: padding + 子项;

border-radius: 6px;

hover: background font-icon-6;

active: background font-icon-5; /* 持久化选中 */

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 1;

/* 基础 item */
[[#01. 基础 item]];

order: 0;

active: 容器内图标 border font-icon-1 fill font-icon-4; /* 持久化选中 */

/* 数量标签 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: font-icon-3;

active: color C4; /* 持久化选中 */

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;
```

## 03. 右键菜单 item

有二级菜单的 item，用提示右箭头图标替换掉快捷键标签

``` css
/* 总容器：基础 item + 快捷键标签 or 二级菜单提示右箭头图标 */
/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 4px 6px;
gap: 16px;

width: 填充父容器;
height: 子项;

border-radius: 6px;

hover: background font-icon-6

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 基础 item */
[[#01. 基础 item]];

order: 0;

/* 快捷键标签 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: font-icon-3;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;

/* 二级菜单提示右箭头图标 */
width: 8px;
height: 8px;

background: font-icon-3;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;
```

## 04. 右键菜单分割线

``` css
/* 分割线容器：线条 */
/* Auto layout */
display: flex;
flex-direction: column;
align-items: flex-start;
padding: 2px 0px;

width: 填充父容器;
height: padding + 线条;

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 线条 */
width: 填充父容器;
height: 0px;

border: 1px solid border-1; /* 圆头 */

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;
```

## 05. 区块副标题 item

有的区块需要添加按钮，比如文件树需要添加文件夹等，但有的区块不需要添加按钮，不要需要添加按钮的直接删掉即可。

折叠按钮和添加都按钮用：chevron-small，add-small（16×16 显示），按钮为图标按钮-小 24×24（padding 4 / 圆角 4）；常态图标 font-icon-3，hover background font-icon-6 / 图标 font-icon-2。

> **顺序（2026-07-23 用户定）：标题 + 数量 + 展开箭头 + …… + 添加按钮——展开箭头与标题交换位置，箭头在标题组之后**。整行（标题 / 数量 / 箭头）均为折叠判定区；详情面板 grip 变体 hover 整行抓手 + 提亮 font-icon-1，**添加按钮不参与抓手**（常规图标按钮 hover 底色）。

``` css
/* 总容器：标题容器 + 添加按钮 */
/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 0px;
/* 无间距：X 内边距兼作间隔，文件名右 4 */

width: 填充父容器;
height: 子项;

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 标题容器：标题 + 数量 + 展开按钮 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
/* 无间距：X 内边距兼作间隔，文件名右 4 */

margin: 0 auto;
width: 按钮 + gap + 标题;
height: 子项;

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 1;

/* 展开按钮 */
[[01_按钮#01. 图标按钮 - 无底色装饰]] / 小图标;

color: 容器内图标 font-icon-3;

order: 0;

/* 标题 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: font-icon-3;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;

/* 添加按钮 */
[[01_按钮#01. 图标按钮 - 无底色装饰]] / 小图标;

color: 容器内图标 font-icon-3;

order: 1;
```

## 06. 详情面板关联文件夹 item

> 一栏布局（2026-07-23 用户定，替代原竖向两行稿）：文件夹名（图标 + 名称）与文件名一栏；
> 文件名与 X 一组（无间距，X 内边距 4 兼作间隔；文件名右 4 与左 8 补齐一致）；X 常态不占位隐藏（宽度 0，空间留给文件名），悬停 item 时展开 24px 露出，文件名随之左移。

``` css
/* 总容器：基础 item + 文件名组 */
box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 4px 4px 4px 8px;
gap: 6px;

width: 填充父容器;
height: padding + 子项;

background: C4;

hover: background C5（详情面板 item 悬停，2026-07-23 用户定）

border: 1px solid border-1;
border-radius: 8px;

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 基础 item */
[[#01. 基础 item]];

/* 文件名组：文件名称 + 删除按钮，间距 4 */
display: flex;
flex-direction: row;
align-items: center;
/* 无间距：X 内边距兼作间隔，文件名右 4 */

/* 文件名称 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: font-icon-3;

/* 删除按钮 */
[[01_按钮#01. 图标按钮 - 无底色装饰]] / 小图标;

color: 容器内图标 font-icon-3;

常态: width 0 / opacity 0（不占位隐藏）;
hover（item 级）: width 24px / opacity 1（露出，文件名左移）;
hover（自身）: 图标 font-icon-2
```


## 07. 详情面板标签 item

``` css
/* 总容器：基础 item + 删除按钮 */
box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 4px 4px 4px 8px; /* 2026-07-23 用户定 */
gap: 6px;

width: padding + gap + 基础 item + 删除按钮;
height: padding + 子项;

background: C4;

hover: background C5（详情面板 item 悬停）

border: 1px solid border-1;
border-radius: 8px;

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 1;

/* 基础 item */
[[#01. 基础 item]];

order: 0;

/* 删除按钮 */
[[01_按钮#01. 图标按钮 - 无底色装饰]] / 小图标;

color: 容器内图标 font-icon-3;

hover: background C5（详情面板 item 悬停）

order: 1;
```

## 08. 颜色信息 item

``` css
/* 总容器：圆形色块（根据实际数量） */
box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 4px;
/* 无间距：X 内边距兼作间隔，文件名右 4 */

width: padding + gap +子项;
height: padding + 子项;

background: C4;

border: 1px solid border-1;
border-radius: 14px; /* 胶囊形 */

/* Inside auto layout */
flex: none;
flex-grow: 1;

/* 圆形色块*/
box-sizing: border-box;

width: 16px;
height: 16px;

background: 实际分析的颜色;

border: 1px solid border-1;

hover: 1px solid font-icon-2 / 色号悬停 tip;

/* Inside auto layout */
flex: none;
flex-grow: 0;
```

## 09. 评分 item

一共五个五角星图标，图标和快捷入口视图的图标一致，将尺寸缩小至 12 x 12。

``` css
/* 总容器：图标 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
/* 无间距：X 内边距兼作间隔，文件名右 4 */

width: gap + 子项;
height: 子项;

/* Inside auto layout */
flex: none;
flex-grow: 1;

/* 图标 */
width: 12px;
height: 12px;

color: border font-icon-3;

hover: border font-icon-2;

active: border 警告色 fill 警告色; /* 持久化选中 */

/* Inside auto layout */
flex: none;
flex-grow: 0;
```

## 10. 列表 Item

``` css
/* 总容器：标题容器 + 详情容器 */
box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;

width: 734px;
height: 57px;

background: C3 / C2; /* 两个底色交替出现，用于视觉能快速区分每一行 */

border: 1px solid border-1;
border-radius: 8px;

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;

/* 标题容器：缩略图 + 名称 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 8px;

margin: 0 auto;
width: 缩略图 + gap + 名称长度;
height: 子项缩略图高度;

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;

/* 缩略图 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: flex-start;
padding: 0px;

width: 80px; /* 可随滑块和滚轮缩放 */
height: 45px; /* 可随滑块和滚轮缩放 */

/* 不同尺寸的缩略图都被限定在容器里，不裁剪，等高不等宽，缩略图圆角边框线都和文件夹卡片一致 */

flex: none;
order: 0;
flex-grow: 0;

/* 名称 */
[[01_样式 token#02. 文字]] / 正文

color: font-icon-1;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;

/* 详情容器：尺寸 + 格式 + 文件大小 + 修改日期容器 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 16px;

margin: 0 auto;
width: 313px;
height: 20px;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;

/* 详情容器：尺寸 + 格式 + 文件大小 */
[[01_样式 token#02. 文字]] / 正文

color: font-icon-3;

/* 修改日期容器：日期 + 时间 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
/* 无间距：X 内边距兼作间隔，文件名右 4 */

width: 119px;
height: 18px;

/* Inside auto layout */
flex: none;
order: 3;
flex-grow: 0;

/* 日期 + 时间 */
[[01_样式 token#02. 文字]] / 正文

日期：2026/07/12 /* 年月日 */
时间：14:26 /* 小时 分钟 */

color: font-icon-3;
```

