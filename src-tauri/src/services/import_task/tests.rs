  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{ASSTES_DIR, FOLDERS_FILE};
  use crate::core::models::FoldersFile;
  use crate::services::import::Source;
  use std::fs;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Mutex<Option<Library>>,
    root: PathBuf,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { _dir: dir, lib: Mutex::new(Some(lib)), root }
  }

  fn opts(structure: Structure, source: Source, paths: Vec<PathBuf>) -> ImportOptions {
    ImportOptions { structure, source, target_folder: None, paths }
  }

  /// 同步跑导入并收集事件
  fn run(h: &Harness, opts: ImportOptions) -> (ImportOutcome, Vec<ImportEvent>) {
    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    let outcome = run_import(&h.lib, h.root.clone(), opts, |e| events.lock().unwrap().push(e), &cancel);
    (outcome, events.into_inner().unwrap())
  }

  /// 事件类型序列（断言顺序用）
  fn kinds(events: &[ImportEvent]) -> Vec<&'static str> {
    events
      .iter()
      .map(|e| match e {
        ImportEvent::Started { .. } => "started",
        ImportEvent::Folders => "folders",
        ImportEvent::Asset { .. } => "asset",
        ImportEvent::Finished { .. } => "finished",
      })
      .collect()
  }

  fn with_lib<R>(h: &Harness, f: impl FnOnce(&Library) -> R) -> R {
    f(h.lib.lock().unwrap().as_ref().unwrap())
  }

  fn info_dir_count(h: &Harness) -> usize {
    fs::read_dir(h.root.join(ASSTES_DIR)).unwrap().count()
  }

  /// 造两文件源树：素材/a.txt + 素材/子目录/b.txt
  fn make_tree(h: &Harness) -> PathBuf {
    let src = h._dir.path().join("素材");
    fs::create_dir_all(src.join("子目录")).unwrap();
    fs::write(src.join("a.txt"), "aaa").unwrap();
    fs::write(src.join("子目录/b.txt"), "bbb").unwrap();
    src
  }

  #[test]
  fn flatten_imports_flat_and_streams_events() {
    let h = make_lib();
    let src = make_tree(&h);
    let (out, events) = run(&h, opts(Structure::Flatten, Source::Copy, vec![src]));

    assert_eq!((out.imported, out.failed.len()), (2, 0), "failed: {:?}", out.failed);
    // 平铺：不建 Folder，两条 Placement 落根目录
    assert!(with_lib(&h, |l| l.folders.folders.is_empty()));
    assert_eq!(with_lib(&h, |l| l.index.placements_by_folder[ROOT_FOLDER_ID].len()), 2);
    // 事件序列：started → asset×2 → finished；flatten 无 folders
    assert_eq!(kinds(&events), vec!["started", "asset", "asset", "finished"]);
    assert!(matches!(events[0], ImportEvent::Started { total: 2 }));
    assert!(matches!(events.last().unwrap(), ImportEvent::Finished { imported: 2, .. }));
  }

  #[test]
  fn keep_builds_hierarchy_first_then_streams_assets() {
    let h = make_lib();
    let src = h._dir.path().join("参考");
    fs::create_dir_all(src.join("人物")).unwrap();
    fs::write(src.join("封面.png"), "x").unwrap();
    fs::write(src.join("人物/走路.fla"), "y").unwrap();

    let (out, events) = run(&h, opts(Structure::Keep, Source::Copy, vec![src]));
    assert_eq!(out.imported, 2);
    // folders 事件先于全部 asset 事件
    assert_eq!(kinds(&events), vec!["started", "folders", "asset", "asset", "finished"]);
    // 层级含根文件夹本身：参考（顶层）→ 人物（子级），Placement 落末级
    with_lib(&h, |l| {
      let top = l.folders.folders.iter().find(|f| f.name == "参考" && f.parent.is_none()).unwrap();
      let sub = l.folders.folders.iter().find(|f| f.name == "人物").unwrap();
      assert_eq!(sub.parent.as_deref(), Some(top.id.as_str()));
      assert_eq!(l.index.placements_by_folder[&sub.id].len(), 1);
      assert_eq!(l.index.placements_by_folder[&top.id].len(), 1);
    });
  }

  #[test]
  fn copy_keeps_source_move_removes_source() {
    let h = make_lib();
    let f1 = h._dir.path().join("复制.txt");
    let f2 = h._dir.path().join("剪切.txt");
    fs::write(&f1, "1").unwrap();
    fs::write(&f2, "2").unwrap();

    run(&h, opts(Structure::Flatten, Source::Copy, vec![f1.clone()]));
    assert!(f1.exists(), "复制后源文件应仍在");
    run(&h, opts(Structure::Flatten, Source::Move, vec![f2.clone()]));
    assert!(!f2.exists(), "剪切后源文件应消失");
  }

  #[test]
  fn same_hash_collects_conflict_without_linking() {
    let h = make_lib();
    let f1 = h._dir.path().join("甲.txt");
    let f2 = h._dir.path().join("乙.txt");
    fs::write(&f1, "相同内容").unwrap();
    fs::write(&f2, "相同内容").unwrap();

    // Move 模式：冲突挂起项的源文件同样保留不动（用户未决策前不做任何动作）
    let (out, events) = run(&h, opts(Structure::Flatten, Source::Move, vec![f1, f2.clone()]));
    assert_eq!((out.imported, out.conflicts.len(), out.failed.len()), (1, 1, 0));
    assert!(f2.exists(), "冲突挂起的源文件应保留");
    assert_eq!(info_dir_count(&h), 1);
    // 无自动链接：只有一条 origin Placement；冲突项不发 Asset 事件
    assert_eq!(with_lib(&h, |l| l.folders.items.len()), 1);
    assert_eq!(kinds(&events), vec!["started", "asset", "finished"]);
    // 冲突项双侧显示数据齐备（弹窗左右对照直接用）：txt 无宽高
    let c = &out.conflicts[0];
    assert_eq!(c.source_name, "乙.txt");
    assert_eq!(c.source_size, 12); // "相同内容" 4 字 × 3 字节 UTF-8
    assert_eq!((c.source_w, c.source_h), (None, None));
    assert_eq!((c.target_folder.as_str(), c.target_folder_name.as_str()), (ROOT_FOLDER_ID, "根目录"));
    assert_eq!((c.existing_name.as_str(), c.existing_ext.as_str()), ("甲.txt", "txt"));
    assert_eq!(c.existing_folder_name, "根目录");
    with_lib(&h, |l| assert!(l.index.assets.contains_key(&c.existing_asset_id)));

    // png 场景：双侧宽高 + 已有侧缩略图齐备
    let png1 = h._dir.path().join("旧图.png");
    let png2 = h._dir.path().join("新图.png");
    image::RgbaImage::from_pixel(64, 32, image::Rgba([1, 2, 3, 255])).save(&png1).unwrap();
    image::RgbaImage::from_pixel(64, 32, image::Rgba([1, 2, 3, 255])).save(&png2).unwrap();
    run(&h, opts(Structure::Flatten, Source::Copy, vec![png1]));
    let (out2, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![png2]));
    let c = &out2.conflicts[0];
    assert_eq!((c.source_w, c.source_h), (Some(64), Some(32)));
    assert_eq!((c.existing_w, c.existing_h), (Some(64), Some(32)));
    assert_eq!(c.existing_name, "旧图.png");
    assert!(c.existing_thumb_path.as_ref().unwrap().ends_with("_thumbnail.webp"));
  }

  #[test]
  fn duplicate_display_name_gets_counter_suffix() {
    let h = make_lib();
    let d1 = h._dir.path().join("源一");
    let d2 = h._dir.path().join("源二");
    fs::create_dir_all(&d1).unwrap();
    fs::create_dir_all(&d2).unwrap();
    fs::write(d1.join("男人.fla"), "内容一").unwrap();
    fs::write(d2.join("男人.fla"), "内容二").unwrap();

    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![d1, d2]));
    assert_eq!(out.imported, 2);
    with_lib(&h, |l| {
      let names: Vec<&str> = l.folders.items.iter().map(|p| p.name.as_str()).collect();
      assert!(names.contains(&"男人.fla"));
      assert!(names.contains(&"男人-1.fla"));
    });
  }

  #[test]
  fn mixed_files_and_folder_and_bad_path() {
    let h = make_lib();
    let single = h._dir.path().join("单文件.txt");
    fs::write(&single, "s").unwrap();
    let folder = h._dir.path().join("夹");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("内.txt"), "i").unwrap();
    let missing = h._dir.path().join("不存在.txt");

    let (out, _) = run(&h, opts(Structure::Keep, Source::Copy, vec![single, folder, missing]));
    assert_eq!(out.imported, 2);
    assert_eq!(out.failed.len(), 1);
    assert!(out.failed[0].reason.contains("不存在"));
  }

  #[test]
  fn import_writes_metadata_and_persists_folders() {
    let h = make_lib();
    let png = h._dir.path().join("图.PNG");
    image::RgbaImage::from_pixel(64, 32, image::Rgba([0, 255, 0, 255])).save(&png).unwrap();

    let (out, events) = run(&h, opts(Structure::Flatten, Source::Copy, vec![png]));
    assert_eq!(out.imported, 1);
    // Asset 事件带完整视图（宽高 / 缩略图绝对路径）
    let ImportEvent::Asset { placement } = &events[1] else { panic!("应为 asset 事件") };
    assert_eq!((placement.w, placement.h), (Some(64), Some(32)));
    assert!(placement.thumb_path.as_ref().unwrap().ends_with("_thumbnail.webp"));
    assert!(placement.origin);

    with_lib(&h, |l| {
      let asset = l.index.assets.values().next().unwrap();
      assert_eq!(asset.ext, "png");
      assert!(asset.hash.starts_with("sha256:"));
      let info = h.root.join(ASSTES_DIR).join(format!("{}.info", asset.id));
      assert!(info.join("metadata.json").is_file());
      assert!(info.join(asset.thumbnail.as_ref().unwrap()).is_file());
    });
    // folders.json 结束强制落盘
    let back: FoldersFile = read_json(&h.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(back.items.len(), 1);
    assert!(back.items[0].origin);
  }

  #[test]
  fn cancel_stops_mid_import_and_reports_partial_counts() {
    let h = make_lib();
    let paths: Vec<PathBuf> = (0..3)
      .map(|i| {
        let p = h._dir.path().join(format!("f{i}.txt"));
        fs::write(&p, format!("内容{i}")).unwrap();
        p
      })
      .collect();

    // 第一个 asset 事件即置位取消 → 后续文件不再处理
    let cancel = AtomicBool::new(false);
    let events = Mutex::new(Vec::new());
    let outcome = run_import(
      &h.lib,
      h.root.clone(),
      opts(Structure::Flatten, Source::Copy, paths),
      |e| {
        if matches!(e, ImportEvent::Asset { .. }) {
          cancel.store(true, Ordering::Relaxed);
        }
        events.lock().unwrap().push(e);
      },
      &cancel,
    );

    assert_eq!((outcome.imported, outcome.conflicts.len(), outcome.failed.len()), (1, 0, 0));
    assert_eq!(kinds(&events.into_inner().unwrap()), vec!["started", "asset", "finished"]);
    // 已导入的 1 条正常落盘
    assert_eq!(with_lib(&h, |l| l.folders.items.len()), 1);
  }

  #[test]
  fn cancel_preserves_collected_conflicts() {
    let h = make_lib();
    // 先入库一个资源作为冲突目标
    let base = h._dir.path().join("基准.txt");
    fs::write(&base, "相同内容").unwrap();
    run(&h, opts(Structure::Flatten, Source::Copy, vec![base]));

    let dup = h._dir.path().join("重复.txt");
    let n1 = h._dir.path().join("新一.txt");
    let n2 = h._dir.path().join("新二.txt");
    fs::write(&dup, "相同内容").unwrap();
    fs::write(&n1, "新内容一").unwrap();
    fs::write(&n2, "新内容二").unwrap();

    // 顺序：冲突项先入收集，第一个 asset 事件置位取消 → 第二个新文件不再处理
    let cancel = AtomicBool::new(false);
    let outcome = run_import(
      &h.lib,
      h.root.clone(),
      opts(Structure::Flatten, Source::Copy, vec![dup.clone(), n1, n2]),
      |e| {
        if matches!(e, ImportEvent::Asset { .. }) {
          cancel.store(true, Ordering::Relaxed);
        }
      },
      &cancel,
    );

    // 取消中断时已收集的冲突如实随结果返回
    assert_eq!((outcome.imported, outcome.conflicts.len(), outcome.failed.len()), (1, 1, 0));
    assert_eq!(outcome.conflicts[0].source_path, dup);
  }
