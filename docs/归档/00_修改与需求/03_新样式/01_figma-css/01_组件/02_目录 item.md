## 1. 动画效果

- 悬停：在 backgruound 上叠加 暗 - 悬停 颜色。
- 选中：在 backgruound 上叠加 亮 - 选中 颜色 / border 1.5px color 文字常态。

> 过度动画不要太快，让用户可感知，border 过渡看是否需要将常态设置为：border 1.5px color 0%。
> border 统一用内阴影模拟，避免线条变粗后布局抖动。

## 1. 根父级

``` css
/* 目录树 - 根父级 - item */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;

width: 218px;
height: 32px;

/* 底色/C5 */
background: #282A2A;
/* 线条和交互/亮 - 选中 */
border: 1px solid rgba(230, 251, 232, 0.1412);
border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 18;
align-self: stretch;
flex-grow: 0;


/* 图标 + 标题 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 70px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
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
order: 0;
flex-grow: 0;


/* svg */

width: 9px;
height: 5px;

/* 文字/标签 */
background: rgba(234, 255, 247, 0.3412);
transform: rotate(-90deg);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 文件夹 - 关闭 */

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
order: 1;
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


/* 名称 */

width: 28px;
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
order: 2;
flex-grow: 0;


/* 数量标签 */

width: 37px;
height: 20px;

/* 正文 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 14px;
line-height: 20px;
/* identical to box height, or 143% */
text-align: right;

/* 文字/标签 */
color: rgba(234, 255, 247, 0.3412);


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;
```

## 2. 其他层级

``` css
/* 目录树 - item */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;

width: 218px;
height: 32px;

/* 底色/C5-底色 */
background: #1F2020;
border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 19;
align-self: stretch;
flex-grow: 0;


/* 图标 + 标题 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 84px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
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
order: 0;
flex-grow: 0;


/* svg */

width: 9px;
height: 5px;

/* 文字/标签 */
background: rgba(234, 255, 247, 0.3412);
transform: rotate(-90deg);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 文件夹 - 关闭 */

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
order: 1;
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


/* 名称 */

width: 42px;
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
order: 2;
flex-grow: 0;


/* 数量标签 */

width: 25px;
height: 20px;

/* 正文 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 14px;
line-height: 20px;
/* identical to box height, or 143% */

/* 文字/标签 */
color: rgba(234, 255, 247, 0.3412);


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;
```

## 3. 档案

``` css
/* 目录树 - 档案 - item */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;

width: 218px;
height: 32px;

/* 底色/C5-底色 */
background: #1F2020;
border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 20;
align-self: stretch;
flex-grow: 0;


/* 图标 + 标题 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 70px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 档案提示圆点 */

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


/* svg */

width: 9px;
height: 9px;

/* 品牌色/常态 */
background: #0A8079;

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 档案 */

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
order: 1;
flex-grow: 0;


/* SVG */

width: 14px;
height: 15px;

/* 文字/常态 */
background: rgba(247, 255, 252, 0.6941);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 名称 */

width: 28px;
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
order: 2;
flex-grow: 0;


/* 数量标签 */

width: 37px;
height: 20px;

/* 正文 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 14px;
line-height: 20px;
/* identical to box height, or 143% */
text-align: right;

/* 文字/标签 */
color: rgba(234, 255, 247, 0.3412);


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;
```

## 4. 快速访问

``` css
/* 全部 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;

width: 218px;
height: 32px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;


/* 图标 + 标题 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 52px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 资源库 */

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
height: 13.85px;

/* 文字/常态 */
background: rgba(247, 255, 252, 0.6941);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 全部 */

width: 28px;
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


/* 数量标签 */

width: 37px;
height: 20px;

/* 正文 */
font-family: 'Microsoft YaHei UI';
font-style: normal;
font-weight: 400;
font-size: 14px;
line-height: 20px;
/* identical to box height, or 143% */

/* 文字/标签 */
color: rgba(234, 255, 247, 0.3412);


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;
```