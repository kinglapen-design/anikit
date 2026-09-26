//! 提示词业务（2026-09-26 提示词轮，PRD 03_数据链接 02_文件「提示词」）：
//! 提示词 = 对库内 md/txt 文本资产的「引用 + 内容快照」（PromptBinding，core/models.rs）——
//! 平时显示快照（源文档被彻底删除不断链：引用置 None、快照保留降级为已扫描态），可跳转源文件。
//! 三态：未指定（无 prompt 字段）/ 已指定（asset_id = Some）/ 已扫描（asset_id = None，
//! 纯快照无引用——仅资产侧「分析提示词」产出；档案字段同构但无已扫描产出路径）。
//!
//! 写点纪律：资产侧落 metadata.json（custom_data::write_asset_meta 单写点——落盘 + 索引同步 +
//! 刷 modified_at）；档案侧落 archives.json（core::library::save_archives 单写点 +
//! custom_data::archive_record_mut 懒建记录）。「删除提示词文档」不在本层单写——
//! 由命令层组合（delete_placements + clear，单撤销事务）。

use std::path::{Path, PathBuf};

use crate::core::library::{self, Library};
use crate::core::models::{AssetMeta, PromptBinding, ARCHIVE_KIND};
use crate::services::custom_data;
use crate::services::operations;
use crate::services::prompt_scan;
use crate::services::text;
use crate::services::views::PlacementView;

/// 已扫描态显示名（纯快照无源文档可引用）
pub const SCANNED_NAME: &str = "扫描结果";

/// 源文档显示名快照：首条活 Placement 名，无则 originName
///（commands/archive.rs thumb_display_name 同口径）
fn source_display_name(lib: &Library, asset_id: &str, origin_name: &str) -> String {
  lib
    .index
    .placements_of(asset_id)
    .first()
    .map(|p| p.name.clone())
    .unwrap_or_else(|| origin_name.to_string())
}

/// 读源文本资产：（存在性 + 文本门禁 + 读 origin 内容）→（显示名快照, 内容）。
/// 文本判定：is_text 吃 Path 按扩展名判定——按 ext 口径构造 Path::new(&origin_name)
fn read_prompt_source(lib: &Library, prompt_asset_id: &str) -> Result<(String, String), String> {
  let src = lib.index.assets.get(prompt_asset_id).ok_or_else(|| "提示词文档不存在，可能已被删除".to_string())?;
  if !text::is_text(Path::new(&src.origin_name)) {
    return Err(format!("「{}」不是文本资源", src.name));
  }
  let origin = lib.info_dir(prompt_asset_id).join(&src.origin_name);
  let content = text::read_text(&origin)?;
  Ok((source_display_name(lib, prompt_asset_id, &src.origin_name), content))
}

/// 绑定是否与「指定源 + 当下源内容」一致（无改动判定：updated_at 不参与比较——它是快照时刻）
fn binding_matches(cur: Option<&PromptBinding>, src_id: &str, name: &str, content: &str) -> bool {
  cur.is_some_and(|p| p.asset_id.as_deref() == Some(src_id) && p.name == name && p.content == content)
}

/// 指定提示词（资产侧）：校验 prompt 资产存在且为文本、禁自指 → 读源内容 → 引用 + 快照落盘。
/// 返回是否有改动（同源同内容 = 无改动未落盘，命令层据此取消撤销事务）
pub fn assign_asset(lib: &mut Library, asset_id: &str, prompt_asset_id: &str) -> Result<bool, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  if asset_id == prompt_asset_id {
    return Err("不能把提示词指定为资产自身".to_string());
  }
  let (name, content) = read_prompt_source(lib, prompt_asset_id)?;
  if binding_matches(meta.prompt.as_ref(), prompt_asset_id, &name, &content) {
    return Ok(false);
  }
  let binding =
    PromptBinding { asset_id: Some(prompt_asset_id.to_string()), name, content, updated_at: operations::now() };
  custom_data::write_asset_meta(lib, meta, |m| m.prompt = Some(binding))?;
  Ok(true)
}

/// 指定提示词（档案侧，与资产侧同口径；archive_record_mut 前置含「目标须为档案」校验 + 懒建记录）
pub fn assign_archive(lib: &mut Library, folder_id: &str, prompt_asset_id: &str) -> Result<bool, String> {
  let (name, content) = read_prompt_source(lib, prompt_asset_id)?;
  let rec = custom_data::archive_record_mut(lib, folder_id)?;
  if binding_matches(rec.prompt.as_ref(), prompt_asset_id, &name, &content) {
    return Ok(false);
  }
  rec.prompt = Some(PromptBinding { asset_id: Some(prompt_asset_id.to_string()), name, content, updated_at: operations::now() });
  library::save_archives(lib).map_err(|e| e.to_string())?;
  Ok(true)
}

/// 清除提示词（资产侧）：菜单「断开提示词文档链接」与「清空提示词」共用——整绑定移除
///（快照随解绑清除，无「断链留快照」半态；降级快照仅由源删除钩子产生，见 recycle::sync_after_purge）
pub fn clear_asset(lib: &mut Library, asset_id: &str) -> Result<bool, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  if meta.prompt.is_none() {
    return Ok(false);
  }
  custom_data::write_asset_meta(lib, meta, |m| m.prompt = None)?;
  Ok(true)
}

/// 清除提示词（档案侧，同资产侧口径）
pub fn clear_archive(lib: &mut Library, folder_id: &str) -> Result<bool, String> {
  let rec = custom_data::archive_record_mut(lib, folder_id)?;
  if rec.prompt.is_none() {
    return Ok(false);
  }
  rec.prompt = None;
  library::save_archives(lib).map_err(|e| e.to_string())?;
  Ok(true)
}

/// 同步快照（资产侧，详情「保存提示词」钮 = 同步快照语义）：已指定态重读源内容更新
/// content + updated_at；未指定 / 已扫描报错。内容未变 = false 不落盘
pub fn refresh_asset_snapshot(lib: &mut Library, asset_id: &str) -> Result<bool, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  let src_id = match meta.prompt.as_ref().map(|p| p.asset_id.clone()) {
    Some(Some(id)) => id,
    Some(None) => return Err("扫描结果无源文档，不能同步快照".to_string()),
    None => return Err("未指定提示词".to_string()),
  };
  let (_name, content) = read_prompt_source(lib, &src_id)?;
  if meta.prompt.as_ref().map(|p| p.content.as_str()) == Some(content.as_str()) {
    return Ok(false);
  }
  let now = operations::now();
  custom_data::write_asset_meta(lib, meta, |m| {
    if let Some(p) = m.prompt.as_mut() {
      p.content = content;
      p.updated_at = now;
    }
  })?;
  Ok(true)
}

/// 同步快照（档案侧，同资产侧口径）
pub fn refresh_archive_snapshot(lib: &mut Library, folder_id: &str) -> Result<bool, String> {
  let src_id = {
    let rec = custom_data::archive_record_mut(lib, folder_id)?;
    match rec.prompt.as_ref().map(|p| p.asset_id.clone()) {
      Some(Some(id)) => id,
      Some(None) => return Err("扫描结果无源文档，不能同步快照".to_string()),
      None => return Err("未指定提示词".to_string()),
    }
  };
  let (_name, content) = read_prompt_source(lib, &src_id)?;
  let now = operations::now();
  let rec = custom_data::archive_record_mut(lib, folder_id)?;
  let cur = rec.prompt.as_mut().expect("上方已校验有绑定");
  if cur.content == content {
    return Ok(false);
  }
  cur.content = content;
  cur.updated_at = now;
  library::save_archives(lib).map_err(|e| e.to_string())?;
  Ok(true)
}

/// 分析提示词（仅资产侧，档案无「分析」产出路径）：prompt_scan 解析图片内嵌元数据
///（本地解析无 AI）→ 已扫描态绑定（asset_id = None 纯快照，name = "扫描结果"）落盘并返回；
/// 覆盖既有绑定（重新分析 = 新快照）
pub fn analyze_asset(lib: &mut Library, asset_id: &str) -> Result<PromptBinding, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  let origin = lib.info_dir(asset_id).join(&meta.origin_name);
  let content = prompt_scan::scan_prompt(&origin, &meta.ext)?;
  let binding = PromptBinding { asset_id: None, name: SCANNED_NAME.to_string(), content, updated_at: operations::now() };
  custom_data::write_asset_meta(lib, meta, |m| m.prompt = Some(binding.clone()))?;
  Ok(binding)
}

/// 新建提示词文档（PRD 03_数据链接 02_文件「创建为 MD 文本」）：复用 operations::create_document
///（指定词干 + 撞名 _序号 同既有口径）；content 非空 = 写内容（text::write_origin：
/// 覆写 origin 恒 UTF-8 + metadata.size/modifiedAt/hash 同步——2026-09-26 自 commands/text.rs
/// 下迁 services 的实现，本层不可反向依赖 commands）。
/// 返回三元组同 create_document（view, info 目录, 撤销备份路径）供命令层 record_disk_op
pub fn new_prompt_document(
  lib: &mut Library,
  folder_id: Option<&str>,
  kind: &str,
  doc_name: &str,
  content: &str,
) -> Result<(PlacementView, PathBuf, PathBuf), String> {
  let (view, info_dir, backup) = operations::create_document(lib, kind, folder_id, Some(doc_name))?;
  if !content.is_empty() {
    text::write_origin(lib, &view.asset_id, content)?;
  }
  Ok((view, info_dir, backup))
}

/// 新建提示词文档的名称 / 内容种子（命令层「新建提示词文档」流程用；菜单「新建 .md/.txt
/// 提示词文档」未指定态与「保存提示词为 .md/.txt 文档」已扫描态共用）：
/// 已扫描态 = 快照内容；未指定态 = 空内容；名恒 = 资产显示名词干。已指定态报错（该态菜单不出此入口）
pub fn doc_seed(lib: &Library, asset_id: &str) -> Result<(String, String), String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?;
  let content = match &meta.prompt {
    None => String::new(),
    Some(p) if p.asset_id.is_none() => p.content.clone(),
    Some(_) => return Err("已指定提示词文档，不能重复新建".to_string()),
  };
  Ok((display_stem(lib, meta), content))
}

/// 档案提示词文档名种子（命令层用）：名 = 档案名，内容恒空（档案无已扫描态）；已指定报错同 doc_seed
pub fn archive_doc_seed(lib: &Library, folder_id: &str) -> Result<String, String> {
  let f = lib.index.folders.get(folder_id).ok_or_else(|| "目标档案不存在，可能已被删除".to_string())?;
  if f.kind.as_deref() != Some(ARCHIVE_KIND) {
    return Err("目标不是档案".to_string());
  }
  let bound = lib.archives.archives.iter().any(|a| a.folder_id == folder_id && a.prompt.is_some());
  if bound {
    return Err("已指定提示词文档，不能重复新建".to_string());
  }
  Ok(f.name.clone())
}

/// 资产显示名词干：首条活 Placement 名去扩展名（卡片显示名口径），无活位置 = meta.name（建档词干）
fn display_stem(lib: &Library, meta: &AssetMeta) -> String {
  lib
    .index
    .placements_of(&meta.id)
    .first()
    .map(|p| {
      Path::new(&p.name).file_stem().map(|s| s.to_string_lossy().into_owned()).filter(|s| !s.is_empty()).unwrap_or_else(|| p.name.clone())
    })
    .unwrap_or_else(|| meta.name.clone())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{ARCHIVES_FILE, FOLDERS_FILE};
  use crate::core::models::{Folder, FoldersFile};
  use crate::core::undo::{DiskOp, UndoManager};
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::recycle::sync_after_purge;
  use std::collections::HashMap;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  /// 造文本资产（真实内容入库）+ origin Placement 挂 root
  fn add_text(h: &mut Harness, id: &str, name: &str, content: &str) -> AssetMeta {
    let src = h.dir.path().join(name);
    std::fs::write(&src, content).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    ops::add_placement(&mut h.lib, id.into(), "root".into(), name.into(), true, &mut HashMap::new());
    operations::finish(&mut h.lib).unwrap();
    meta
  }

  /// 造二进制资产（fake 字节，ext 由文件名带）+ origin Placement 挂 root
  fn add_bin(h: &mut Harness, id: &str, name: &str) -> AssetMeta {
    add_bin_bytes(h, id, name, b"fake")
  }

  fn add_bin_bytes(h: &mut Harness, id: &str, name: &str, bytes: &[u8]) -> AssetMeta {
    let src = h.dir.path().join(name);
    std::fs::write(&src, bytes).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    ops::add_placement(&mut h.lib, id.into(), "root".into(), name.into(), true, &mut HashMap::new());
    operations::finish(&mut h.lib).unwrap();
    meta
  }

  fn add_archive(lib: &mut Library, id: &str, name: &str) {
    let folder = Folder {
      id: id.into(),
      name: name.into(),
      parent: None,
      block: None,
      icon: None,
      kind: Some(ARCHIVE_KIND.into()),
      color: None,
      groups: None,
      order: 0,
    };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  /// 最小 PNG：签名 + IHDR + tEXt(parameters) + IEND
  fn png_with_parameters(text: &str) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |ctype: &[u8; 4], data: &[u8]| {
      out.extend_from_slice(&(data.len() as u32).to_be_bytes());
      out.extend_from_slice(ctype);
      out.extend_from_slice(data);
      out.extend_from_slice(&[0; 4]);
    };
    chunk(b"IHDR", &[0; 13]);
    let mut t = b"parameters".to_vec();
    t.push(0);
    t.extend_from_slice(text.as_bytes());
    chunk(b"tEXt", &t);
    chunk(b"IEND", &[]);
    out
  }

  #[test]
  fn assign_persists_binding_and_validates() {
    let mut h = make_lib();
    add_bin(&mut h, "AN-IMG", "雪牧.png");
    add_text(&mut h, "AN-DOC", "提示词.md", "masterpiece, 雪牧");
    let before = h.lib.index.assets["AN-IMG"].modified_at.clone();

    assert!(assign_asset(&mut h.lib, "AN-IMG", "AN-DOC").unwrap());
    let p = h.lib.index.assets["AN-IMG"].prompt.clone().unwrap();
    assert_eq!(p.asset_id.as_deref(), Some("AN-DOC"), "已指定态引用源文档");
    assert_eq!(p.name, "提示词.md", "显示名快照 = 首条活 Placement 名");
    assert_eq!(p.content, "masterpiece, 雪牧", "内容快照");
    assert!(!p.updated_at.is_empty());
    // 落盘一致 + 刷 modified_at（单写点口径）
    let disk: AssetMeta = read_json(&h.lib.info_dir("AN-IMG").join("metadata.json")).unwrap();
    assert_eq!(disk.prompt, Some(p));
    assert!(disk.modified_at >= before);
    // 同源同内容 = 无改动 false
    assert!(!assign_asset(&mut h.lib, "AN-IMG", "AN-DOC").unwrap());
    // 门禁：自指 / 不存在 / 非文本
    assert!(assign_asset(&mut h.lib, "AN-IMG", "AN-IMG").unwrap_err().contains("自身"));
    assert!(assign_asset(&mut h.lib, "AN-IMG", "AN-不存在").unwrap_err().contains("不存在"));
    assert!(assign_asset(&mut h.lib, "AN-DOC", "AN-IMG").unwrap_err().contains("不是文本资源"));
    assert!(assign_asset(&mut h.lib, "AN-不存在", "AN-DOC").is_err());
  }

  #[test]
  fn assign_archive_lazy_record_and_persists() {
    let mut h = make_lib();
    add_archive(&mut h.lib, "F-A", "雪牧档案");
    add_text(&mut h, "AN-DOC", "设定.txt", "档案提示词内容");

    assert!(assign_archive(&mut h.lib, "F-A", "AN-DOC").unwrap());
    let rec = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap();
    let p = rec.prompt.clone().unwrap();
    assert_eq!(p.asset_id.as_deref(), Some("AN-DOC"));
    assert_eq!(p.content, "档案提示词内容");
    let disk: crate::core::models::ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(disk.archives.len(), 1, "懒建记录落盘");
    assert_eq!(disk.archives[0].prompt.as_ref().unwrap().content, "档案提示词内容");
    assert!(!assign_archive(&mut h.lib, "F-A", "AN-DOC").unwrap(), "无改动 false");
    // 普通文件夹报错
    let folder = Folder { id: "F-P".into(), name: "普通".into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
    h.lib.folders.folders.push(folder.clone());
    h.lib.index.folders.insert("F-P".into(), folder);
    assert!(assign_archive(&mut h.lib, "F-P", "AN-DOC").unwrap_err().contains("不是档案"));
  }

  #[test]
  fn clear_removes_binding() {
    let mut h = make_lib();
    add_bin(&mut h, "AN-IMG", "雪牧.png");
    add_text(&mut h, "AN-DOC", "提示词.md", "内容");
    add_archive(&mut h.lib, "F-A", "档案");
    assign_asset(&mut h.lib, "AN-IMG", "AN-DOC").unwrap();
    assign_archive(&mut h.lib, "F-A", "AN-DOC").unwrap();

    assert!(clear_asset(&mut h.lib, "AN-IMG").unwrap());
    assert!(h.lib.index.assets["AN-IMG"].prompt.is_none(), "整绑定移除（快照随解绑清除）");
    assert!(!clear_asset(&mut h.lib, "AN-IMG").unwrap(), "已无绑定 = 无改动");
    assert!(clear_archive(&mut h.lib, "F-A").unwrap());
    assert!(h.lib.archives.archives[0].prompt.is_none());
    assert!(!clear_archive(&mut h.lib, "F-A").unwrap());
    let disk: crate::core::models::ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert!(disk.archives[0].prompt.is_none(), "档案侧落盘一致");
  }

  #[test]
  fn refresh_snapshot_rereads_source() {
    let mut h = make_lib();
    add_bin(&mut h, "AN-IMG", "雪牧.png");
    add_text(&mut h, "AN-DOC", "提示词.md", "旧内容");
    add_archive(&mut h.lib, "F-A", "档案");
    assign_asset(&mut h.lib, "AN-IMG", "AN-DOC").unwrap();
    assign_archive(&mut h.lib, "F-A", "AN-DOC").unwrap();

    // 源内容变更（经写文本单写点）→ 同步快照
    text::write_origin(&mut h.lib, "AN-DOC", "新内容 v2").unwrap();
    assert!(refresh_asset_snapshot(&mut h.lib, "AN-IMG").unwrap());
    assert_eq!(h.lib.index.assets["AN-IMG"].prompt.as_ref().unwrap().content, "新内容 v2");
    assert!(refresh_archive_snapshot(&mut h.lib, "F-A").unwrap());
    assert_eq!(h.lib.archives.archives[0].prompt.as_ref().unwrap().content, "新内容 v2");
    let disk: AssetMeta = read_json(&h.lib.info_dir("AN-IMG").join("metadata.json")).unwrap();
    assert_eq!(disk.prompt.unwrap().content, "新内容 v2", "落盘一致");
    // 内容未变 = 无改动
    assert!(!refresh_asset_snapshot(&mut h.lib, "AN-IMG").unwrap());
    assert!(!refresh_archive_snapshot(&mut h.lib, "F-A").unwrap());
    // 未指定 / 已扫描报错
    assert!(refresh_asset_snapshot(&mut h.lib, "AN-DOC").unwrap_err().contains("未指定提示词"));
    h.lib.index.assets.get_mut("AN-IMG").unwrap().prompt = Some(PromptBinding {
      asset_id: None,
      name: SCANNED_NAME.into(),
      content: "扫描".into(),
      updated_at: "2026-09-26T00:00:00+08:00".into(),
    });
    assert!(refresh_asset_snapshot(&mut h.lib, "AN-IMG").unwrap_err().contains("无源文档"));
  }

  #[test]
  fn analyze_scans_embedded_and_persists_scanned_state() {
    let mut h = make_lib();
    add_bin_bytes(&mut h, "AN-IMG", "生成图.png", &png_with_parameters("1girl, 雪牧, masterpiece"));
    add_text(&mut h, "AN-TXT", "文档.md", "x");

    let b = analyze_asset(&mut h.lib, "AN-IMG").unwrap();
    assert_eq!(b.asset_id, None, "已扫描态纯快照无引用");
    assert_eq!(b.name, SCANNED_NAME);
    assert_eq!(b.content, "1girl, 雪牧, masterpiece");
    let disk: AssetMeta = read_json(&h.lib.info_dir("AN-IMG").join("metadata.json")).unwrap();
    assert_eq!(disk.prompt, Some(b));
    // 门禁：文本资产无内嵌提示词可分析；无内嵌源报错
    assert!(analyze_asset(&mut h.lib, "AN-TXT").unwrap_err().contains("该格式无内嵌提示词可分析"));
    add_bin(&mut h, "AN-PNG2", "普通.png");
    assert!(analyze_asset(&mut h.lib, "AN-PNG2").unwrap_err().contains("未检测到内嵌提示词"));
    assert!(analyze_asset(&mut h.lib, "AN-不存在").is_err());
  }

  /// undo 回滚（UndoManager::default() 手动三段，custom_data.rs 范例）：
  /// record 目标 metadata.json → assign → commit → undo → 绑定回滚
  #[test]
  fn assign_undoes_to_previous_binding() {
    let mut h = make_lib();
    add_bin(&mut h, "AN-IMG", "雪牧.png");
    add_text(&mut h, "AN-DOC", "提示词.md", "内容");

    let meta_path = h.lib.info_dir("AN-IMG").join("metadata.json");
    let mut undo = UndoManager::default();
    undo.begin("指定提示词");
    undo.record_file(&meta_path).unwrap();
    assign_asset(&mut h.lib, "AN-IMG", "AN-DOC").unwrap();
    undo.commit().unwrap();
    assert!(h.lib.index.assets["AN-IMG"].prompt.is_some());

    undo.undo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert!(h.lib.index.assets["AN-IMG"].prompt.is_none(), "undo 回滚绑定");
    undo.redo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.assets["AN-IMG"].prompt.as_ref().unwrap().asset_id.as_deref(), Some("AN-DOC"), "redo 重做");
  }

  /// 源文档被彻底删除不断链（sync_after_purge 钩子）：引用置 None、快照保留降级已扫描态；
  /// 档案侧同口径（archives.json 有改动才落）
  #[test]
  fn purge_source_degrades_binding_keeps_snapshot() {
    let mut h = make_lib();
    add_bin(&mut h, "AN-IMG", "雪牧.png");
    add_text(&mut h, "AN-DOC", "提示词.md", "快照内容保留");
    add_archive(&mut h.lib, "F-A", "档案");
    assign_asset(&mut h.lib, "AN-IMG", "AN-DOC").unwrap();
    assign_archive(&mut h.lib, "F-A", "AN-DOC").unwrap();

    sync_after_purge(&mut h.lib, &["AN-DOC".to_string()]).unwrap();
    let p = h.lib.index.assets["AN-IMG"].prompt.clone().unwrap();
    assert_eq!(p.asset_id, None, "引用置 None 降级已扫描态");
    assert_eq!(p.content, "快照内容保留", "快照保留不断链");
    assert_eq!(p.name, "提示词.md", "显示名快照保留");
    let disk: AssetMeta = read_json(&h.lib.info_dir("AN-IMG").join("metadata.json")).unwrap();
    assert_eq!(disk.prompt.as_ref().unwrap().asset_id, None, "资产侧落盘一致");
    let rec = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap();
    assert_eq!(rec.prompt.as_ref().unwrap().asset_id, None, "档案侧同降级");
    let disk_a: crate::core::models::ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(disk_a.archives[0].prompt.as_ref().unwrap().content, "快照内容保留", "档案侧落盘一致");
  }

  #[test]
  fn doc_seed_by_state() {
    let mut h = make_lib();
    add_bin(&mut h, "AN-IMG", "雪牧.png");
    // 未指定态：空内容 + 名 = 资产显示名词干
    let (name, content) = doc_seed(&h.lib, "AN-IMG").unwrap();
    assert_eq!((name.as_str(), content.as_str()), ("雪牧", ""));
    // 已扫描态：内容 = 快照
    h.lib.index.assets.get_mut("AN-IMG").unwrap().prompt = Some(PromptBinding {
      asset_id: None,
      name: SCANNED_NAME.into(),
      content: "扫描快照".into(),
      updated_at: "2026-09-26T00:00:00+08:00".into(),
    });
    let (_, content) = doc_seed(&h.lib, "AN-IMG").unwrap();
    assert_eq!(content, "扫描快照");
    // 已指定态报错
    add_text(&mut h, "AN-DOC", "提示词.md", "x");
    assign_asset(&mut h.lib, "AN-IMG", "AN-DOC").unwrap();
    assert!(doc_seed(&h.lib, "AN-IMG").unwrap_err().contains("已指定"));
    assert!(doc_seed(&h.lib, "AN-不存在").is_err());
    // 档案侧：名 = 档案名，恒空内容；非档案 / 已指定报错
    add_archive(&mut h.lib, "F-A", "雪牧档案");
    assert_eq!(archive_doc_seed(&h.lib, "F-A").unwrap(), "雪牧档案");
    assign_archive(&mut h.lib, "F-A", "AN-DOC").unwrap();
    assert!(archive_doc_seed(&h.lib, "F-A").unwrap_err().contains("已指定"));
    assert!(archive_doc_seed(&h.lib, "F-不存在").is_err());
  }

  /// 新建提示词文档全流程（命令层同序）：未指定建空文档 + 绑定；命名「资产名.md」撞车 _2；
  /// undo 一步回滚（文档消失 + 绑定清除）
  #[test]
  fn new_prompt_document_full_flow_and_undo() {
    let mut h = make_lib();
    add_bin(&mut h, "AN-IMG", "雪牧.png");
    let (doc_name, content) = doc_seed(&h.lib, "AN-IMG").unwrap();

    // 命令层撤销三步同序：record folders.json + 目标 metadata.json → 建文档 + 绑定 → 磁盘逆操作
    let mut undo = UndoManager::default();
    undo.begin("新建提示词文档");
    undo.record_file(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    undo.record_file(&h.lib.info_dir("AN-IMG").join("metadata.json")).unwrap();
    let (view, info_dir, backup) = new_prompt_document(&mut h.lib, None, "md", &doc_name, &content).unwrap();
    assert_eq!(view.name, "雪牧.md", "名 = 资产显示名词干 + 扩展名");
    assign_asset(&mut h.lib, "AN-IMG", &view.asset_id).unwrap();
    assert!(info_dir.join("雪牧.md").exists());
    undo.record_disk_op(
      DiskOp::Rename { from: info_dir.clone(), to: backup.clone() },
      DiskOp::Rename { from: backup, to: info_dir.clone() },
    );
    undo.commit().unwrap();

    let doc_id = view.asset_id.clone();
    let p = h.lib.index.assets["AN-IMG"].prompt.clone().unwrap();
    assert_eq!(p.asset_id.as_deref(), Some(doc_id.as_str()), "绑定新文档");
    assert_eq!(p.content, "", "空文档空快照");

    // 撞名唯一化（第一文档已建，同名再来 = _1 后缀，library::unique_name 口径；服务层直测命名）
    let (v2, _, _) = new_prompt_document(&mut h.lib, None, "md", "雪牧", "").unwrap();
    assert_eq!(v2.name, "雪牧_1.md");
    std::fs::remove_dir_all(h.lib.info_dir(&v2.asset_id)).unwrap(); // 清场不干扰后续断言
    h.lib.index.assets.remove(&v2.asset_id);

    // undo 一步回滚：info 目录挪入备份 + folders.json / 目标 metadata.json 快照写回
    undo.undo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert!(!h.lib.index.assets.contains_key(&doc_id), "undo 后文档消失");
    assert!(!h.lib.info_dir(&doc_id).exists(), "info 目录挪入备份");
    assert!(h.lib.index.assets["AN-IMG"].prompt.is_none(), "undo 后绑定清除");
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.items.len(), 1, "只剩雪牧.png 的 origin 位置");
    // redo：文档回來 + 绑定恢复
    undo.redo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert!(h.lib.index.assets.contains_key(&doc_id), "redo 文档回來");
    assert!(h.lib.index.assets["AN-IMG"].prompt.is_some());
  }

  /// 已扫描态「保存提示词为文档」：内容 = 快照带入新文档
  #[test]
  fn new_prompt_document_from_scanned_carries_snapshot() {
    let mut h = make_lib();
    add_bin_bytes(&mut h, "AN-IMG", "生成图.png", &png_with_parameters("扫描出的提示词"));
    analyze_asset(&mut h.lib, "AN-IMG").unwrap();
    let (doc_name, content) = doc_seed(&h.lib, "AN-IMG").unwrap();
    assert_eq!(content, "扫描出的提示词");

    let (view, _, _) = new_prompt_document(&mut h.lib, None, "md", &doc_name, &content).unwrap();
    assert_eq!(view.name, "生成图.md");
    let origin = h.lib.info_dir(&view.asset_id).join("生成图.md");
    assert_eq!(std::fs::read_to_string(&origin).unwrap(), "扫描出的提示词", "快照内容写入新文档");
    // 写内容同步 metadata.size / hash（write_origin 口径）
    let meta = &h.lib.index.assets[&view.asset_id];
    assert_eq!(meta.size, "扫描出的提示词".len() as u64);
    assert_eq!(meta.hash, ops::sha256_bytes("扫描出的提示词".as_bytes()));
    assign_asset(&mut h.lib, "AN-IMG", &view.asset_id).unwrap();
    assert_eq!(h.lib.index.assets["AN-IMG"].prompt.as_ref().unwrap().name, "生成图.md", "绑定显示名 = 新文档 Placement 名");
  }
}
