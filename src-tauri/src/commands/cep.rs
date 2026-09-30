//! 命令 API：CEP 插件安装与导出设置（8.1 二轮 2b）——安装状态查询 / 一键安装 / 卸载 /
//! 导出参数读写；发送至 Animate（8.3，PRD 07-04 快速发送）。
//! 纯库外操作（文件复制 + 注册表 + 全局 config），不持库锁、不接撤销。

use crate::core::config::{self, CepExportSettings};
use crate::core::paths;
use crate::services::cep_install::{self, CepInstallStatus};
use crate::services::send_stage;

/// CEP 插件安装状态（设置页 / 菜单展示）
#[tauri::command]
pub async fn get_cep_install_status() -> Result<CepInstallStatus, String> {
  Ok(cep_install::status())
}

/// 一键安装 / 升级 CEP 插件（复制到 extensions + PlayerDebugMode 确保）
#[tauri::command]
pub async fn install_cep_plugin() -> Result<CepInstallStatus, String> {
  cep_install::install()
}

/// 卸载 CEP 插件（删除 extensions 下本插件目录；未安装 = 幂等成功），返回最新状态
#[tauri::command]
pub async fn uninstall_cep_plugin() -> Result<CepInstallStatus, String> {
  cep_install::uninstall()
}

/// 导出设置读取（settings.cepExport 子树；CEP 面板与 8.2 批量导出共用）
#[tauri::command]
pub async fn get_cep_export_settings() -> Result<CepExportSettings, String> {
  config::cep_export_settings().map_err(|e| e.to_string())
}

/// 导出设置保存（写回 + WS 广播推送在线插件实时生效）
#[tauri::command]
pub async fn save_cep_export_settings(settings: CepExportSettings) -> Result<(), String> {
  settings.validate()?;
  config::save_cep_export_settings(settings).map_err(|e| e.to_string())?;
  crate::server::broadcast_settings("animate");
  Ok(())
}

/// 发送至 Animate（8.3，PRD 07-04 快速发送）：fla → 剪贴板载荷（Flash 11 Picture）
/// 写入 → WS 下发粘贴（jsx clipPaste(true) 原位粘贴，坐标含在载荷内）；
/// 转换失败自动回退兜底（jsx 打开-复制-粘贴-关闭，慢但通用）。
/// 无队列（旧 electron 版积压痛点根治）：面板未连接立即报错；发送锁在途期间拒绝新发送。
#[tauri::command]
pub async fn send_to_animate(state: tauri::State<'_, crate::commands::library::AppState>, asset_id: String) -> Result<String, String> {
  // 短锁读资产元数据（慢 IO 与 WS 等待全程不持库锁）
  let (name, fla_path) = {
    let guard = state.library.lock().unwrap();
    let lib = guard.as_ref().ok_or("未打开资源库")?;
    let meta = lib.index.assets.get(&asset_id).ok_or("资产不存在")?;
    if meta.ext != "fla" {
      return Err("仅 fla 素材支持发送至 Animate".to_string());
    }
    (meta.origin_name.clone(), paths::origin_file(&lib.assets_dir, &meta.id, &meta.origin_name))
  };
  if !fla_path.is_file() {
    return Err(format!("实体文件不存在：{}", fla_path.display()));
  }
  send_fla(name, fla_path).await
}

/// 发送至 Animate 共享流程（库素材 send_to_animate 与工作空间按路径入口 send_path_to_animate
/// 共用，2026-09-29 右键菜单移植抽出）：剪贴板载荷转换（锁外线程池）→ 失败回退兜底 → WS 下发；
/// 发送锁在途期间拒绝新发送（无队列口径不变）
pub(crate) async fn send_fla(name: String, fla_path: std::path::PathBuf) -> Result<String, String> {
  let _permit = crate::server::try_acquire_send().ok_or("上一次发送尚未完成，请稍候")?;

  // 锁外转换（zip+XML 慢 IO 走线程池）；失败 = 自动回退兜底（转换原因记日志不打扰用户）
  let path2 = fla_path.clone();
  let built = tauri::async_runtime::spawn_blocking(move || send_stage::build_clip_payload(&path2))
    .await
    .map_err(|e| format!("转换任务失败：{e}"))?;
  let reply = match built {
    Ok(payload) => {
      let bytes = payload.len();
      tauri::async_runtime::spawn_blocking(move || crate::adapters::clipboard::write_flash_picture(&payload))
        .await
        .map_err(|e| format!("剪贴板任务失败：{e}"))??; // 剪贴板被占用等——兜底同款也要写剪贴板，直接报错让用户重试
      log::info!("发送至 AN：{}（剪贴板载荷 {} 字节）", fla_path.display(), bytes);
      crate::server::request_paste("paste", &name, None).await?
    }
    Err(e) => {
      log::warn!("剪贴板转换失败，回退打开-复制-粘贴：{}（{}）", e, fla_path.display());
      let p = fla_path.to_string_lossy().replace('\\', "/");
      crate::server::request_paste("pasteFla", &name, Some(&p)).await?
    }
  };
  if reply.get("ok").and_then(|x| x.as_bool()) == Some(true) {
    Ok(name)
  } else {
    Err(reply.get("message").and_then(|x| x.as_str()).unwrap_or("粘贴失败").to_string())
  }
}
