use std::{env, fs, path::PathBuf};

fn main() {
    tauri_build::build();

    // 拷 vendor ffmpeg dll 到 profile 输出目录（exe 隐式加载 av*.dll 命中；
    // 幂等，cargo clean 后自动恢复）——与上级 decode-proto/build.rs 同逻辑。
    if let Ok(ffmpeg_dir) = env::var("FFMPEG_DIR") {
        let bin = PathBuf::from(&ffmpeg_dir).join("bin");
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let profile_dir = out_dir.ancestors().nth(3).unwrap().to_path_buf();
        for entry in fs::read_dir(&bin).expect("vendor/ffmpeg/bin 不存在") {
            let p = entry.unwrap().path();
            if p.extension().and_then(|e| e.to_str()) == Some("dll") {
                fs::copy(&p, profile_dir.join(p.file_name().unwrap())).ok();
            }
        }
        println!("cargo:rerun-if-changed={}", bin.display());
    }
}
