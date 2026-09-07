## 031 serde 内部 tag 枚举：`rename_all` 只改变体名，不改字段名

- **现象：** 2026-08-13 扫描重复文件"当前目录"实机报错 `invalid args 'scope': missing field 'folder_id'`——前端发 `folderId`，后端却在等 `folder_id`。
- **原因：** `#[serde(tag = "kind", rename_all = "camelCase")]` 用在枚举上时，`rename_all` 只作用于**变体名**（kind 值），struct 变体的**字段名**不受控；`Folder { folder_id }` 保持蛇形。之前 `Assets` 变体靠字段级 `#[serde(rename = "assetIds")]` 补丁没暴露问题，新加的 `Folder` 字段漏补即炸。
- **解法：** 枚举上加 `rename_all_fields = "camelCase"`（serde ≥1.0.190 支持），撤掉字段级补丁；并补 `serde_json` 反序列化回归用例（命令参数的 camelCase 契约以后都有测试守门）。
- **预防：** 给 Tauri 命令写枚举参数时，凡带字段的 struct 变体一律 `rename_all` + `rename_all_fields` 成对写，并配一条 JSON 反序列化断言；不要靠字段级 rename 补丁（新变体必漏）。
