use tauri::{
  menu::{CheckMenuItem, Menu, MenuItem},
  tray::TrayIconBuilder,
  Emitter, Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod adapters;
mod commands;
mod core;
mod server;
mod services;

/// 托盘菜单项句柄（收件箱盘符勾选态 / 标注由 services::inbox::sync_tray 同步）
pub struct TrayItems {
  pub inbox: CheckMenuItem<tauri::Wry>,
}

/// 跨窗口拖出：从窗口拖文件到其他软件打开（WebView HTML5 拖拽出不了窗口边界，
/// 由原生层发起系统级拖拽，Windows 底层即 CF_HDROP——拖至任务栏图标走系统"打开方式"同链路）。
/// 入参 = 资产 id 集（前端在指针越出窗口边界的手势途中调用，需一次 IPC 直达 DoDragDrop，
/// 路径若先经前端往返解析，OLE 接管时机被拉长、松手即失拖出窗口期）；锁内只做 index 查表 +
/// real_origin_file 定位（序列帧 = frames/ 封面帧），不涉重 IO；不存在 / 解析失败的 id 静默跳过。
/// 注意：OLE 拖拽（DoDragDrop）必须在主线程发起，同步命令即运行于主线程（踩坑 004）。
#[tauri::command]
fn drag_assets(
  window: tauri::WebviewWindow,
  state: tauri::State<'_, commands::library::AppState>,
  asset_ids: Vec<String>,
) -> Result<(), String> {
  let paths = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    resolve_drag_paths(lib, &asset_ids)
  };
  start_files_drag(&window, paths)
}

/// 工作空间拖出（2026-10-01 ws ↔ Explorer 双向拖拽，PRD 02-13）：ws 文件卡片拖出窗口 =
/// drag_assets 同机制（指针越出窗口边界后原生层发起 CF_HDROP 系统级拖拽，宣告复制效果、
/// 源文件不动；落资源管理器 = 复制）。入参 = rel 清单（手势途中一次 IPC 直达，同 drag_assets
/// 口径）；逐条按 ws 校验转绝对路径（越界 / 不存在跳过 + 去重保序；**文件夹不拖出**——
/// 同库「文件夹不拖出」口径，拖出集只收文件）；空集报错。同步命令主线程 DoDragDrop（踩坑 004）
#[tauri::command]
fn ws_drag_paths(
  window: tauri::WebviewWindow,
  state: tauri::State<'_, commands::library::AppState>,
  rel_paths: Vec<String>,
) -> Result<(), String> {
  let mut seen = std::collections::HashSet::new();
  let paths: Vec<std::path::PathBuf> = rel_paths
    .iter()
    .filter_map(|rel| commands::workspace::validated_ws_rel(&state, rel).ok())
    .filter(|p| p.is_file())
    .filter(|p| seen.insert(p.clone()))
    .collect();
  start_files_drag(&window, paths)
}

/// 跨窗口拖出共用主体（drag_assets / ws_drag_paths）：空集报错 + 主线程 DoDragDrop
///（DragItem::Files / 空拖拽图 / Options::default = DragMode::Copy 只宣告 DROPEFFECT_COPY）
fn start_files_drag(window: &tauri::WebviewWindow, paths: Vec<std::path::PathBuf>) -> Result<(), String> {
  if paths.is_empty() {
    return Err("没有可拖出的文件".to_string());
  }
  drag::start_drag(
    window,
    drag::DragItem::Files(paths),
    drag::Image::Raw(Vec::new()),
    |_result, _cursor| {},
    drag::Options::default(),
  )
  .map_err(|e| e.to_string())
}

/// 拖出路径解析：id → info 内真实实体路径，去重保序，失败项跳过
fn resolve_drag_paths(lib: &core::library::Library, asset_ids: &[String]) -> Vec<std::path::PathBuf> {
  let mut seen = std::collections::HashSet::new();
  asset_ids
    .iter()
    .filter_map(|id| services::asset_menu::real_origin_file(lib, id).ok())
    .filter(|p| seen.insert(p.clone()))
    .collect()
}

/// 主窗口是否在最上层（2026-09-08 实机走查第 19 条）：判定 APP 是否显示在最上层，
/// 不是就呼出到最上层。旧用 is_focused 近似前台——窗口被其他 APP 遮挡在后但焦点未移
///（is_focused 仍 true）时旧判定会误走 hide 收进托盘。改用系统前台窗口比对 HWND。
/// 非 Windows / 取句柄失败回退 is_focused 近似。
#[cfg(windows)]
fn is_main_foreground(w: &tauri::WebviewWindow) -> bool {
  use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
  match w.hwnd() {
    Ok(raw) => (unsafe { GetForegroundWindow() }) == windows::Win32::Foundation::HWND(raw.0 as _),
    Err(_) => w.is_focused().unwrap_or(false),
  }
}

#[cfg(not(windows))]
fn is_main_foreground(w: &tauri::WebviewWindow) -> bool {
  w.is_focused().unwrap_or(false)
}

/// 显示主窗口统一出口（2026-09-02 走查修复：最小化状态 show() 不还原——SW_SHOW 对最小化
/// 窗口无效，须先 unminimize；热键 / 托盘 / 单实例共用）。emit_shown = 是否通知前端恢复播放
///（托盘隐藏态恢复需要——隐藏即停播；最小化未停播，单实例唤起不通知，保持各调用点原语义）
fn show_main_window(w: &tauri::WebviewWindow, emit_shown: bool) {
  if w.is_minimized().unwrap_or(false) {
    let _ = w.unminimize();
  }
  let _ = w.show();
  let _ = w.set_focus();
  if emit_shown {
    let _ = w.emit("main-window-shown", ());
  }
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
        show_main_window(&w, false);
      }
    }))
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .manage(commands::library::AppState {
      space: std::sync::Mutex::new(None),
      library: std::sync::Mutex::new(None),
      imports: std::sync::Mutex::new(std::collections::HashMap::new()),
      inbox: std::sync::Mutex::new(None),
      workspace: std::sync::Mutex::new(Vec::new()),
      undo: std::sync::Mutex::new(core::undo::UndoManager::default()),
      ws_searches: std::sync::Mutex::new(std::collections::HashMap::new()),
    })
    .setup(|app| {
      // 日志 release 同样启用（2026-09-29 兼容轮：朋友机透明子窗口创建失败零提示零日志，
      // 排障纯靠猜——文件日志 = 发版排障唯一线索）；文件在 app_log_dir
      //（Tauri v2 Windows = %LOCALAPPDATA%/com.anikit.dev/logs/anikit.log），超 5MB 轮转保留 2 份
      app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .targets([
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }),
          ])
          .max_file_size(5_000_000)
          .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(2))
          .build(),
      )?;

      // 预览解析队列（4.3 预览窗口）：单 worker 串行执行 psd 解码 / 帧条生成，
      // 提交新任务作废旧任务（快速切换素材不堆积），须在命令可达前初始化
      services::preview_queue::init(app.handle().clone());

      // 工作空间缩略图 worker 池（2026-09-26 性能优化）：固定并发 + 跨请求去重 + 失败记忆，
      // 须在命令可达前初始化（事件经 workspace-thumbs-ready 回填）
      services::ws_thumb_queue::init(app.handle().clone());

      // 动图帧流服务（阶段四·自研解码器）：WS server 推帧，预览窗口动图播放源
      // cfg(not(test))：init 内部持 AppHandle + 起 WS server，测试构建编译不出该函数（踩坑 014）
      #[cfg(not(test))]
      services::frame_stream::init(app.handle().clone());

      // 小动画生成队列（2.3 恢复）：单 worker 串行转码，卡片播放源 info/{stem}_anim.webp
      services::anim_preview_queue::init(app.handle().clone());

      // 插件通信 WS 服务（8.1，PRD 07-01）：Animate CEP 等软件插件的统一通信通道
      #[cfg(not(test))]
      server::init(app.handle().clone());

      // AI 外接通道 MCP 服务（一期只读，PRD 08-06）：Streamable HTTP，127.0.0.1 动态端口 + 端口文件
      #[cfg(not(test))]
      server::init_mcp(app.handle().clone());

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
            // 最上层判定（2026-09-08 走查：可见但被其他 APP 遮挡在后时一律呼出到最上层，
            // 不再看 is_focused——被遮挡未失焦的旧场景会误走 hide 收进托盘）
            let foreground = is_main_foreground(&w);
            // 最小化态一律走呼出（2026-09-02 走查修复：Win+D / 任务栏最小化不移焦点，
            // focused 可能仍 true → 旧 visible && focused 误命中把窗口 hide 进托盘，
            // 表现为"第一次按键先收进托盘"）
            let minimized = w.is_minimized().unwrap_or(false);
            if visible && foreground && !minimized {
              let _ = w.hide();
              // 窗口隐藏即通知前端暂停播放（2026-07-27 用户定：最小化保持播放，隐藏到托盘停止）
              let _ = w.emit("main-window-hidden", ());
            } else {
              // 重新显示即通知前端恢复悬停/自动播放（2026-08-23 用户实机报修：隐藏再开不恢复）
              show_main_window(&w, true);
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
              show_main_window(&w, true);
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
                show_main_window(&w, true);
              }
            }
          }
        })
        .build(app)?;

      // 关闭按钮 = 隐藏到托盘，不退出（快捷搜索框要求软件常驻后台）
      // 窗口底色 = 不透明实底 #1F1F1F（2026-10-01 终局转向：弃云母换回最大化过渡
      // 动画——撤 Mica + 撤整窗玻璃 + WS_CAPTION 全程保留，详见 adapters/window_fx.rs
      // 模块注释；前端基态实色单态，fx 双态机制 2026-10-02 已拆）
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

        #[cfg(target_os = "windows")]
        {
          use windows::Win32::Foundation::HWND;
          if let Ok(raw) = w.hwnd() {
            let hwnd = HWND(raw.0 as _);
            // 主窗口守卫（2026-08-30 失焦欺骗：骗 DWM 恒按激活渲染——失焦阴影不再收小；
            // 曾兼管「最大化摘 caption」杀 DWM 原生按钮，2026-10-01 随弃云母撤玻璃
            // 一并撤除——按钮画布 = 玻璃，玻璃没了 caption 全程保留，动画/无按钮兼得，
            // 详见 window_fx::main_window_guard 注释）
            adapters::window_fx::install_main_window_guard(hwnd);
            // 主窗口边框 = DWM 默认半透明独存，不显式染色（2026-10-02 用户定：
            // 「DWM 默认的边框不是半透明的么」——09-06 六轮口径回归；撤玻璃后 DWM
            // 默认边框正常画（本机 25H2 实证：「25H2 默认停画」只是玻璃 -1 场景特例，
            // 不可外推无玻璃窗口）。picking 进出由 set_main_border_picking 切染品牌色 ↔
            // 恢复 COLOR_NONE 默认。子窗口同口径放开（同日「子窗口也都放开，都统一」）。
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
      drag_assets,
      ws_drag_paths,
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
      commands::library::rename_library,
      commands::library::list_merge_candidates,
      commands::library::merge_libraries,
      commands::workspace::workspace_root,
      commands::workspace::list_workspace_dir,
      commands::workspace::ws_dir_stats,
      commands::workspace::ws_dirs_exist,
      commands::workspace::request_workspace_thumbs,
      commands::workspace::reveal_path,
      commands::workspace::open_path_with_default_app,
      commands::workspace::open_path_with_dialog,
      commands::workspace::ws_file_base64,
      commands::workspace::ws_shell_menu,
      commands::workspace::ws_shell_menu_background,
      commands::workspace::send_path_to_animate,
      commands::workspace::ws_create_folder,
      commands::workspace::ws_rename,
      commands::workspace::ws_delete,
      commands::workspace::ws_move,
      commands::workspace::ws_paste,
      commands::workspace::ws_duplicate,
      commands::workspace::ws_write_clipboard,
      commands::workspace::ws_clipboard_files,
      commands::workspace::ws_paste_system,
      commands::workspace::ws_import_paths,
      commands::workspace::ws_list_pinned,
      commands::workspace::ws_pin,
      commands::workspace::ws_unpin,
      commands::workspace::ws_add_link,
      commands::workspace::ws_remove_link,
      commands::workspace::ws_retarget_link,
      commands::workspace::ws_search,
      commands::workspace::ws_search_crawl_start,
      commands::workspace::ws_search_crawl_cancel,
      commands::workspace::everything_status,
      commands::workspace::launch_everything,
      commands::workspace::open_external_url,
      commands::workspace::ws_link_out,
      commands::workspace::ws_link_overview,
      commands::workspace::ws_remove_portal,
      commands::workspace::ws_reveal_external,
      commands::workspace::open_system_recycle_bin,
      commands::settings::get_ui_settings,
      commands::settings::save_ui_settings,
      commands::settings::save_window_state,
      commands::settings::exit_app,
      commands::settings::set_main_border_picking,
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
      commands::media::get_media_source_by_path,
      commands::media::get_audio_peaks,
      commands::media::get_audio_peaks_by_path,
      commands::font::get_font_info,
      commands::font::get_font_info_by_path,
      commands::font::install_font,
      commands::font::uninstall_font,
      commands::text::read_text_asset,
      commands::text::read_text_by_path,
      commands::text::write_text_asset,
      commands::docx::read_docx_asset,
      commands::docx::read_docx_by_path,
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
      commands::operations::set_folder_colors,
      commands::operations::set_folder_groups,
      commands::operations::assign_group,
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
      commands::prompt::analyze_asset_prompt,
      commands::prompt::assign_asset_prompt,
      commands::prompt::assign_archive_prompt,
      commands::prompt::clear_asset_prompt,
      commands::prompt::clear_archive_prompt,
      commands::prompt::refresh_asset_prompt_snapshot,
      commands::prompt::refresh_archive_prompt_snapshot,
      commands::prompt::new_asset_prompt_document,
      commands::prompt::new_archive_prompt_document,
      commands::prompt::delete_asset_prompt_document,
      commands::prompt::delete_archive_prompt_document,
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
      commands::asset_menu::copy_files_to_system_clipboard,
      commands::asset_menu::get_clipboard_seq,
      commands::asset_menu::set_thumb_asset,
      commands::asset_menu::restore_thumb_asset,
      commands::asset_menu::unbind_thumb_asset,
      commands::cep::get_cep_install_status,
      commands::cep::install_cep_plugin,
      commands::cep::uninstall_cep_plugin,
      commands::cep::get_cep_export_settings,
      commands::cep::save_cep_export_settings,
      commands::cep::send_to_animate,
      commands::plugin::open_plugin_window,
      commands::mcp::get_mcp_status,
      commands::mcp::set_mcp_server_enabled,
      commands::mcp::set_mcp_write_enabled,
      commands::plugin_server::get_plugin_server_status,
      commands::plugin_server::set_plugin_server_enabled,
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
    // 退出时收尾收件箱（3.2）：停监听 + 卸载盘符（PRD：启动时挂载、退出卸载）；
    // 同时收工作空间监听（PRD 02 步骤 1）
    .run(|app, event| {
      if let tauri::RunEvent::Exit = event {
        services::inbox::stop(app);
        services::workspace::stop_watch(app);
      }
    });
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::operations::finish;
  use std::collections::HashMap;

  fn add_file_asset(lib: &mut core::library::Library, id: &str, file: &str) {
    let src = lib.root.join(file);
    std::fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    lib.index.assets.insert(id.into(), meta);
    ops::add_placement(lib, id.into(), "root".into(), file.into(), true, &mut HashMap::new());
    finish(lib).unwrap();
  }

  #[test]
  fn resolve_drag_paths_resolves_dedups_and_skips_missing() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    add_file_asset(&mut lib, "AN-A", "a.png");
    add_file_asset(&mut lib, "AN-B", "b.psd");
    let paths = resolve_drag_paths(
      &lib,
      &["AN-A".into(), "AN-不存在".into(), "AN-B".into(), "AN-A".into()],
    );
    assert_eq!(paths.len(), 2, "不存在的跳过 + 重复 id 去重：{paths:?}");
    assert!(paths[0].ends_with("a.png") && paths[0].is_file(), "info 内真实实体");
    assert!(paths[1].ends_with("b.psd"));
    assert!(resolve_drag_paths(&lib, &["AN-不存在".into()]).is_empty(), "全失败 = 空");
  }
}
