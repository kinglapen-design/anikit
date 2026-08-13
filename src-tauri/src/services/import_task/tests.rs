  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::FOLDERS_FILE;
  use crate::core::models::{AssetMeta, FoldersFile};
  use crate::core::paths;
  use crate::services::import::Source;
  use std::fs;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Mutex<Option<Library>>,
    root: PathBuf,
    assets_dir: PathBuf,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let root = lib.root.clone();
    let assets_dir = lib.assets_dir.clone();
    Harness { _dir: dir, lib: Mutex::new(Some(lib)), root, assets_dir }
  }

  fn opts(structure: Structure, source: Source, paths: Vec<PathBuf>) -> ImportOptions {
    ImportOptions {
      structure,
      source,
      target_folder: None,
      paths,
      imported_from: None,
      sequence_mode: SequenceMode::Scatter,
    }
  }

  /// 打包模式导入（4.1）
  fn pack_opts(paths: Vec<PathBuf>) -> ImportOptions {
    ImportOptions { sequence_mode: SequenceMode::Pack, ..opts(Structure::Flatten, Source::Copy, paths) }
  }

  /// 同步跑导入并收集事件
  fn run(h: &Harness, opts: ImportOptions) -> (ImportOutcome, Vec<ImportEvent>) {
    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    let outcome = run_import(&h.lib, h.root.clone(), h.assets_dir.clone(), opts, |e| events.lock().unwrap().push(e), &cancel);
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
    fs::read_dir(&h.assets_dir).unwrap().count()
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
  fn conflict_fills_source_thumb_preview() {
    let h = make_lib();
    // 同内容两个路径：第一个入库，第二个撞 hash 挂起冲突
    let svg_a = h._dir.path().join("甲.svg");
    let svg_b = h._dir.path().join("乙.svg");
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect width="40" height="40" fill="#ff0000"/></svg>"##;
    fs::write(&svg_a, svg).unwrap();
    fs::write(&svg_b, svg).unwrap();
    run(&h, opts(Structure::Flatten, Source::Copy, vec![svg_a]));
    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![svg_b]));

    assert_eq!(out.conflicts.len(), 1);
    // webview 直渲不了的 svg：后端生成预览缓存（cache/tmp），供弹窗"新的文件"显示
    let preview = out.conflicts[0].source_thumb_path.as_deref().expect("应生成预览缓存");
    assert!(preview.ends_with(".webp") && preview.contains("tmp"), "预览缓存路径：{preview}");
    assert!(Path::new(preview).is_file());

    // 图片格式（png）走前端直读源文件，不生成预览缓存
    let png_a = h._dir.path().join("一.png");
    let png_b = h._dir.path().join("二.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255])).save(&png_a).unwrap();
    fs::copy(&png_a, &png_b).unwrap();
    run(&h, opts(Structure::Flatten, Source::Copy, vec![png_a]));
    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![png_b]));
    assert_eq!(out.conflicts.len(), 1);
    assert!(out.conflicts[0].source_thumb_path.is_none(), "图片格式不生成预览缓存");
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
      let info = paths::info_dir(&h.assets_dir, &asset.id);
      assert!(info.join("metadata.json").is_file());
      assert!(info.join(asset.thumbnail.as_ref().unwrap()).is_file());
    });
    // folders.json 结束强制落盘
    let back: FoldersFile = read_json(&h.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(back.items.len(), 1);
    assert!(back.items[0].origin);
  }

  #[test]
  fn cancel_before_start_processes_nothing() {
    let h = make_lib();
    let paths: Vec<PathBuf> = (0..3)
      .map(|i| {
        let p = h._dir.path().join(format!("f{i}.txt"));
        fs::write(&p, format!("内容{i}")).unwrap();
        p
      })
      .collect();

    // 预置取消：不领取任何文件（并行语义：置位后不再领新活），事件仍完整
    let cancel = AtomicBool::new(true);
    let events = Mutex::new(Vec::new());
    let outcome = run_import(
      &h.lib,
      h.root.clone(),
      h.assets_dir.clone(),
      opts(Structure::Flatten, Source::Copy, paths),
      |e| events.lock().unwrap().push(e),
      &cancel,
    );

    assert_eq!((outcome.imported, outcome.conflicts.len(), outcome.failed.len()), (0, 0, 0));
    assert_eq!(kinds(&events.into_inner().unwrap()), vec!["started", "finished"]);
    assert_eq!(with_lib(&h, |l| l.folders.items.len()), 0);
  }

  #[test]
  fn cancel_mid_import_keeps_completed_and_conflicts() {
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

    // 第一个 asset 事件即置位取消。并行语义：冲突在阶段 2 已收集（确定），
    // 进行中的文件写完（imported ≥ 1，具体数量取决于调度，不作硬性断言）
    let cancel = AtomicBool::new(false);
    let events = Mutex::new(Vec::new());
    let outcome = run_import(
      &h.lib,
      h.root.clone(),
      h.assets_dir.clone(),
      opts(Structure::Flatten, Source::Copy, vec![dup.clone(), n1, n2]),
      |e| {
        if matches!(e, ImportEvent::Asset { .. }) {
          cancel.store(true, Ordering::Relaxed);
        }
        events.lock().unwrap().push(e);
      },
      &cancel,
    );

    assert!(outcome.imported >= 1, "触发取消的文件应已完成：{outcome:?}");
    assert_eq!(outcome.conflicts.len(), 1);
    assert_eq!(outcome.failed.len(), 0);
    assert_eq!(outcome.conflicts[0].source_path, dup);
    // 不变式：started 必先、finished 必尾
    let events = events.into_inner().unwrap();
    assert!(matches!(events.first().unwrap(), ImportEvent::Started { .. }));
    assert!(matches!(events.last().unwrap(), ImportEvent::Finished { .. }));
    // 已注册数量与落盘一致（+1 为冲突目标自身的 origin Placement）
    assert_eq!(with_lib(&h, |l| l.folders.items.len()), outcome.imported as usize + 1);
  }

  #[test]
  fn parallel_import_of_32_files_matches_serial_semantics() {
    let h = make_lib();
    let paths: Vec<PathBuf> = (0..32)
      .map(|i| {
        let p = h._dir.path().join(format!("并行{i}.txt"));
        fs::write(&p, format!("内容{i}")).unwrap();
        p
      })
      .collect();

    let (out, events) = run(&h, opts(Structure::Flatten, Source::Copy, paths));
    assert_eq!((out.imported, out.conflicts.len(), out.failed.len()), (32, 0, 0), "failed: {:?}", out.failed);
    // 与串行一致的终态：32 条 Placement、32 个实体、名字全唯一
    with_lib(&h, |l| {
      assert_eq!(l.folders.items.len(), 32);
      assert_eq!(l.index.assets.len(), 32);
      let names: std::collections::HashSet<&str> = l.folders.items.iter().map(|p| p.name.as_str()).collect();
      assert_eq!(names.len(), 32);
    });
    assert_eq!(info_dir_count(&h), 32);
    // 事件不变式：started 必先、finished 必尾、asset 数量 = 32
    assert!(matches!(events.first().unwrap(), ImportEvent::Started { total: 32 }));
    assert!(matches!(events.last().unwrap(), ImportEvent::Finished { .. }));
    assert_eq!(events.iter().filter(|e| matches!(e, ImportEvent::Asset { .. })).count(), 32);
  }

  /// 性能基准（手动跑）：cargo test bench_serial_vs_parallel -- --ignored --nocapture
  /// 100 张 1024x512 随机噪声图（≈2MB/张），串行 vs 并行（sha256 + 复制 + 缩略图）
  #[test]
  #[ignore]
  fn bench_serial_vs_parallel() {
    use rand::Rng;
    use rayon::prelude::*;
    use std::time::Instant;

    let h = make_lib();
    let dir = h._dir.path().join("基准素材");
    fs::create_dir_all(&dir).unwrap();
    let mut rng = rand::thread_rng();
    let paths: Vec<PathBuf> = (0..100)
      .map(|i| {
        let p = dir.join(format!("噪声{i:03}.png"));
        let img =
          image::RgbaImage::from_fn(1024, 512, |_, _| image::Rgba([rng.gen(), rng.gen(), rng.gen(), 255]));
        img.save(&p).unwrap();
        p
      })
      .collect();
    println!(">>> 100 张测试图就绪（约 2MB/张 随机噪声 PNG）");

    // 单文件重活（与管线阶段 1+3 相同）：sha256 + 复制 + 探测 + 缩略图 + metadata
    let heavy = |p: &PathBuf, tag: &str| {
      let file = PendingFile { path: p.clone(), chain: Vec::new() };
      let hash = ops::sha256_file(&file.path).unwrap();
      ops::build_asset_with_id(&h.assets_dir, &file, &hash, tag, Source::Copy, None).unwrap();
    };

    let t = Instant::now();
    for (i, p) in paths.iter().enumerate() {
      heavy(p, &format!("BS{i:03}"));
    }
    let serial = t.elapsed();

    let t = Instant::now();
    paths.par_iter().enumerate().for_each(|(i, p)| heavy(p, &format!("BP{i:03}")));
    let parallel = t.elapsed();

    let speedup = serial.as_secs_f64() / parallel.as_secs_f64();
    println!(">>> 串行: {serial:.2?}  并行: {parallel:.2?}  加速比: {speedup:.2}x");
  }

  // ---------- 收件箱撞 hash 冲突（2026-08-13 用户定案：同渠道静默合并废除，一律走正常重复提示） ----------

  /// inbox 渠道导入辅助：携带 importedFrom 标记 + Move 过境（自动 / 手动同口径，不再区分）
  fn inbox_opts(paths: Vec<PathBuf>) -> ImportOptions {
    ImportOptions {
      structure: Structure::Flatten,
      source: Source::Move,
      target_folder: None,
      paths,
      imported_from: Some("inbox".into()),
      sequence_mode: SequenceMode::Scatter,
    }
  }

  /// 唯一资源的 info 目录（单资源场景）
  fn only_info_dir(h: &Harness) -> PathBuf {
    fs::read_dir(&h.assets_dir).unwrap().next().unwrap().unwrap().path()
  }

  /// 撞同渠道（inbox）资产同 hash：冲突挂起走重复提示，不静默合并——已有资产不改名
  ///（磁盘实体 / metadata.json / 索引 / Placement 全不动），源文件保留待决策，不发 Asset 事件
  #[test]
  fn inbox_hash_collision_suspends_conflict_without_rename() {
    let h = make_lib();
    let recover = h._dir.path().join("RECOVER_无标题.fla");
    fs::write(&recover, "相同内容").unwrap();
    let (out1, _) = run(&h, inbox_opts(vec![recover]));
    assert_eq!((out1.imported, out1.conflicts.len()), (1, 0));

    let real = h._dir.path().join("无标题.fla");
    fs::write(&real, "相同内容").unwrap();
    let (out2, ev2) = run(&h, inbox_opts(vec![real.clone()]));
    assert_eq!((out2.imported, out2.conflicts.len(), out2.failed.len()), (0, 1, 0), "同渠道撞 hash 应冲突挂起，不静默合并");
    assert!(real.exists(), "冲突未决策源文件保留");
    assert_eq!(out2.conflicts[0].existing_name, "RECOVER_无标题.fla");

    assert_eq!(info_dir_count(&h), 1, "资产唯一");
    let (origin_name, ifrom, placement_name) = with_lib(&h, |l| {
      let meta = l.index.assets.values().next().unwrap();
      (meta.origin_name.clone(), meta.imported_from.clone(), l.folders.items[0].name.clone())
    });
    assert_eq!(origin_name, "RECOVER_无标题.fla", "已有资产不被改名");
    assert_eq!(placement_name, "RECOVER_无标题.fla", "Placement 显示名不动");
    assert_eq!(ifrom.as_deref(), Some("inbox"), "渠道标记保留（已导入区块口径不变）");

    let info_dir = only_info_dir(&h);
    let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).unwrap();
    assert_eq!(meta.origin_name, "RECOVER_无标题.fla", "metadata.json 不动");
    assert!(info_dir.join("RECOVER_无标题.fla").exists(), "磁盘实体名不动");
    assert!(!ev2.iter().any(|e| matches!(e, ImportEvent::Asset { .. })), "冲突挂起不发 Asset 事件");
  }

  /// 同名同内容的纯重复过境同样冲突挂起（原静默丢弃一并废除）：源保留待用户决策
  #[test]
  fn inbox_pure_duplicate_also_conflicts() {
    let h = make_lib();
    let a = h._dir.path().join("a.txt");
    fs::write(&a, "x").unwrap();
    run(&h, inbox_opts(vec![a.clone()]));
    fs::write(&a, "x").unwrap(); // 同名同内容再来一次
    let (out, ev) = run(&h, inbox_opts(vec![a.clone()]));
    assert_eq!((out.imported, out.conflicts.len(), out.failed.len()), (0, 1, 0));
    assert!(a.exists(), "冲突未决策源文件保留");
    assert_eq!(info_dir_count(&h), 1);
    assert!(!ev.iter().any(|e| matches!(e, ImportEvent::Asset { .. })), "冲突挂起不发 Asset 事件");
  }

  /// 已有资产非 inbox 渠道（正式资产）：照常冲突询问，源保留待决策
  #[test]
  fn inbox_import_conflicts_against_formal_asset() {
    let h = make_lib();
    let formal = h._dir.path().join("正式.fla");
    fs::write(&formal, "相同内容").unwrap();
    run(&h, opts(Structure::Flatten, Source::Copy, vec![formal]));

    let new = h._dir.path().join("新来的.fla");
    fs::write(&new, "相同内容").unwrap();
    let (out, _) = run(&h, inbox_opts(vec![new.clone()]));
    assert_eq!(out.conflicts.len(), 1, "正式资产应走冲突询问");
    assert!(new.exists(), "冲突未决策源文件保留");
    let origin = with_lib(&h, |l| l.index.assets.values().next().unwrap().origin_name.clone());
    assert_eq!(origin, "正式.fla", "正式资产名不受影响");
  }

  /// 反向：常规导入撞 inbox 渠道资产 → 照常冲突（合并废除后双向同口径）
  #[test]
  fn normal_import_still_conflicts_against_inbox_asset() {
    let h = make_lib();
    let a = h._dir.path().join("RECOVER_a.fla");
    fs::write(&a, "相同内容").unwrap();
    run(&h, inbox_opts(vec![a]));

    let b = h._dir.path().join("b.fla");
    fs::write(&b, "相同内容").unwrap();
    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![b]));
    assert_eq!(out.conflicts.len(), 1, "常规导入撞 inbox 资产仍应询问");
  }

  // ---------- 序列帧打包（PRD 01-2.5，4.1） ----------

  /// 造 PNG 序列帧目录（内容按序号区分）
  fn make_sequence(h: &Harness, dir: &str, stem: &str, nums: &[u32]) -> PathBuf {
    let src = h._dir.path().join(dir);
    fs::create_dir_all(&src).unwrap();
    for &n in nums {
      let p = src.join(format!("{stem}{n:03}.png"));
      image::RgbaImage::from_pixel(16, 12, image::Rgba([n as u8, 10, 10, 255])).save(&p).unwrap();
    }
    src
  }

  /// 打包模式：序列帧整组建一个 Asset（frames/ 归一 + fps/frameCount/thumbFrame），
  /// 散文件照常逐个导入；started.total 按单元计
  #[test]
  fn pack_imports_sequence_as_single_asset() {
    let h = make_lib();
    let src = make_sequence(&h, "导出", "走路", &[1, 2, 3, 4]);
    fs::write(src.join("封面.txt"), "散文件").unwrap();

    let (out, events) = run(&h, pack_opts(vec![src]));
    assert_eq!((out.imported, out.failed.len()), (2, 0), "failed: {:?}", out.failed);
    assert!(matches!(events[0], ImportEvent::Started { total: 2 }), "total 按单元计：1 组 + 1 散文件");
    assert_eq!(info_dir_count(&h), 2);

    let meta = with_lib(&h, |l| l.index.assets.values().find(|a| a.frame_count.is_some()).unwrap().clone());
    assert_eq!((meta.name.as_str(), meta.ext.as_str()), ("走路", "png"));
    assert_eq!((meta.fps, meta.frame_count), (Some(24.0), Some(4)));
    assert_eq!(meta.thumb_frame.as_deref(), Some("走路-001.png"));
    assert_eq!((meta.width, meta.height), (Some(16), Some(12)));
    // frames/ 归一重命名（位宽 3），缩略图复用图片管线
    let info_dir = paths::info_dir(&h.assets_dir, &meta.id);
    for n in 1..=4 {
      assert!(info_dir.join("frames").join(format!("走路-{n:03}.png")).is_file());
    }
    assert!(meta.thumbnail.as_deref().unwrap().ends_with("_thumbnail.webp"));
    // Placement 显示名 = 标题.扩展名；copy 源帧保留
    let placement = with_lib(&h, |l| l.folders.items.iter().find(|p| p.asset == meta.id).unwrap().clone());
    assert_eq!(placement.name, "走路.png");
    assert!(h._dir.path().join("导出/走路001.png").exists());
  }

  /// 缺帧容忍与打包判定：缺 1 帧仍打包；同目录 <3 帧不成组、逐文件导入
  #[test]
  fn pack_tolerates_small_gap_and_ignores_short_groups() {
    let h = make_lib();
    let src = make_sequence(&h, "缺帧", "跳", &[1, 2, 4, 5]); // 跨度 5 缺 1 → 成组
    let short = make_sequence(&h, "短组", "跑", &[1, 2]); // <3 → 不成组

    let (out, _) = run(&h, pack_opts(vec![src, short]));
    assert_eq!(out.imported, 3, "1 组 + 2 散文件：{:?}", out.failed);
    let meta = with_lib(&h, |l| l.index.assets.values().find(|a| a.frame_count.is_some()).unwrap().clone());
    assert_eq!(meta.frame_count, Some(4));
    assert_eq!(with_lib(&h, |l| l.index.assets.values().filter(|a| a.frame_count.is_none()).count()), 2);
  }

  /// 剪切来源对整组生效：打包后全部源帧移走
  #[test]
  fn pack_move_removes_all_source_frames() {
    let h = make_lib();
    let src = make_sequence(&h, "剪切组", "走", &[1, 2, 3]);
    let mut o = pack_opts(vec![src.clone()]);
    o.source = Source::Move;
    let (out, _) = run(&h, o);
    assert_eq!(out.imported, 1);
    for n in 1..=3 {
      assert!(!src.join(format!("走{n:03}.png")).exists(), "剪切后源帧应消失");
    }
  }

  /// 撞组 hash：整组挂起一个冲突，显示名用序列帧标题，源帧保留待决策
  #[test]
  fn pack_hash_conflict_suspends_whole_group() {
    let h = make_lib();
    let src = make_sequence(&h, "组", "跑", &[1, 2, 3]);
    let (out1, _) = run(&h, pack_opts(vec![src.clone()]));
    assert_eq!(out1.imported, 1);

    let (out2, _) = run(&h, pack_opts(vec![src.clone()]));
    assert_eq!((out2.imported, out2.conflicts.len()), (0, 1));
    let c = &out2.conflicts[0];
    assert_eq!(c.source_name, "跑.png", "冲突显示名 = 序列帧标题");
    let seq = c.sequence.as_ref().expect("冲突项应携带组信息");
    assert_eq!((seq.name.as_str(), seq.width, seq.frames.len()), ("跑", 3, 3));
    assert!(src.join("跑001.png").exists(), "冲突未决策源帧保留");
  }

  /// keep 层级 + 打包：组落目录链末级文件夹
  #[test]
  fn pack_keep_lands_group_in_chain_leaf() {
    let h = make_lib();
    let tree = h._dir.path().join("参考");
    let src = tree.join("走路");
    fs::create_dir_all(&src).unwrap();
    for n in 1..=3u8 {
      image::RgbaImage::from_pixel(8, 8, image::Rgba([n, 0, 0, 255])).save(src.join(format!("帧{n}.png"))).unwrap();
    }
    let mut o = pack_opts(vec![tree]);
    o.structure = Structure::Keep;
    let (out, _) = run(&h, o);
    assert_eq!(out.imported, 1);
    with_lib(&h, |l| {
      let p = &l.folders.items[0];
      let folder = &l.index.folders[&p.folder];
      assert_eq!(folder.name, "走路", "组应落目录链末级");
    });
  }

  /// 分散模式：序列帧不打包，逐文件导入（与现状一致）
  #[test]
  fn scatter_imports_frames_individually() {
    let h = make_lib();
    let src = make_sequence(&h, "散", "走", &[1, 2, 3]);
    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![src]));
    assert_eq!(out.imported, 3);
    assert!(with_lib(&h, |l| l.index.assets.values().all(|a| a.frame_count.is_none())));
  }

  /// 动图导入挂钩（2026-07-26 定案）：单元注册完成且资产为动图 → 提交解包队列，
  /// 逐帧高清图持久化 info/frames/ + frames.json（全局队列 cfg(test) 只落盘不发事件）
  #[test]
  fn anim_import_enqueues_unpack_to_info_frames() {
    let h = make_lib();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/anim3.gif");
    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![src]));
    assert_eq!(out.imported, 1, "导入失败：{:?}", out.failed);
    let meta = with_lib(&h, |l| l.index.assets.values().find(|a| a.ext == "gif").unwrap().clone());
    assert_eq!(meta.frame_count, Some(3), "probe 应写 frameCount");

    let info = paths::info_dir(&h.assets_dir, &meta.id);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    // frames.json 在帧图之后写，一起等防竞态
    while !((1..=3).all(|n| info.join("frames").join(format!("{}-{n:05}.webp", meta.name)).is_file())
      && info.join("frames.json").is_file())
    {
      assert!(std::time::Instant::now() < deadline, "等待超时：动图解包落盘");
      std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let delays = crate::services::anim_unpack::read_delays(&info).unwrap();
    assert_eq!(delays, [200, 200, 200]);
  }
