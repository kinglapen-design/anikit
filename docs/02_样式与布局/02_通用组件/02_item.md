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

color: border C4-40 fill C4-7;

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;

/* 标题 */
font: [[01_样式 token#02. 文字]] / 正文;

color: C4;

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

width: 子项; /* = 展开按钮图标 9px：item 组左 3 + 容器 9 + gap 2 = 14，与汉堡菜单 / 副标题按钮左缘对齐 */
height: 主容器高度; /* 增加鼠标判定范围，方便点选 */

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;

/* 展开按钮图标 */
width: 9px;
height: 9px;

background: C4-40;

hover: background C4-70;

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

hover: background C4-7;

active: background C4-15; /* 持久化选中 */

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 1;

/* 基础 item */
[[#01. 基础 item]];

order: 0;

active: 容器内图标 border C4 fill C4-7; /* 持久化选中 */

/* 数量标签 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: C4-40;

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

hover: background C4-7

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 基础 item */
[[#01. 基础 item]];

order: 0;

/* 快捷键标签 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: C4-40;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;

/* 二级菜单提示右箭头图标 */
width: 8px;
height: 8px;

background: C4-40;

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

border: 1px solid C4-15; /* 圆头 */

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;
```

## 05. 区块副标题 item

有的区块需要添加按钮，比如文件树需要添加文件夹等，但有的区块不需要添加按钮，不要需要添加按钮的直接删掉即可。

折叠按钮和添加都按钮用：chevron-small，add-small（16×16 显示），按钮为图标按钮-小 24×24（padding 4 / 圆角 4）；常态图标 C4-40，hover background C4-7 / 图标 C4-70。

``` css
/* 总容器：标题容器 + 添加按钮 */
/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 0px;
gap: 4px;

width: 填充父容器;
height: 子项;

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 标题容器：展开按钮 + 标题 */
/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 4px;

margin: 0 auto;
width: 按钮 + gap + 标题;
height: 子项;

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 1;

/* 展开按钮 */
[[01_按钮#01. 图标按钮 - 无底色装饰]] / 小图标;

color: 容器内图标 C4-40;

order: 0;

/* 标题 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: C4-40;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;

/* 添加按钮 */
[[01_按钮#01. 图标按钮 - 无底色装饰]] / 小图标;

color: 容器内图标 C4-40;

order: 1;
```

## 06. 详情面板关联文件夹 item

``` css
/* 总容器：文件夹标题容器 + 文件名称 */
box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: column;
align-items: flex-start;
padding: 6px 6px 6px 8px;
gap: 2px;

width: 填充父容器;
height: padding + gap + 子项;

background: C1-15;

hover: background C1-7

border: 1px solid C4-15;
border-radius: 8px;

/* Inside auto layout */
flex: none;
align-self: stretch;
flex-grow: 0;

/* 文件夹标题容器：基础 item + 删除按钮 */
/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 0px;

width: 填充父容器;
height: 子项;

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;

/* 基础 item */
[[#01. 基础 item]];

order: 0;

/* 删除按钮 */
[[01_按钮#01. 图标按钮 - 无底色装饰]] / 小图标;

color: 容器内图标 C4-40;

hover: background C4-7

order: 1;

/* 文件名称 */
font: [[01_样式 token#02. 文字]] / 副文本;

color: C4-40;

order: 1;
```

## 07. 详情面板标签 item

``` css
/* 总容器：基础 item + 删除按钮 */
box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 6px 6px 8px;
gap: 6px;

width: padding + gap + 基础 item + 删除按钮;
height: padding + 子项;

background: C1-15;

hover: background C1-7

border: 1px solid C4-15;
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

color: 容器内图标 C4-40;

hover: background C4-7

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
gap: 4px;

width: padding + gap +子项;
height: padding + 子项;

background: C1-15;

border: 1px solid C4-15;
border-radius: 14px; /* 胶囊形 */

/* Inside auto layout */
flex: none;
flex-grow: 1;

/* 圆形色块*/
box-sizing: border-box;

width: 16px;
height: 16px;

background: 实际分析的颜色;

border: 1px solid C4-15;

hover: 1px solid C4-70 / 色号悬停 tip;

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
gap: 4px;

width: gap + 子项;
height: 子项;

/* Inside auto layout */
flex: none;
flex-grow: 1;

/* 图标 */
width: 12px;
height: 12px;

color: border C4-40;

hover: border C4-70;

active: border 警告色 fill 警告色; /* 持久化选中 */

/* Inside auto layout */
flex: none;
flex-grow: 0;
```


