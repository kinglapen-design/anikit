# 踩坑 108：全局共享类与 scoped 同名——scoped 未声明的属性不免疫全局穿透（2026-10-09）

**错误信号**：preview 陈列里属性筛选弹窗 chips 横排流式正常，APP 实机却竖排一列一个；且二轮目验时完全正常，三轮后才坏——回归无感知。

## 根因

CSS 特异性是**逐属性**竞争的：三轮给 `bars.css` 列表族新立全局 `.fp-list { flex-direction: column }`（比例 / 日期弹窗用），与二轮 `FilterTagsPop.vue` scoped 的 `.fp-list`（chips 流式区）同名撞车。scoped 规则编译为 `.fp-list[data-v-x]`，特异性 (0,2,0) 高于全局 (0,1,0)——但只赢它**声明过的**属性（display / gap / padding）；`flex-direction` scoped 从未声明（想要默认值 row），全局的 column 于是在该属性上无竞争生效，chips 区变竖排。

## 正解

共享类语义化 + 组件变体改类名：`.fp-list` = 列表族（column）专用，属性弹窗 chips 区改名 `.fp-taglist` 并显式 `flex-direction: row`（2026-10-09 走查修复）。等效备选 = scoped 里显式声明 `flex-direction: row` 以更高特异性压回——但同名类留着一个想 row 一个想 column，迟早再咬人，改名断根。

## 预防

- 新立全局共享类（bars.css / items.css 等共享件）时，**先全库 grep 同名类存量**再定名；
- 加共享类后**复看既有同名组件的陈列与实机态**（本次三轮加类后只目验了新弹窗，漏看属性弹窗）；
- 组件 scoped 样式依赖「默认值」的属性（flex-direction / position / overflow 等），若同名全局类存在风险，显式声明出来。
