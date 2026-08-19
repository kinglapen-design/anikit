//! 阶段四 spike 最小 Tauri 壳：N 路解码线程产 320p RGBA 帧，三通道推 webview 对比。
//! 运行：cargo run -- --lanes=16 --media=../testdata/big.gif
//! 页面：index.html?lanes=16&ch=ws|invoke|proto

use decode_shell_lib::decode;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[derive(Clone, Default)]
struct Latest {
    w: u32,
    h: u32,
    seq: u64,
    rgba: Arc<Vec<u8>>,
}

struct Lane {
    slot: Mutex<Latest>,
    tx: tokio::sync::watch::Sender<u64>,
}

impl Lane {
    fn new() -> Arc<Self> {
        let (tx, _rx) = tokio::sync::watch::channel(0u64);
        Arc::new(Lane {
            slot: Mutex::new(Latest::default()),
            tx,
        })
    }
}

#[derive(Clone)]
struct Lanes(Arc<Vec<Arc<Lane>>>);

fn grab(lane: &Lane, after_seq: u64, wait_ms: u64) -> Latest {
    let t0 = std::time::Instant::now();
    loop {
        let g = lane.slot.lock().unwrap().clone();
        if g.seq > after_seq || t0.elapsed().as_millis() as u64 >= wait_ms {
            return g;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// [w u32le][h u32le][seq u64le][rgba...]
fn frame_bytes(f: &Latest) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + f.rgba.len());
    out.extend_from_slice(&f.w.to_le_bytes());
    out.extend_from_slice(&f.h.to_le_bytes());
    out.extend_from_slice(&f.seq.to_le_bytes());
    out.extend_from_slice(&f.rgba);
    out
}

#[tauri::command]
fn pull_frame(lane: usize, after_seq: u64, lanes: tauri::State<Lanes>) -> tauri::ipc::Response {
    let lane = &lanes.0[lane % lanes.0.len()];
    let f = grab(lane, after_seq, 100);
    tauri::ipc::Response::new(frame_bytes(&f))
}

fn spawn_ws(lanes: Lanes) {
    tauri::async_runtime::spawn(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:9357")
            .await
            .expect("WS 端口占用");
        while let Ok((stream, _)) = listener.accept().await {
            let lanes = lanes.clone();
            tokio::spawn(async move {
                use futures_util::{SinkExt, StreamExt};
                let mut lane_idx = 0usize;
                let ws = match tokio_tungstenite::accept_hdr_async(
                    stream,
                    |req: &tokio_tungstenite::tungstenite::handshake::server::Request,
                     resp: tokio_tungstenite::tungstenite::handshake::server::Response| {
                        lane_idx = req
                            .uri()
                            .path()
                            .trim_start_matches('/')
                            .parse()
                            .unwrap_or(0);
                        Ok(resp)
                    },
                )
                .await
                {
                    Ok(ws) => ws,
                    Err(e) => {
                        eprintln!("WS 握手失败 {e:?}");
                        return;
                    }
                };
                let lane = Arc::clone(&lanes.0[lane_idx % lanes.0.len()]);
                let mut rx = lane.tx.subscribe();
                let (mut write, mut read) = ws.split();
                let mut last = 0u64;
                loop {
                    tokio::select! {
                        _ = rx.changed() => {
                            let f = lane.slot.lock().unwrap().clone();
                            if f.seq != last {
                                last = f.seq;
                                if write.send(tokio_tungstenite::tungstenite::Message::Binary(frame_bytes(&f).into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                        msg = read.next() => {
                            match msg {
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => break,
                                _ => {}
                            }
                        }
                    }
                }
            });
        }
    });
}

fn main() {
    ffmpeg_next::init().expect("ffmpeg init");
    let arg = |key: &str| {
        std::env::args()
            .find_map(|a| a.strip_prefix(&format!("--{key}=")).map(String::from))
    };
    let lanes_n: usize = arg("lanes").and_then(|s| s.parse().ok()).unwrap_or(16);
    let pace: f32 = arg("pace").and_then(|s| s.parse().ok()).unwrap_or(0.0); // 0=全速；1=按帧 delay 原速播放
    let media = arg("media")
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../testdata/big.gif").to_string());

    let lanes = Lanes(Arc::new((0..lanes_n).map(|_| Lane::new()).collect()));
    for lane in lanes.0.iter() {
        let lane = Arc::clone(lane);
        let media = media.clone();
        std::thread::spawn(move || {
            decode::decode_loop(&media, 320, move |f| {
                if pace > 0.0 && f.delay_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_secs_f32(
                        f.delay_ms as f32 / 1000.0 * pace,
                    ));
                }
                let seq = {
                    let mut g = lane.slot.lock().unwrap();
                    let seq = g.seq + 1;
                    *g = Latest {
                        w: f.w,
                        h: f.h,
                        seq,
                        rgba: Arc::new(f.rgba),
                    };
                    seq
                };
                lane.tx.send_replace(seq);
            });
        });
    }

    let lanes_proto = lanes.clone();
    let lanes_proto2 = lanes.clone();
    tauri::Builder::default()
        .manage(lanes.clone())
        .invoke_handler(tauri::generate_handler![pull_frame])
        .register_uri_scheme_protocol("frame", move |_ctx, request| {
            let lanes = &lanes_proto2;
            let lane_idx: usize = request
                .uri()
                .path()
                .trim_start_matches('/')
                .parse()
                .unwrap_or(0);
            let after: u64 = request
                .uri()
                .query()
                .and_then(|q| q.strip_prefix("after="))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let f = grab(&lanes.0[lane_idx % lanes.0.len()], after, 100);
            tauri::http::Response::builder()
                .header("Content-Type", "application/octet-stream")
                .header("Access-Control-Allow-Origin", "*")
                .body(frame_bytes(&f))
                .unwrap()
        })
        .setup(move |_app| {
            let _ = &lanes_proto; // WS 通道共用帧槽
            spawn_ws(lanes_proto.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("tauri run");
}
