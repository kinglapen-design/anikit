## 028 `:hover:not()` 伪类计入特异性：悬停规则压过状态类，拖拽并入只见边框不见填充

- **现象：** 树 item 拖拽并入（drop-into）时 1.5px 品牌色边框正常，30% 品牌填充完全看不见，像没加一样；文件夹卡片同口径填充正常。
- **原因：** hover 规则 `.tree-item.child:hover:not(.selected)` 的特异性——`:hover` 与 `:not(.selected)` 都按类级计入（`:not()` 取其参数），合计 (0,4,0)，压过 `.tree-item.child.drop-into` 的 (0,3,0)；拖拽时鼠标正悬停在目标 item 上，hover 的暗-悬停罩层把并入的 30% 品牌罩层整个盖住。box-shadow 无 hover 规则竞争所以边框正常。
- **解法：** 三条 hover 规则补 `:not(.drop-into)`——状态类（选中 / 并入）存在时 hover 罩层退让（与既有 `:not(.selected)` 同思路）。
- **预防：** 写 `:hover:not(X)` 规则时，**把它当成比双类更高的特异性来源**——凡后续新增"悬停期间也要显示的状态类"（选中 / 拖拽 / 高亮），都要回头检查 hover 规则是否会把它的 background 压掉；状态类与 hover 共存时优先用 `:not(状态)` 排除法，而不是指望堆高状态类特异性。
