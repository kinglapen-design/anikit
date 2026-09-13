## 076 手工 cargo build --release 打出的包找 devUrl——免安装打包必须走 tauri build 链（2026-09-13）

- **现象：** 免安装包（release/anikit-portable）在实机启动后窗口白屏报「无法访问此页面 / 127.0.0.1 拒绝连接 / ERR_CONNECTION_REFUSED」——webview 去连 vite dev 服务器（tauri.conf.json 的 `build.devUrl` = http://127.0.0.1:5373），而不是加载内置前端。
- **原因：** 打包时图快用了 `cd src-tauri && cargo build --release`——tauri 的 dev/prod 前端路由不是看 cargo profile，而是由 tauri-cli 构建链决定：`tauri dev` 把 devUrl 烧进二进制，`tauri build` 才把 `frontendDist`（dist/）嵌入。绕开 tauri-cli 直接 cargo build，codegen 按默认（dev）处理 → 产物找 devUrl。体积也是信号：错版 21.5MB（无前端），正版 26MB（嵌入 dist）。
- **解法：** 免安装打包一律 `pnpm tauri build --no-bundle`（跳过 NSIS/MSI 安装包，只要 target/release/app.exe），再按既定结构手工组装：app.exe → anikit.exe + target/release/ffmpeg.exe·ffprobe.exe + vendor/ffmpeg/bin/av*.dll + plugins/animate-cep → zip。
- **预防：** Tauri 项目的可分发产物**永远从 tauri-cli 构建链产出**（tauri build / tauri android build 等），plain cargo build 只服务本地测试。拿到包先冒烟启一次再交付——"能编过"不等于"前端嵌进去了"，exe 体积突变（±前端体量）是一眼可查的粗检。
