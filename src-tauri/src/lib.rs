use tauri::{
  menu::{CheckMenuItem, Menu, MenuItem},
  tray::TrayIconBuilder,
  Emitter, Manager,
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
      space: std::sync::Mutex::new(None),
      library: std::sync::Mutex::new(None),
      imports: std::sync::Mutex::new(std::collections::HashMap::new()),
      inbox: std::sync::Mutex::new(None),
      undo: std::sync::Mutex::new(core::undo::UndoManager::default()),
    })
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // 预览解析队列（4.3 预览窗口）：单 worker 串行执行 psd 解码 / 帧条生成，
      // 提交新任务作废旧任务（快速切换素材不堆积），须在命令可达前初始化
      services::preview_queue::init(app.handle().clone());

      // 动图帧流服务（阶段四·自研解码器）：WS server 推帧，动图卡播放源
      // cfg(not(test))：init 内部持 AppHandle + 起 WS server，测试构建编译不出该函数（踩坑 014）
      #[cfg(not(test))]
      services::frame_stream::init(app.handle().clone());

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
              // 窗口隐藏即通知前端暂停播放（2026-07-27 用户定：最小化保持播放，隐藏到托盘停止）
              let _ = w.emit("main-window-hidden", ());
            } else {
              let _ = w.show();
              let _ = w.set_focus();
              // 重新显示即通知前端恢复悬停/自动播放（2026-08-23 用户实机报修：隐藏再开不恢复）
              let _ = w.emit("main-window-shown", ());
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
              let _ = w.emit("main-window-shown", ());
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
                let _ = w.emit("main-window-shown", ());
              }
            }
          }
        })
        .build(app)?;

      // 关闭按钮 = 隐藏到托盘，不退出（快捷搜索框要求软件常驻后台）
      // 窗口为 transparent + 运行时按系统版本接云母（7.1 十七轮：Win11 22H2+ Mica，
      // 其余实色回退 --n-g-a1-solid，前端 get_window_fx_mode 查询后切 .window 底色）
      if let Some(w) = app.get_webview_window("main") {
        // 主窗口几何恢复（2026-07-28 用户定：窗口大小位置持久化；tauri.conf visible:false
        // 先隐后显防跳变，预览子窗口不记录不恢复）：显示器拔除致坐标落空时只恢复尺寸不恢复位置
        if let Some(s) = core::config::window_state() {
          let on_monitor = app
            .available_monitors()
            .map(|ms| {
              ms.iter().any(|m| {
                let (p, z) = (m.position(), m.size());
                s.x + 32 > p.x && s.y + 32 > p.y && s.x < p.x + z.width as i32 && s.y < p.y + z.height as i32
              })
            })
            .unwrap_or(false);
          // 最小恢复尺寸 800×500（2026-07-29 复验修复：曾恢复出 422×313 小窗——
          // 原 400×300 下限形同虚设；写回侧已加最小化 / 小尺寸守卫，此处兜底）
          let _ = w.set_size(tauri::PhysicalSize::new(s.width.max(800), s.height.max(500)));
          if on_monitor {
            let _ = w.set_position(tauri::PhysicalPosition::new(s.x, s.y));
          }
          if s.maximized {
            let _ = w.maximize();
          }
        }
        let _ = w.show();
        let _ = w.set_focus();
        // 云母接入须在 show 后（7.1 十七轮实机实证：show 前 set_effects 会把 WebView2
        // 内容吞掉只剩云母渐变——热切换不吞；透明窗口初始化期的 DWM 背景材料与 webview
        // 合成冲突，窗口显示后再设则两者兼容）
        adapters::window_fx::apply_main(&w);

        #[cfg(target_os = "windows")]
        {
          use windows::Win32::Foundation::HWND;
          use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
          if let Ok(raw) = w.hwnd() {
            let hwnd = HWND(raw.0 as _);
            // 主窗口 DWM 1px 边框染为 亮-选中（token：主窗口边框档）叠 C1 实色 #2E332F
            //（COLORREF 0x00BBGGRR；2026-08-08 新 token 体系换色，旧 border-1 #3A3940 作废），
            // Win11 22000+ 生效，Win10 忽略
            let border: u32 = 0x002F_332E;
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
            // 关闭到托盘即通知前端暂停播放（2026-07-27 用户定）
            let _ = w2.emit("main-window-hidden", ());
          }
        });
      }

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      drag_files,
      commands::library::create_space,
      commands::library::open_space,
      commands::library::get_current_space,
      commands::library::list_space_libraries,
      commands::library::create_library,
      commands::library::open_library,
      commands::library::get_current_library,
      commands::library::list_recent_libraries,
      commands::library::set_auto_import,
      commands::library::reorder_libraries,
      commands::library::delete_library,
      commands::library::count_library_orphans,
      commands::settings::get_ui_settings,
      commands::settings::save_ui_settings,
      commands::settings::save_window_state,
      commands::settings::exit_app,
      commands::settings::get_window_fx_mode,
      commands::import::classify_paths,
      commands::import::detect_sequences,
      commands::import::import_paths,
      commands::import::import_clipboard,
      commands::import::cancel_import,
      commands::import::resolve_conflicts,
      commands::import::list_folder_tree,
      commands::import::list_placements,
      commands::inbox::list_inbox_snapshot,
      commands::inbox::import_inbox_entry,
      commands::inbox::open_inbox_entry,
      commands::inbox::rename_inbox_entry,
      commands::inbox::delete_inbox_entry,
      commands::preview::get_asset_file_path,
      commands::preview::list_sequence_frames,
      commands::preview::prepare_anim_strips,
      commands::sequence::dissolve_sequence,
      commands::sequence::delete_sequence_frames,
      commands::sequence::renumber_sequence_frames,
      commands::anim_stream::start_anim_stream,
      commands::anim_stream::stop_anim_stream,
      commands::anim_stream::set_anim_stream_size,
      commands::anim_stream::set_anim_stream_playing,
      commands::anim_stream::set_anim_stream_rate,
      commands::anim_stream::seek_anim_stream,
      commands::thumb_frame::set_thumb_frame,
      commands::thumb_frame::set_video_thumb,
      commands::media::get_media_source,
      commands::media::get_waveform,
      commands::font::get_font_info,
      commands::font::install_font,
      commands::font::uninstall_font,
      commands::text::read_text_asset,
      commands::text::write_text_asset,
      commands::notes::get_notes,
      commands::notes::set_note,
      commands::notes::get_note_thumb,
      commands::undo::undo,
      commands::undo::redo,
      commands::undo::get_undo_state,
      commands::operations::create_folder,
      commands::operations::create_document,
      commands::operations::rename_folder,
      commands::operations::set_folder_icon,
      commands::operations::set_folder_color,
      commands::operations::set_folder_groups,
      commands::operations::move_placements,
      commands::operations::reorder_placements,
      commands::operations::reorder_folders,
      commands::operations::move_folders,
      commands::operations::apply_block_changes,
      commands::operations::reorder_blocks,
      commands::operations::link_placements,
      commands::operations::sync_asset_links,
      commands::operations::set_placement_origin,
      commands::operations::duplicate_placements,
      commands::operations::clone_folders,
      commands::operations::rename_placements,
      commands::operations::rename_placement,
      commands::operations::delete_placements,
      commands::archive::create_archive,
      commands::archive::convert_to_archive,
      commands::archive::convert_to_folder,
      commands::archive::list_archives,
      commands::archive::get_archive_data,
      commands::archive::get_asset_detail,
      commands::archive::set_archive_props,
      commands::archive::set_archive_descriptions,
      commands::archive::set_archive_cover_settings,
      commands::custom_data::set_asset_props,
      commands::custom_data::set_asset_descriptions,
      commands::custom_data::list_prop_pool,
      commands::custom_data::delete_props_globally,
      commands::relations::list_relations,
      commands::relations::add_relations,
      commands::relations::update_relation,
      commands::relations::remove_relation,
      commands::recent::list_recent_assets,
      commands::recycle::list_recycle,
      commands::recycle::restore_recycle,
      commands::recycle::delete_recycle,
      commands::recycle::clear_recycle,
      commands::scan::scan_duplicates,
      commands::search::search_assets,
      commands::preview::open_with_default_app,
      commands::preview::open_preview_window,
      commands::asset_menu::open_with_dialog,
      commands::asset_menu::reveal_in_explorer,
      commands::asset_menu::set_thumbnail_from_clipboard,
      commands::asset_menu::set_thumbnail_custom,
      commands::asset_menu::refresh_thumbnail,
      commands::asset_menu::copy_thumbnail_to_clipboard,
      commands::asset_menu::export_assets,
      commands::asset_menu::export_folder,
      commands::asset_menu::export_retry,
      commands::asset_menu::cancel_export,
      commands::asset_menu::delete_assets_permanently,
      commands::asset_menu::delete_folder,
      commands::asset_menu::delete_folders,
      commands::asset_menu::get_asset_base64,
      commands::asset_menu::get_asset_origin_path,
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
