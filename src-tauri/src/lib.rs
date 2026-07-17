/// 任务 2 spike：从窗口拖出文件到其他软件（WebView HTML5 拖拽出不了窗口边界，
/// 由原生层发起系统级拖拽，Windows 底层即 CF_HDROP）。
#[tauri::command]
async fn drag_files(window: tauri::WebviewWindow, paths: Vec<String>) -> Result<(), String> {
  tauri::async_runtime::spawn_blocking(move || {
    let items: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    drag::start_drag(
      &window,
      drag::DragItem::Files(items),
      drag::Image::Raw(Vec::new()),
      |_result, _cursor| {},
      drag::Options::default(),
    )
    .map_err(|e| e.to_string())
  })
  .await
  .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![drag_files])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
