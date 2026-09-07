## 053 CDP 探针桩返回可变 DB 集合必须深拷贝——共享引用绕过 Vue 响应式（双 item / 编辑不刷新假象）

2026-08-23，6.2 关系探针攻关六轮。症状链：add 成功后详情面板渲染**两条相同 item**（store 数组确有 2 条同 id 记录）+ 二次修改提交后 store 数据已新、DOM 恒旧。

根因 = 桩 `list_relations: () => DB.relations` **原样返回桩数组引用**：前端 store 数组与桩 DB 是同一对象——① 桩侧 add 的 `DB.relations.push` 直接写进前端 state（绕过 Vue 代理不 trigger），前端 confirm 再 push 一次 → 双 item；② 前端 `this.relations = DB.relations` 刷新赋值是同引用 → Vue `Object.is` 判定不 trigger → 编辑不刷新。**产品环境（真 IPC 序列化返回全新对象）两者都不成立**，纯探针 artifact。

修法：桩返回 `JSON.parse(JSON.stringify(...))` 斩断共享。**教训**：桩造可变集合（list 系命令）一律深拷贝；get 系返回字面量对象无此问题（每次新建）。**2026-08-24 同型再犯（6.2 链接管理探针）**：桩 `list_placements: () => DB.placements` 未深拷贝——sync_asset_links 桩真改 DB 后前端 refresh 拿到同引用数组不 trigger，视图恒旧数据；关系探针当时只深拷贝了 list_relations，**桩会写数据的集合都要深拷贝，不只 relations**（速查表 §12 桩纪律已补强）。同场再验既有坑：多文件改动探针前**批量 touch**（vite watcher 漏变更，探针拿旧模块白跑五轮）；v-for 内模板 ref = 数组收集（`ref.value[0]`，直调 `.select` 抛 TypeError）。
