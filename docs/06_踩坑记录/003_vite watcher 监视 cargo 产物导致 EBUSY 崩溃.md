## 003 vite watcher 监视 cargo 产物导致 EBUSY 崩溃

- **现象：** `tauri dev` 启动后 vite 崩溃：`Error: EBUSY: resource busy or locked, watch '.../src-tauri/target/debug/deps/app_lib.dll'`。
- **原因：** vite 的文件监视默认覆盖项目根目录，cargo 构建时改写并锁定 `src-tauri/target/**` 下的 dll，watcher 报错后杀死了 beforeDevCommand。
- **解法：** vite.config.ts 增加 `server.watch.ignored = ["**/src-tauri/**"]`。
- **预防：** 同上，配置已固化；新增前端目录时注意 watcher 排除范围。
