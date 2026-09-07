## 010 `mod core` 遮蔽内建 core crate

- **现象：** 数据层新增 `mod core;` 后编译报错：`core::ffi::c_void` 解析失败（cannot find `ffi` in `core`）。
- **原因：** crate 内声明与内建 core crate 同名的本地模块后，`core::` 路径优先解析为本地模块，原有指向内建 crate 的路径全部失效。
- **解法：** 该处改用 `std::ffi::c_void`（std 路径不受遮蔽影响）。
- **预防：** 顶层模块避免与内建 crate（core / std / alloc）同名；已同名时（本项目目录结构定为 `src-tauri/src/core/`），crate 内一律走 `std::` / `crate::` 显式路径。
