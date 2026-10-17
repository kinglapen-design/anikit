mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_markdown,
            commands::read_markdown,
            commands::write_markdown,
            commands::create_markdown,
            commands::template_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PromptMD");
}
