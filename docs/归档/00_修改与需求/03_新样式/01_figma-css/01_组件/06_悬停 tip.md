![[Pasted image 20260807045945.png]]

根据具体情况更改。

```
/* 悬停 tip */

box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
align-items: flex-start;
padding: 6px 8px;
gap: 12px;

position: absolute;
width: 111px;
height: 33px;

/* 底色/C2 */
background: #161717;
/* 线条和交互/亮 - 选中 */
border: 1px solid rgba(230, 251, 232, 0.1412);
/* 投影 */
box-shadow: 0px 4px 40px rgba(0, 0, 0, 0.6);
border-radius: 8px;


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
order: 0;
flex-grow: 0;


/* 快捷键 */

/* Auto layout */
display: flex;
flex-direction: row;
justify-content: flex-end;
align-items: center;
padding: 0px;
gap: 6px;

width: 55px;
height: 21px;


/* Inside auto layout */
flex: none;
order: 1;
flex-grow: 0;


/* ctrl */

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


/* N */

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
order: 1;
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