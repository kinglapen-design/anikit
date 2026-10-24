//! 收件箱文件监听适配器（PRD 05-01）：notify 增量事件（Windows 走 ReadDirectoryChangesW，
//! 非轮询、常驻零 CPU）监听 inbox 目录；按"顶层条目"（被丢入的文件 / 文件夹）聚合活动，
//! 做写入完成检测——距最后一次活动超过 STABLE（编辑器贴图用短档）、大小两轮探测一致、
//! 全部文件可移走，三者齐备才整批回调。逐条目独立判定（2026-07-22 用户定案）：被占用的条目自己等放手，
//! 不影响其他条目入库；recover 类临时文件会随之入库，由用户自删（伴生图识别归 3.7 族后置）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

/// 写入完成判定：距最后一次文件活动的静默时长（PRD"大小连续 N 秒稳定"）
const STABLE: Duration = Duration::from_secs(2);
/// 编辑器贴图（贴图-*）短静默档：同步写入的工具通常已关文件，仍保留两轮大小与可移走探测
const EDITOR_PASTE_STABLE: Duration = Duration::from_millis(500);
/// 就绪判定节拍：仅队列非空时启用（队列空则纯事件阻塞，零定时器）；
/// 每拍只对事件已上报的条目做探测（exists + 可移走 + 大小），不扫目录
const TICK: Duration = Duration::from_millis(500);

/// 监听句柄：stop 置位 + 唤醒事件 + join 回收线程；Drop 自动停（防泄漏）
pub struct WatchHandle {
  stop: Arc<AtomicBool>,
  /// 事件通道的发送端副本：队列空时线程纯阻塞在 recv（无定时器），停止需补事件唤醒
  wake: mpsc::Sender<notify::Result<Event>>,
  join: Option<JoinHandle<()>>,
}

impl WatchHandle {
  pub fn stop(&mut self) {
    self.stop.store(true, Ordering::Relaxed);
    // 空闲阻塞的线程收不到事件就不会醒，补一个无路径的空事件让它退出
    let _ = self.wake.send(Ok(Event::new(notify::event::EventKind::Other)));
    if let Some(j) = self.join.take() {
      let _ = j.join();
    }
  }
}

impl Drop for WatchHandle {
  fn drop(&mut self) {
    self.stop();
  }
}

/// notify 事件通道两端（make_watcher 产出；别名消 type_complexity）
type EventTx = mpsc::Sender<notify::Result<Event>>;
type EventRx = mpsc::Receiver<notify::Result<Event>>;

/// 建 watcher + 事件通道（spawn / spawn_changes_only 共用）
fn make_watcher(dir: &Path) -> Result<(RecommendedWatcher, EventTx, EventRx), String> {
  let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
  let mut watcher = RecommendedWatcher::new(tx.clone(), Config::default()).map_err(|e| format!("文件监听创建失败：{e}"))?;
  watcher.watch(dir, RecursiveMode::Recursive).map_err(|e| format!("文件监听失败：{e}"))?;
  Ok((watcher, tx, rx))
}

/// 启动监听：目录下每个就绪的顶层条目整批触发 on_ready。
/// 启动时先清点存量条目（上次崩溃残留 / 用户跳过项），与通知事件走同一判定。
/// on_change：任何带路径的监听事件都触发（原始路径清单透传，过滤交由调用方——收件箱不滤、
/// 工作空间滤 .anikit 缓存写回；去抖在前端），服务"工程文件"磁盘实况热刷新。
pub fn spawn(
  dir: PathBuf,
  on_ready: impl Fn(Vec<PathBuf>) + Send + 'static,
  on_change: impl Fn(&[PathBuf]) + Send + 'static,
) -> Result<WatchHandle, String> {
  let (watcher, tx, rx) = make_watcher(&dir)?;
  let stop = Arc::new(AtomicBool::new(false));
  let stop_thread = stop.clone();
  let join = std::thread::spawn(move || watch_loop(dir, rx, watcher, stop_thread, on_ready, on_change));
  Ok(WatchHandle { stop, wake: tx, join: Some(join) })
}

/// 轻量监听（工作空间实况用，2026-09-19）：**不就绪判定**——无"写入完成"语义需求时，
/// spawn 版的 pending / drain_ready 纯属白烧（有文件活动时每 TICK 对活动条目递归
/// entry_movable 逐文件 CreateFileW + entry_size 递归 stat，大目录下常驻烧 IO/CPU；
/// 退出 stop 的 join 还要等当前这趟遍历跑完——用户实机「接入工作空间后托盘退出又卡又慢」）。
/// 纯事件转发 on_change，队列空纯阻塞零定时器；stop 唤醒即退（join 秒回）。
pub fn spawn_changes_only(dir: PathBuf, on_change: impl Fn(&[PathBuf]) + Send + 'static) -> Result<WatchHandle, String> {
  let (watcher, tx, rx) = make_watcher(&dir)?;
  let stop = Arc::new(AtomicBool::new(false));
  let stop_thread = stop.clone();
  let join = std::thread::spawn(move || {
    let _watcher = watcher; // 持有保活，线程退出即停止监听
    loop {
      if stop_thread.load(Ordering::Relaxed) {
        break;
      }
      match rx.recv() {
        Ok(Ok(ev)) => {
          if !ev.paths.is_empty() {
            on_change(&ev.paths);
          }
        }
        Ok(Err(e)) => log::error!("文件监听事件异常：{e}"),
        Err(_) => break, // 发送端全 drop（watcher 已停）
      }
    }
  });
  Ok(WatchHandle { stop, wake: tx, join: Some(join) })
}

/// 待就绪条目状态：最后活动时刻 + 上一轮大小快照
struct Pending {
  last_activity: Instant,
  last_size: Option<u64>,
}

impl Pending {
  fn hot() -> Self {
    Self { last_activity: Instant::now(), last_size: None }
  }
}

fn watch_loop(
  dir: PathBuf,
  rx: mpsc::Receiver<notify::Result<Event>>,
  _watcher: RecommendedWatcher, // 持有保活，线程退出即停止监听
  stop: Arc<AtomicBool>,
  on_ready: impl Fn(Vec<PathBuf>),
  on_change: impl Fn(&[PathBuf]),
) {
  let mut pending: HashMap<PathBuf, Pending> = HashMap::new();
  // 存量清点：目录里已存在的条目按"刚有活动"入队，静默期满后照常入库
  if let Ok(entries) = std::fs::read_dir(&dir) {
    for entry in entries.filter_map(|e| e.ok()) {
      pending.insert(entry.path(), Pending::hot());
    }
  }
  loop {
    if stop.load(Ordering::Relaxed) {
      break;
    }
    // 队列空：纯阻塞等事件（零定时器，真零 CPU 常驻）；队列非空：TICK 节拍驱动就绪判定
    let msg = if pending.is_empty() {
      rx.recv().ok()
    } else {
      match rx.recv_timeout(TICK) {
        Ok(m) => Some(m),
        Err(mpsc::RecvTimeoutError::Timeout) => None,
        Err(mpsc::RecvTimeoutError::Disconnected) => break,
      }
    };
    match msg {
      Some(Ok(ev)) => {
        if !ev.paths.is_empty() {
          on_change(&ev.paths); // 磁盘实况热刷新信号（原始路径透传，调用方过滤，前端去抖）
        }
        for p in &ev.paths {
          if let Some(root) = root_entry(&dir, p) {
            pending.entry(root).and_modify(|s| s.last_activity = Instant::now()).or_insert_with(Pending::hot);
          }
        }
      }
      Some(Err(e)) => log::error!("收件箱监听事件异常：{e}"),
      None => {}
    }
    drain_ready(&mut pending, &on_ready);
  }
}

/// 顶层条目：p 相对 inbox 目录的第一级（文件本身 / 被丢入的文件夹）；
/// 目录自身与目录外路径返回 None
fn root_entry(dir: &Path, p: &Path) -> Option<PathBuf> {
  let first = p.strip_prefix(dir).ok()?.components().next()?;
  Some(dir.join(first.as_os_str()))
}

/// 编辑器贴图（同步写入、统一 `贴图-` 前缀）用短静默档；普通条目维持 2 秒。
fn stable_for(path: &Path) -> Duration {
  let is_editor_paste = path.is_file()
    && path.file_name().map(|name| name.to_string_lossy().starts_with("贴图-")).unwrap_or(false);
  if is_editor_paste { EDITOR_PASTE_STABLE } else { STABLE }
}

/// 逐条判定就绪：静默期满 + 全部文件可移走 + 大小两轮一致 → 出队回调；
/// 条目被移走 / 读不了则丢弃；被占用顺延活动时刻（自己等放手，不拖累其他条目）
fn drain_ready(pending: &mut HashMap<PathBuf, Pending>, on_ready: &impl Fn(Vec<PathBuf>)) {
  let mut ready = Vec::new();
  pending.retain(|path, st| {
    if !path.exists() {
      return false; // 写入中途被移走 / 删除，丢弃
    }
    if st.last_activity.elapsed() < stable_for(path) {
      return true;
    }
    if !entry_movable(path) {
      st.last_activity = Instant::now(); // 占用：顺延，下轮重试
      return true;
    }
    let Some(size) = entry_size(path) else { return false }; // 读不了（多半已不可用），丢弃
    if st.last_size == Some(size) {
      ready.push(path.clone());
      false
    } else {
      st.last_size = Some(size);
      true
    }
  });
  if !ready.is_empty() {
    on_ready(ready);
  }
}

/// 条目内全部文件（文件夹递归）；目录读取失败的条目跳过
pub(crate) fn files_of(path: &Path) -> Vec<PathBuf> {
  if path.is_file() {
    return vec![path.to_path_buf()];
  }
  let mut out = Vec::new();
  if let Ok(entries) = std::fs::read_dir(path) {
    for entry in entries.filter_map(|e| e.ok()) {
      out.extend(files_of(&entry.path()));
    }
  }
  out
}

/// 条目总大小；任一元数据读取失败返回 None
pub(crate) fn entry_size(path: &Path) -> Option<u64> {
  files_of(path).iter().try_fold(0u64, |acc, f| std::fs::metadata(f).ok().map(|m| acc + m.len()))
}

/// 全部文件可移走（过境语义的关键探针）：只读打开能过不代表移得动——
/// 写入方常允许读共享但拒绝 delete 共享（Animate 保存 fla 后仍持有文件即如此），
/// 此时导入 move 必败，文件会搁浅在 inbox 再无事件触发。探测口径与 store_file 的
/// rename 判定一致，没放手就留在队列里等，放手后下一拍自动入库。
pub(crate) fn entry_movable(path: &Path) -> bool {
  files_of(path).iter().all(|f| file_movable(f))
}

/// Windows：以 GENERIC_READ | DELETE 访问探测——持有方未共享读 / 删除时打开失败
/// （共享冲突），与 move（rename）实际判定同口径；我们自身放开全部共享，即开即关
#[cfg(target_os = "windows")]
fn file_movable(path: &Path) -> bool {
  use std::os::windows::ffi::OsStrExt;
  use windows::core::PCWSTR;
  use windows::Win32::Foundation::CloseHandle;
  use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
  };
  const GENERIC_READ: u32 = 0x8000_0000;
  const DELETE: u32 = 0x0001_0000;
  let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
  let handle = unsafe {
    CreateFileW(
      PCWSTR(wide.as_ptr()),
      GENERIC_READ | DELETE,
      FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0),
      None,
      OPEN_EXISTING,
      FILE_ATTRIBUTE_NORMAL,
      None,
    )
  };
  match handle {
    Ok(h) => {
      unsafe {
        let _ = CloseHandle(h);
      }
      true
    }
    Err(_) => false,
  }
}

/// 非 Windows：以读写打开近似（POSIX 无 delete 共享语义，rename 不受持有影响）
#[cfg(not(target_os = "windows"))]
fn file_movable(path: &Path) -> bool {
  std::fs::OpenOptions::new().read(true).write(true).open(path).is_ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn root_entry_maps_to_top_level() {
    let dir = Path::new("inbox");
    assert_eq!(root_entry(dir, Path::new("inbox/a.png")), Some(dir.join("a.png")));
    assert_eq!(root_entry(dir, Path::new("inbox/夹/b.png")), Some(dir.join("夹")));
    assert_eq!(root_entry(dir, Path::new("inbox")), None);
    assert_eq!(root_entry(dir, Path::new("elsewhere/a.png")), None);
  }

  #[test]
  fn entry_size_sums_tree_recursively() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("夹");
    std::fs::create_dir_all(dir.join("子")).unwrap();
    std::fs::write(dir.join("a.txt"), "1234").unwrap();
    std::fs::write(dir.join("子/b.txt"), "12").unwrap();
    assert_eq!(entry_size(&dir), Some(6));
    assert_eq!(entry_size(Path::new("不存在")), Some(0)); // 读不了的目录按空树计
  }

  /// 编辑器贴图在 2 秒静默未满时也可就绪，但仍须两轮大小一致；同批普通文件不放行
  #[test]
  fn editor_paste_uses_short_stable_and_keeps_two_probes() {
    let tmp = tempfile::tempdir().unwrap();
    let paste = tmp.path().join("贴图-20261025-test.png");
    let normal = tmp.path().join("普通.png");
    std::fs::write(&paste, "内容").unwrap();
    std::fs::write(&normal, "内容").unwrap();
    let mut pending = HashMap::new();
    pending.insert(paste.clone(), Pending { last_activity: Instant::now() - Duration::from_millis(600), last_size: None });
    pending.insert(normal.clone(), Pending::hot());
    let (tx, rx) = mpsc::channel();

    drain_ready(&mut pending, &|paths| {
      let _ = tx.send(paths);
    });
    assert!(rx.try_recv().is_err(), "贴图首轮只记录大小，不能立即就绪");
    assert!(pending.contains_key(&paste));

    drain_ready(&mut pending, &|paths| {
      let _ = tx.send(paths);
    });
    let got = rx.recv().unwrap();
    assert_eq!(got, vec![paste.clone()]);
    assert!(!pending.contains_key(&paste));
    assert!(pending.contains_key(&normal), "普通文件仍须等满 2 秒静默");
  }

  /// 真监听集成：写入文件 → 静默期满 → 回调就绪（STABLE + 两轮探测 ≈ 3s，给足余量）
  #[test]
  fn watcher_reports_settled_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("inbox");
    std::fs::create_dir_all(&dir).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut handle = spawn(
      dir.clone(),
      move |paths| {
        let _ = tx.send(paths);
      },
      |_| {},
    )
    .unwrap();

    let file = dir.join("图.png");
    std::fs::write(&file, "内容").unwrap();
    let got = rx.recv_timeout(Duration::from_secs(15)).expect("15 秒内应收到就绪回调");
    assert!(got.contains(&file), "就绪批次应包含写入的文件：{got:?}");

    handle.stop();
  }

  /// 真监听集成：编辑器贴图走短静默档，应在普通文件 2 秒静默期满前就绪
  #[test]
  fn watcher_reports_editor_paste_before_normal_stable() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("inbox");
    std::fs::create_dir_all(&dir).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut handle = spawn(
      dir.clone(),
      move |paths| {
        let _ = tx.send(paths);
      },
      |_| {},
    )
    .unwrap();

    let file = dir.join("贴图-20261025-test.png");
    let started = Instant::now();
    std::fs::write(&file, "内容").unwrap();
    let got = rx.recv_timeout(Duration::from_secs(5)).expect("5 秒内应收到贴图就绪回调");
    assert!(got.contains(&file), "就绪批次应包含贴图：{got:?}");
    assert!(started.elapsed() < STABLE, "贴图就绪超过普通静默档：{:?}", started.elapsed());
    handle.stop();
  }

  /// 空闲（队列空、纯阻塞等事件）时停止应立即返回——唤醒事件机制兜底
  #[test]
  fn stop_while_idle_returns_promptly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("inbox");
    std::fs::create_dir_all(&dir).unwrap();
    let mut handle = spawn(dir, |_| {}, |_| {}).unwrap();
    std::thread::sleep(Duration::from_millis(200)); // 让线程先进入空闲阻塞
    let t0 = Instant::now();
    handle.stop();
    assert!(t0.elapsed() < Duration::from_secs(2), "空闲时停止应立即返回：{:?}", t0.elapsed());
  }

  /// 轻量监听（工作空间实况）：文件事件纯转发（不就绪判定不遍历），stop 立即返回
  ///（2026-09-19「接入工作空间后托盘退出又卡又慢」修复）
  #[test]
  fn changes_only_forwards_events_and_stops_promptly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("ws");
    std::fs::create_dir_all(&dir).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut handle = spawn_changes_only(dir.clone(), move |paths| {
      for p in paths {
        let _ = tx.send(p.clone());
      }
    })
    .unwrap();
    std::thread::sleep(Duration::from_millis(300)); // 让监听就位
    let file = dir.join("a.txt");
    std::fs::write(&file, "x").unwrap();
    let hit = (0..10).any(|_| matches!(rx.recv_timeout(Duration::from_secs(2)), Ok(p) if p.ends_with("a.txt")));
    assert!(hit, "应收到 a.txt 变更事件（纯转发不就绪判定）");
    let t0 = Instant::now();
    handle.stop();
    assert!(t0.elapsed() < Duration::from_secs(2), "轻量监听停止应立即返回：{:?}", t0.elapsed());
  }

  /// 逐条目独立（用户定案）：被占用的条目自己等放手；不被占用的条目照常入库，不被拖累
  #[cfg(target_os = "windows")]
  #[test]
  fn free_entry_imports_while_another_is_occupied() {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
      CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_MODE, FILE_SHARE_READ, OPEN_EXISTING,
    };
    const GENERIC_READ_WRITE: u32 = 0xC000_0000;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("inbox");
    std::fs::create_dir_all(&dir).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut handle = spawn(
      dir.clone(),
      move |paths| {
        let _ = tx.send(paths);
      },
      |_| {},
    )
    .unwrap();

    let locked = dir.join("占用.fla");
    let free = dir.join("自由.png");
    std::fs::write(&locked, "x").unwrap();
    std::fs::write(&free, "y").unwrap();

    // 模拟编辑软件持有（只给读共享，拒绝 delete 共享）
    let wide: Vec<u16> = locked.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let hold = unsafe {
      CreateFileW(
        PCWSTR(wide.as_ptr()),
        GENERIC_READ_WRITE,
        FILE_SHARE_MODE(FILE_SHARE_READ.0),
        None,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        None,
      )
      .unwrap()
    };
    // 自由文件照常入库，且批次不含被占用文件
    let got = rx.recv_timeout(Duration::from_secs(15)).expect("自由文件应照常入库");
    assert!(got.contains(&free), "批次应包含自由文件：{got:?}");
    assert!(!got.contains(&locked), "批次不应包含被占用文件：{got:?}");
    // 占用未放手：不应再有回调
    assert!(rx.recv_timeout(Duration::from_secs(4)).is_err(), "占用期间被占用文件不应入库");

    unsafe {
      let _ = CloseHandle(hold);
    }
    // 放手后被占用文件入库
    let got2 = rx.recv_timeout(Duration::from_secs(15)).expect("放手后应收到就绪回调");
    assert!(got2.contains(&locked), "放手后被占用文件应入库：{got2:?}");
    handle.stop();
  }

  /// 占用探测：持有方只给读共享、拒绝 delete 共享时（Animate 保存后持有 fla 的场景）
  /// 判定不可移走；句柄释放后判定可移走
  #[cfg(target_os = "windows")]
  #[test]
  fn file_movable_detects_delete_share_lock() {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
      CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_MODE, FILE_SHARE_READ, OPEN_EXISTING,
    };
    const GENERIC_READ_WRITE: u32 = 0xC000_0000;

    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("工程.fla");
    std::fs::write(&file, "x").unwrap();

    let wide: Vec<u16> = file.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let hold = unsafe {
      CreateFileW(
        PCWSTR(wide.as_ptr()),
        GENERIC_READ_WRITE,
        FILE_SHARE_MODE(FILE_SHARE_READ.0), // 只共享读，不共享写 / 删除
        None,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        None,
      )
      .unwrap()
    };
    assert!(!file_movable(&file), "delete 共享被拒时应判定不可移走");
    unsafe {
      let _ = CloseHandle(hold);
    }
    assert!(file_movable(&file), "句柄释放后应判定可移走");
  }
}
