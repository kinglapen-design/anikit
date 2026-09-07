## 1. 搜索框

清除按钮用删除图标。

``` css
/* 右键菜单输入框 */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 12px;
gap: 6px;

width: 320px;
height: 44px;

/* 线条和交互/暗 - 悬停 */
border-bottom: 1px solid rgba(221, 247, 223, 0.0784);

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;


/* 图标 + 文字 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 63px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 搜索 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 0px;

width: 18px;
height: 18px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* svg */

width: 15px;
height: 15px;

/* 文字/标签 */
background: rgba(234, 255, 247, 0.3412);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 占位符 */

width: 39px;
height: 20px;

/* 正文 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 14px;
line-height: 20px;
/* identical to box height, or 143% */

/* 文字/极淡 */
color: rgba(238, 255, 248, 0.2353);


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* 删除 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 4px;

width: 20px;
height: 20px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* svg */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 0px;

width: 12px;
height: 12px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* Vector */

width: 7px;
height: 7px;

/* 文字/标签 */
background: rgba(234, 255, 247, 0.3412);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;
```

## 2. 通用 item

单字母快捷键左右间距改为 6px，不然没有呼吸空间。

无快捷键时，隐藏快捷键。

``` css
/* 右键菜单 item */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 296px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 4;
align-self: stretch;
flex-grow: 0;


/* 图标 + 标题 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 94px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 图标 */

/* Auto layout */
display: flex;
flex-direction: column;
justify-content: center;
align-items: center;
padding: 0px;

width: 18px;
height: 18px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* svg */

width: 15px;
height: 13px;

/* 文字/常态 */
background: rgba(247, 255, 252, 0.6941);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 标题 */

width: 70px;
height: 20px;

/* 正文 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 14px;
line-height: 20px;
/* identical to box height, or 143% */

/* 文字/常态 */
color: rgba(247, 255, 252, 0.6941);


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* 快捷键 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: flex-end;
align-items: center;
padding: 0px;
gap: 6px;

width: 95px;
height: 21px;


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* 多字母词组 */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 2px 4px;

width: 27px;
height: 21px;

/* 底色/C3 */
background: #181919;
/* 线条和交互/暗 - 悬停 */
border: 1px solid rgba(221, 247, 223, 0.0784);
border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* ctrl */

width: 19px;
height: 17px;

/* 标签 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 12px;
line-height: 17px;
/* identical to box height, or 142% */

/* 文字/标签 */
color: rgba(234, 255, 247, 0.3412);


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 多字母词组 */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 2px 4px;

width: 34px;
height: 21px;

/* 底色/C3 */
background: #181919;
/* 线条和交互/暗 - 悬停 */
border: 1px solid rgba(221, 247, 223, 0.0784);
border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* shift */

width: 26px;
height: 17px;

/* 标签 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 12px;
line-height: 17px;
/* identical to box height, or 142% */

/* 文字/标签 */
color: rgba(234, 255, 247, 0.3412);


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 单字母 */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 2px 6px;

width: 22px;
height: 21px;

/* 底色/C3 */
background: #181919;
/* 线条和交互/暗 - 悬停 */
border: 1px solid rgba(221, 247, 223, 0.0784);
border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 2;
flex-grow: 0;


/* N */

width: 10px;
height: 17px;

/* 标签 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 12px;
line-height: 17px;
/* identical to box height, or 142% */

/* 文字/标签 */
color: rgba(234, 255, 247, 0.3412);


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;
```

## 3. item + 二级菜单提示

``` css
/* 右键菜单 item */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 296px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 12;
align-self: stretch;
flex-grow: 0;


/* 图标 + 标题 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 164px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 图标 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 0px;

width: 18px;
height: 18px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* svg */

width: 13px;
height: 15px;

/* 文字/常态 */
background: rgba(247, 255, 252, 0.6941);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 标题 */

width: 140px;
height: 20px;

/* 正文 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 14px;
line-height: 20px;
/* identical to box height, or 143% */

/* 文字/常态 */
color: rgba(247, 255, 252, 0.6941);


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* 展开 - 折叠 */

/* Auto layout */
display: flex;
flex-direction: column;
justify-content: center;
align-items: center;
padding: 0px;

width: 12px;
height: 12px;

transform: rotate(-90deg);

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* Vector */

width: 9px;
height: 5px;

/* 文字/标签 */
background: rgba(234, 255, 247, 0.3412);
transform: rotate(-90deg);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;
```