use std::collections::HashMap;
use std::sync::Mutex;

use crate::commands::library::AppState;
use crate::core::json_io::read_json;
use crate::core::models::{AssetMeta, ROOT_FOLDER_ID};
use crate::services::import::{self, PendingFile};
use crate::services::semantic::model::ModelManifest;
use crate::services::semantic::{media, playback};

use super::*;

fn managed_model_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models/multilingual-v1")
}

#[derive(Clone, Copy)]
enum SourceKind { Gif, Webp, Apng, Video, Sequence }

impl SourceKind {
  fn id(self) -> &'static str {
    match self {
      Self::Gif => "AN-SEGMENT-GIF", Self::Webp => "AN-SEGMENT-WEBP", Self::Apng => "AN-SEGMENT-APNG",
      Self::Video => "AN-SEGMENT-VIDEO", Self::Sequence => "AN-SEGMENT-SEQ",
    }
  }

  fn ext(self) -> &'static str {
    match self { Self::Gif => "gif", Self::Webp => "webp", Self::Apng => "png", Self::Video => "mp4", Self::Sequence => "png" }
  }
}

fn fixture(kind: SourceKind) -> (tempfile::TempDir, AppState) {
  let dir = tempfile::tempdir().unwrap();
  let mut lib = crate::core::library::create_test(dir.path());
  let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
  let id = kind.id();
  let source = match kind {
    SourceKind::Gif | SourceKind::Webp | SourceKind::Apng | SourceKind::Video => {
      let path = match kind {
        SourceKind::Gif => "tools/test-assets/anim3.gif",
        SourceKind::Webp => "tools/test-assets/anim3.webp",
        SourceKind::Apng => "tools/test-assets/anim3.png",
        SourceKind::Video => "spike/decode-proto/testdata/big.mp4",
        SourceKind::Sequence => unreachable!(),
      };
      let pending = PendingFile { path: project.join(path), chain: Vec::new() };
      import::build_asset_with_id(&lib.assets_dir, &pending, "sha256:segment-test", id, Source::Copy, None).unwrap()
    }
    SourceKind::Sequence => {
      let frames = (1..=3).map(|index| (project.join(format!("tools/test-assets/blank-test-{index}.png")), index)).collect::<Vec<_>>();
      sequence::build_sequence_asset(&lib.assets_dir, "blank-test", "png", 1, &frames, "sha256:segment-sequence", id, Source::Copy, None).unwrap()
    }
  };
  lib.index.assets.insert(id.into(), source.clone());
  import::add_placement(&mut lib, id.into(), ROOT_FOLDER_ID.into(), format!("{}.{}", source.name, source.ext), true, &mut HashMap::new());
  operations::finish(&mut lib).unwrap();
  let root = lib.root.clone();
  let manifest = ModelManifest::load(&managed_model_dir()).unwrap();
  let mut store = SemanticStore::open_library(&root).unwrap();
  store.register_model(&manifest.revision).unwrap();
  let fingerprint = media::source_fingerprint(&source, &lib.assets_dir, id).unwrap();
  let (start_ms, end_ms) = match kind { SourceKind::Video => (4000, 6000), SourceKind::Sequence => (42, 83), _ => (200, 400) };
  let (start_frame, end_frame) = if matches!(kind, SourceKind::Video) { (None, None) } else { (Some(1), Some(2)) };
  let segment = super::super::types::SegmentInput {
    start_ms, end_ms, start_frame, end_frame, kind: "visual".into(), text: "片段测试".into(), vector: vec![1.0; manifest.revision.dimension],
  };
  store.replace_asset_analysis(id, "anim", &fingerprint, &manifest.revision, media::ANALYSIS_VERSION, &[segment]).unwrap();
  let state = AppState {
    space: Mutex::new(None), library: Mutex::new(Some(lib)), imports: Mutex::new(HashMap::new()), inbox: Mutex::new(None),
    workspace: Mutex::new(Vec::new()), undo: Mutex::new(crate::core::undo::UndoManager::default()),
    ws_searches: Mutex::new(HashMap::new()), semantic_tasks: Mutex::new(HashMap::new()),
  };
  (dir, state)
}

fn commit_source(state: &AppState, id: &str, physical: bool) -> (PlacementView, PathBuf, AssetMeta) {
  let prepared = prepare_at(state, id, 1, None, physical, &managed_model_dir()).unwrap();
  let (view, info, _) = super::commit(state.library.lock().unwrap().as_mut().unwrap(), prepared).unwrap();
  let meta: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
  (view, info, meta)
}

fn commit_fixture(state: &AppState, physical: bool) -> (PlacementView, PathBuf, AssetMeta) {
  commit_source(state, SourceKind::Gif.id(), physical)
}

fn frame_difference(before: &image::RgbaImage, after: &image::RgbaImage) -> f64 {
  assert_eq!(before.dimensions(), after.dimensions());
  let mut difference = 0u64;
  for (left, right) in before.pixels().zip(after.pixels()) {
    assert_eq!(left.0[3], right.0[3], "透明度不应改变");
    for channel in 0..3 {
      difference += u64::from(left.0[channel].abs_diff(right.0[channel]));
    }
  }
  difference as f64 / (before.width() * before.height() * 3) as f64
}

fn assert_frame_close(before: &image::RgbaImage, after: &image::RgbaImage) {
  let difference = frame_difference(before, after);
  assert!(difference <= 32.0, "GIF 整帧平均色差过大：{difference}");
}

#[test]
fn virtual_segment_references_source_and_preserves_original_frames() {
  let (_dir, state) = fixture(SourceKind::Gif);
  let (view, info, meta) = commit_fixture(&state, false);
  assert_eq!(meta.r#type.as_deref(), Some("segment"));
  assert_eq!(meta.source.as_deref(), Some("AN-SEGMENT-GIF"));
  assert_eq!((meta.start_frame, meta.end_frame), (Some(1), Some(2)));
  assert_eq!(meta.frame_count, Some(2));
  assert!(!info.join(&meta.origin_name).exists(), "虚拟片段不复制媒体实体");
  assert!(info.join(meta.thumbnail.unwrap()).is_file());
  let playback = playback::resolve(&state.library.lock().unwrap().as_ref().unwrap().assets_dir, &view.asset_id).unwrap();
  assert_eq!(playback.media_meta().id, "AN-SEGMENT-GIF");
  assert_eq!(playback.segment.unwrap().frame_bounds().unwrap(), (1, 2));
  assert!(playback.origin.is_file());
  let lib = state.library.lock().unwrap();
  let origin = crate::services::asset_menu::real_origin_file(lib.as_ref().unwrap(), &view.asset_id).unwrap();
  assert_eq!(origin, playback.origin);
  drop(lib);
  let source_info = state.library.lock().unwrap().as_ref().unwrap().info_dir("AN-SEGMENT-GIF");
  let before: AssetMeta = read_json(&source_info.join("metadata.json")).unwrap();
  crate::commands::thumb_frame::apply_thumb_frame(state.library.lock().unwrap().as_mut().unwrap(), &view.asset_id, "f:1").unwrap();
  let after: AssetMeta = read_json(&source_info.join("metadata.json")).unwrap();
  assert_eq!(before, after, "虚拟片段设置封面不得改写来源 metadata");
  let virtual_meta: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
  assert_eq!(virtual_meta.id, view.asset_id);
  assert_eq!(virtual_meta.thumb_frame.as_deref(), Some("f:1"));
  std::fs::remove_dir_all(source_info).unwrap();
  let error = playback::resolve(&state.library.lock().unwrap().as_ref().unwrap().assets_dir, &view.asset_id).unwrap_err();
  assert!(error.contains("来源资源已删除"), "{error}");
}

#[test]
fn physical_segment_is_an_independent_playable_asset() {
  let (_dir, state) = fixture(SourceKind::Gif);
  let (_, info, meta) = commit_fixture(&state, true);
  assert_eq!(meta.r#type, None);
  assert_eq!(meta.source.as_deref(), Some("AN-SEGMENT-GIF"));
  assert_eq!((meta.start_frame, meta.end_frame), (Some(1), Some(2)));
  let output = info.join(&meta.origin_name);
  assert!(output.is_file());
  let probe = anim::probe(&output).expect("物理片段应保留可播放动图");
  assert_eq!(probe.frame_count, 2);
  let source = state.library.lock().unwrap().as_ref().unwrap().info_dir("AN-SEGMENT-GIF").join("anim3.gif");
  for (source_index, output_index) in [(1, 0), (2, 1)] {
    let before = anim::decode_frame(&source, source_index).unwrap().0;
    let after = anim::decode_frame(&output, output_index).unwrap().0;
    assert_frame_close(&before, &after);
  }
  assert!(playback::range_from_meta(&meta).unwrap().is_none(), "物理片段不依赖源文件播放");
}

#[test]
fn video_segment_preserves_real_time_range_and_exports_mp4() {
  let (_dir, state) = fixture(SourceKind::Video);
  let (view, info, meta) = commit_source(&state, SourceKind::Video.id(), false);
  let playback = playback::resolve(&state.library.lock().unwrap().as_ref().unwrap().assets_dir, &view.asset_id).unwrap();
  let range = playback.segment.clone().unwrap();
  assert_eq!((range.start_ms, range.end_ms), (4000, 6000));
  assert_eq!(range.start_frame, None);
  assert_eq!(playback.media_meta().id, SourceKind::Video.id());
  assert!(playback.origin.is_file());
  assert!(!info.join(&meta.origin_name).exists());
  let (_, info, meta) = commit_source(&state, SourceKind::Video.id(), true);
  assert_eq!(meta.r#type, None);
  let output = info.join(&meta.origin_name);
  let probe = crate::services::media::probe_video(&output).expect("物理视频应可解码");
  assert!((probe.duration - 2.0).abs() < 0.2, "视频截取时长错误：{}", probe.duration);
  assert_eq!((meta.start, meta.end), (Some(4.0), Some(6.0)));
}

#[test]
fn sequence_segment_exports_the_original_frame_indices() {
  let (_dir, state) = fixture(SourceKind::Sequence);
  let (_, info, meta) = commit_source(&state, SourceKind::Sequence.id(), true);
  assert_eq!((meta.start_frame, meta.end_frame), (Some(1), Some(2)));
  let output = info.join(&meta.origin_name);
  assert_eq!(anim::probe(&output).unwrap().frame_count, 2);
  let source = state.library.lock().unwrap().as_ref().unwrap().info_dir(SourceKind::Sequence.id()).join("frames");
  for (source_index, output_index) in [(1, 0), (2, 1)] {
    let before = image::open(source.join(format!("blank-test_{}.png", source_index + 1))).unwrap().to_rgba8();
    let after = anim::decode_frame(&output, output_index).unwrap().0;
    assert_frame_close(&before, &after);
  }
}

#[test]
fn webp_and_apng_segments_preserve_original_frames() {
  for kind in [SourceKind::Webp, SourceKind::Apng] {
    let (_dir, state) = fixture(kind);
    let (_, info, meta) = commit_source(&state, kind.id(), true);
    assert_eq!(meta.ext, kind.ext());
    let output = info.join(&meta.origin_name);
    assert_eq!(anim::probe(&output).unwrap().frame_count, 2);
    let source = state.library.lock().unwrap().as_ref().unwrap().info_dir(kind.id()).join(format!("anim3.{}", kind.ext()));
    for (source_index, output_index) in [(1, 0), (2, 1)] {
      let before = anim::decode_frame(&source, source_index).unwrap().0;
      let after = anim::decode_frame(&output, output_index).unwrap().0;
      assert_frame_close(&before, &after);
    }
  }
}
