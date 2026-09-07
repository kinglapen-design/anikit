![[Pasted image 20260807045652.png]]

## 1. 一级菜单

```
/* 悬停菜单 - 分组 item - 分组菜单 */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: column;
align-items: flex-start;
padding: 12px;
gap: 2px;

position: absolute;
width: 150px;
height: 267px;

/* 底色/C2 */
background: #161717;
/* 线条和交互/亮 - 选中 */
border: 1px solid rgba(230, 251, 232, 0.1412);
/* 投影 */
box-shadow: 0px 4px 40px rgba(0, 0, 0, 0.6);
border-radius: 8px;


/* 跳转 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 126px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;


/* 跳转 */

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


/* 新建分组 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 126px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 1;
align-self: stretch;
flex-grow: 0;


/* 新建分组 */

width: 56px;
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
order: 0;
flex-grow: 0;


/* 编辑分组 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 126px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 2;
align-self: stretch;
flex-grow: 0;


/* 编辑分组 */

width: 56px;
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
order: 0;
flex-grow: 0;


/* 展开/折叠 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: space-between;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 126px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 3;
align-self: stretch;
flex-grow: 0;


/* 展开/折叠 */

width: 62px;
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


/* 上移 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 126px;
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

width: 46px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 上移 */

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
order: 0;
flex-grow: 0;


/* 箭头 - 小 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: center;
align-items: center;
padding: 0px;

width: 12px;
height: 12px;

transform: rotate(-180deg);

/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* Vector */

width: 9px;
height: 9px;

/* 文字/标签 */
background: rgba(234, 255, 247, 0.3412);
transform: rotate(-180deg);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 下移 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 6px;

width: 126px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 5;
align-self: stretch;
flex-grow: 0;


/* 图标 + 标题 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 0px;
gap: 6px;

width: 28px;
height: 20px;


/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 下移 */

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
order: 0;
flex-grow: 0;


/* 箭头 - 小 */

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
order: 1;
flex-grow: 0;


/* Vector */

width: 9px;
height: 9px;

/* 文字/标签 */
background: rgba(234, 255, 247, 0.3412);

/* Inside auto layout */
flex: none;
order: 0;
flex-grow: 0;


/* 删除 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 126px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 6;
align-self: stretch;
flex-grow: 0;


/* 删除 */

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
order: 0;
flex-grow: 0;
```

## 2. 分组 item - 展开 / 折叠 - 二级菜单

```
/* 分组 item - 展开 / 折叠 - 二级菜单 */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: column;
align-items: flex-start;
padding: 12px;
gap: 2px;

position: absolute;
width: 180px;
height: 127px;

/* 底色/C2 */
background: #161717;
/* 线条和交互/亮 - 选中 */
border: 1px solid rgba(230, 251, 232, 0.1412);
/* 投影 */
box-shadow: 0px 4px 40px rgba(0, 0, 0, 0.6);
border-radius: 8px;


/* 当前分组 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 156px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 0;
align-self: stretch;
flex-grow: 0;


/* 当前分组 */

width: 56px;
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
order: 0;
flex-grow: 0;


/* 全部分组 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 156px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 1;
align-self: stretch;
flex-grow: 0;


/* 全部分组 */

width: 56px;
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
order: 0;
flex-grow: 0;


/* 排除当前分组展开全部 */

/* Auto layout */
display: flex;
flex-direction: row;
align-items: center;
padding: 6px 8px;
gap: 12px;

width: 156px;
height: 33px;

border-radius: 6px;

/* Inside auto layout */
flex: none;
order: 2;
align-self: stretch;
flex-grow: 0;


/* 排除当前分组展开全部 */

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
order: 0;
flex-grow: 0;
```

