//! 导入查重候选复核：metadata.hash 可能早于 origin 当前内容，命中后先求真实哈希再分类。
//! 仅处理本批 incoming hash 命中的可见资产；文件 IO 在库锁外并行，回写仍先落盘再同步索引。

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::core::json_io::write_json;
use crate::core::library::Library;
use crate::services::import as ops;
use crate::services::operations;

use super::Unit;

struct Candidate {
  asset_id: String,
  origin_name: String,
  origin: PathBuf,
  expected_size: u64,
  expected_hash: String,
}

struct Rehashed {
  candidate: Candidate,
  size: u64,
  hash: String,
}

pub fn refresh_candidate_hashes(lib: &Mutex<Option<Library>>, units: &[(Unit, String)]) -> Result<(), String> {
  let candidates = {
    let guard = lib.lock().map_err(|_| "应用状态异常（锁冲突），请重试".to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "素材库已关闭，无法导入".to_string())?;
    snapshot_candidates(lib, units)
  };
  let rehashed: Vec<Rehashed> = candidates.into_par_iter().filter_map(rehash_candidate).collect();
  commit_rehashed(lib, rehashed)
}

fn snapshot_candidates(lib: &Library, units: &[(Unit, String)]) -> Vec<Candidate> {
  let incoming: HashSet<&str> = units.iter().map(|(_, hash)| hash.as_str()).collect();
  let alive: HashSet<&str> = lib.folders.items.iter().map(|p| p.asset.as_str()).collect();
  lib.index
    .assets
    .values()
    .filter(|meta| {
      meta.r#type.is_none()
        && meta.hidden != Some(true)
        && alive.contains(meta.id.as_str())
        && incoming.contains(meta.hash.as_str())
    })
    .map(|meta| Candidate {
      origin: lib.info_dir(&meta.id).join(&meta.origin_name),
      asset_id: meta.id.clone(),
      origin_name: meta.origin_name.clone(),
      expected_size: meta.size,
      expected_hash: meta.hash.clone(),
    })
    .collect()
}

fn rehash_candidate(candidate: Candidate) -> Option<Rehashed> {
  let result = fs::metadata(&candidate.origin)
    .map_err(|e| e.to_string())
    .and_then(|meta| ops::sha256_file(&candidate.origin).map(|hash| (meta.len(), hash)).map_err(|e| e.to_string()));
  match result {
    Ok((size, hash)) if size != candidate.expected_size || hash != candidate.expected_hash => {
      Some(Rehashed { candidate, size, hash })
    }
    Ok(_) => None,
    Err(e) => {
      log::warn!("导入查重复核读取 origin 失败（{}）：{e}", candidate.asset_id);
      None
    }
  }
}

fn commit_rehashed(lib: &Mutex<Option<Library>>, items: Vec<Rehashed>) -> Result<(), String> {
  if items.is_empty() {
    return Ok(());
  }
  let mut guard = lib.lock().map_err(|_| "应用状态异常（锁冲突），请重试".to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "素材库已关闭，无法导入".to_string())?;
  for item in items {
    let c = item.candidate;
    let Some(current) = lib.index.assets.get(&c.asset_id) else { continue };
    if current.origin_name != c.origin_name || current.size != c.expected_size || current.hash != c.expected_hash {
      log::info!("导入查重复核跳过并发变更资产（{}）", c.asset_id);
      continue;
    }
    let mut meta = current.clone();
    meta.size = item.size;
    meta.hash = item.hash;
    meta.modified_at = operations::now();
    let metadata_path = lib.info_dir(&c.asset_id).join("metadata.json");
    write_json(&metadata_path, &meta).map_err(|e| format!("库内文件哈希回写失败：{e}"))?;
    log::info!("导入查重复核修正哈希漂移（{}）", c.asset_id);
    lib.index.assets.insert(c.asset_id, meta);
  }
  Ok(())
}
