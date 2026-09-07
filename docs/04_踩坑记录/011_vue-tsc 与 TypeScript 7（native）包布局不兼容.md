## 011 vue-tsc 与 TypeScript 7（native）包布局不兼容

- **现象：** 执行 `npx vue-tsc --noEmit` 做类型检查，报错 `ERR_PACKAGE_PATH_NOT_EXPORTED: Package subpath './lib/tsc' is not defined by "exports" in typescript/package.json`。
- **原因：** 项目 TypeScript 为 7.x（native 重写版），包内导出布局与 vue-tsc 3.x 期望的 TS 5 布局不符；且 vue-tsc 未列入项目依赖。
- **解法：** 不引入 vue-tsc；类型验证继续靠 `pnpm build`（rolldown 转译）+ 编辑器语言服务，与阶段 1/2 一致。
- **预防：** 新会话勿再试 `npx vue-tsc`；待 TS 7 正式版 / vue-tsc 跟进适配后再评估纳入（阶段 6 打磨时复查）。
