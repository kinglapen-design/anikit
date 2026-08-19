use std::{env, fs, path::PathBuf};

/// 把 vendor/ffmpeg/bin/*.dll 拷到 profile 输出目录（target/<profile>/），
/// 保证 exe 隐式加载 av*.dll 命中；幂等，cargo clean 后自动恢复。
fn main() {
    let ffmpeg_dir = env::var("FFMPEG_DIR").expect("FFMPEG_DIR 未设置（见 .cargo/config.toml）");
    let bin = PathBuf::from(&ffmpeg_dir).join("bin");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out → 上三级 = target/<profile>
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR 结构异常")
        .to_path_buf();
    for entry in fs::read_dir(&bin).expect("vendor/ffmpeg/bin 不存在") {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) == Some("dll") {
            let dest = profile_dir.join(p.file_name().unwrap());
            fs::copy(&p, &dest).ok();
        }
    }
    println!("cargo:rerun-if-changed={}", bin.display());
}
