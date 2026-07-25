use tauri::{
  menu::{CheckMenuItem, Menu, MenuItem},
  tray::TrayIconBuilder,
  Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod adapters;
mod commands;
mod core;
mod services;

/// 托盘菜单项句柄（收件箱盘符勾选态 / 标注由 services::inbox::sync_tray 同步）
pub struct TrayItems {
  pub inbox: CheckMenuItem<tauri::Wry>,
}

/// 任务 2 spike：从窗口拖出文件到其他软件（WebView HTML5 拖拽出不了窗口边界，
/// 由原生层发起系统级拖拽，Windows 底层即 CF_HDROP）。
/// 注意：OLE 拖拽（DoDragDrop）必须在主线程发起，同步命令即运行于主线程。
#[tauri::command]
fn drag_files(window: tauri::WebviewWindow, paths: Vec<String>) -> Result<(), String> {
  let items: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
  drag::start_drag(
    &window,
    drag::DragItem::Files(items),
    drag::Image::Raw(Vec::new()),
    |_result, _cursor| {},
    drag::Options::default(),
  )
  .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // WebView2 色彩管理对齐（2026-07-23 偏色排查）：WebView2 与 Edge 在 BT2020 广色域屏上
  // 色彩表现不一致（WebView2 已知问题 WebViewFeedback #141/#4245）——Edge/Figma 偏紫、
  // 软件偏灰，暗部最明显。强制按 sRGB 渲染，保证设计稿（sRGB）在软件内所见即所得。
  // 须在 webview 创建前设置。
  #[cfg(windows)]
  std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--force-color-profile=srgb");

  tauri::Builder::default()
    // 单实例必须最先注册：二次启动时聚焦已有窗口
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
      }
    }))
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .manage(commands::library::AppState {
      library: std::sync::Mutex::new(None),
      imports: std::sync::Mutex::new(std::collections::HashMap::new()),
      inbox: std::sync::Mutex::new(None),
    })
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // 全局热键 alt+`：呼出 / 隐藏主窗口（后续用于快捷搜索框）
      // 注册失败（热键被他方占用）只记日志降级，不阻断启动
      if let Err(e) = app.global_shortcut().on_shortcut(
        Shortcut::new(Some(Modifiers::ALT), Code::Backquote),
        |app, _shortcut, event| {
          if event.state() != ShortcutState::Pressed {
            return;
          }
          if let Some(w) = app.get_webview_window("main") {
            let visible = w.is_visible().unwrap_or(false);
            let focused = w.is_focused().unwrap_or(false);
            if visible && focused {
              let _ = w.hide();
            } else {
              let _ = w.show();
              let _ = w.set_focus();
            }
          }
        },
      ) {
        log::error!("全局热键 alt+` 注册失败（可能被其他程序占用）: {e}");
      }

      // 托盘菜单：显示 / 收件箱盘符开关（3.2）/ 退出
      let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
      let inbox_item = CheckMenuItem::with_id(app, "inbox-drive", "收件箱盘符（未挂载）", true, false, None::<&str>)?;
      let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
      let menu = Menu::with_items(app, &[&show, &inbox_item, &quit])?;
      app.manage(TrayItems { inbox: inbox_item.clone() });
      let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().expect("no window icon").clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
          "show" => {
            if let Some(w) = app.get_webview_window("main") {
              let _ = w.show();
              let _ = w.set_focus();
            }
          }
          "inbox-drive" => services::inbox::toggle_drive(app),
          "quit" => app.exit(0),
          _ => {}
        })
        .on_tray_icon_event(|tray, event| {
          // 左键单击托盘图标 = 显示主窗口
          if let tauri::tray::TrayIconEvent::Click {
            button,
            button_state,
            ..
          } = event
          {
            if matches!(button, tauri::tray::MouseButton::Left)
              && matches!(button_state, tauri::tray::MouseButtonState::Up)
            {
              if let Some(w) = tray.app_handle().get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
              }
            }
          }
        })
        .build(app)?;

      // 关闭按钮 = 隐藏到托盘，不退出（快捷搜索框要求软件常驻后台）
      // 窗口为不透明 + 原生装饰（DWM 圆角/阴影），线框色用 DWMWA_BORDER_COLOR 指定
      if let Some(w) = app.get_webview_window("main") {
        #[cfg(target_os = "windows")]
        {
          use windows::Win32::Foundation::HWND;
          use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
          if let Ok(raw) = w.hwnd() {
            let hwnd = HWND(raw.0 as _);
            // 主窗口 DWM 1px 边框染为 border-1 #3A3940（COLORREF 0x00BBGGRR），
            // Win11 22000+ 生效，Win10 忽略
            let border: u32 = 0x0040_393A;
            unsafe {
              let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &border as *const _ as *const std::ffi::c_void,
                std::mem::size_of_val(&border) as u32,
              );
            }
          }
        }

        let w2 = w.clone();
        w.on_window_event(move |event| {
          if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = w2.hide();
          }
        });
      }

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      drag_files,
      commands::library::create_library,
      commands::library::open_library,
      commands::library::get_current_library,
      commands::library::list_recent_libraries,
      commands::import::classify_paths,
      commands::import::detect_sequences,
      commands::import::import_paths,
      commands::import::import_clipboard,
      commands::import::cancel_import,
      commands::import::resolve_conflicts,
      commands::import::list_folder_tree,
      commands::import::list_placements,
      commands::preview::get_asset_file_path,
      commands::preview::list_sequence_frames,
      commands::preview::set_thumb_frame,
      commands::preview::open_with_default_app,
      commands::preview::open_preview_window,
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    // 退出时收尾收件箱（3.2）：停监听 + 卸载盘符（PRD：启动时挂载、退出卸载）
    .run(|app, event| {
      if let tauri::RunEvent::Exit = event {
        services::inbox::stop(app);
      }
    });
}
