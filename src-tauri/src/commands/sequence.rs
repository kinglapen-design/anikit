//! 命令 API：序列帧包操作（5.2，PRD 04-序列帧 / 05-序列帧打包）——解散导入 / 删除帧 / 重排序号。
//! 删除帧与重排序号走撤销三步接入（锁序 library → undo，与 commands::operations 同约）；
//! 解散导入 = 危险操作**不进 undo**（警告弹窗明示不可撤销，与回收站彻底清除同级）——
//! 每帧移动出库为独立素材、原包 info 物理删除，磁盘变动无快照可回。
//! 全部 async：涉及磁盘 IO 一律走线程池（规范第 2 条）。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library;
use crate::core::notes::NOTES_FILE;
use crate::core::paths;
use crate::core::undo::DiskOp;
use crate::services::operations as ops;

/// 解散序列帧包导入：每帧移动出库建档为独立素材（落点 = 原包 origin 所在文件夹），
/// 原包整体删除（逐帧笔记随之删除）。返回解散出的素材数。
/// 不可撤销：不进 undo；失败时服务层已回滚磁盘，此处内存从盘重载对齐（盘上是事实来源）。
#[tauri::command]
pub async fn dissolve_sequence(state: State<'_, AppState>, asset_id: String) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  match ops::dissolve_sequence(lib, &asset_id) {
    Ok(out) => Ok(out.new_ids.len()),
    Err(e) => {
      if let Err(re) = library::reload(lib) {
        log::error!("解散失败后内存重载失败：{re}");
      }
      Err(e)
    }
  }
}

/// 删除序列帧包的帧（抽帧调节奏）：帧移 cache/seq-trash 暂存（会话结束真删），
/// 可撤销（磁盘改名对登记 DiskOp::Rename 双向——暂存区与 frames/ 互斥，恒不撞名）；
/// 封面帧被删改指剩余首帧并重建缩略图；帧笔记同步清除。至少保留 1 帧。返回实际删除数。
#[tauri::command]
pub async fn delete_sequence_frames(
  state: State<'_, AppState>,
  asset_id: String,
  frame_names: Vec<String>,
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  let info_dir = lib.info_dir(&asset_id);
  undo.begin(format!("删除 {} 张序列帧", frame_names.len()));
  undo.record_file(&paths::metadata_file(&lib.assets_dir, &asset_id)).map_err(|e| e.to_string())?;
  undo.record_file(&info_dir.join(NOTES_FILE)).map_err(|e| e.to_string())?;
  match ops::delete_frames(lib, &asset_id, &frame_names) {
    Ok(out) => {
      if out.moves.is_empty() {
        undo.cancel(); // 全陈旧名无实际操作，不留空事务
        return Ok(0);
      }
      for (from, to) in &out.moves {
        undo.record_disk_op(
          DiskOp::Rename { from: to.clone(), to: from.clone() }, // undo：暂存移回 frames/
          DiskOp::Rename { from: from.clone(), to: to.clone() }, // redo：再移走
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(out.deleted)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 重排序号：剩余帧按当前文件名顺序重排为连续序号（词干_001..N，位宽只增不减）；
/// 帧笔记 key 随映射迁移；可撤销（两段改名的磁盘操作序列由服务层按 apply_disk 逆序语义折算）。
/// 返回实际改名数（0 = 序号已连续，无操作）。
#[tauri::command]
pub async fn renumber_sequence_frames(state: State<'_, AppState>, asset_id: String) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  let info_dir = lib.info_dir(&asset_id);
  undo.begin("重排序号");
  undo.record_file(&paths::metadata_file(&lib.assets_dir, &asset_id)).map_err(|e| e.to_string())?;
  undo.record_file(&info_dir.join(NOTES_FILE)).map_err(|e| e.to_string())?;
  match ops::renumber_frames(lib, &asset_id) {
    Ok(out) => {
      if out.renamed == 0 {
        undo.cancel(); // 已连续无操作，不留空事务
        return Ok(0);
      }
      for ((uf, ut), (rf, rt)) in out.undo_ops.iter().zip(out.redo_ops.iter()) {
        undo.record_disk_op(
          DiskOp::Rename { from: uf.clone(), to: ut.clone() },
          DiskOp::Rename { from: rf.clone(), to: rt.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(out.renamed)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}
