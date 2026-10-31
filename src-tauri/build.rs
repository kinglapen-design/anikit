use std::{env, fs, path::PathBuf};

fn main() {
  tauri_build::build();

  // lib 单测由 #[link(name="resource")] 引入同一份 Windows 资源；正式 bins 仍由 tauri-winres 链接。
  if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
    println!("cargo:rustc-link-search=native={}", env::var("OUT_DIR").unwrap());
  }

  // 阶段四：拷 vendor ffmpeg dll 到 profile 输出目录（exe/测试隐式加载 av*.dll 命中；
  // 幂等，cargo clean 后自动恢复；release 分发随 bundle 另行携带）
  if let Ok(ffmpeg_dir) = env::var("FFMPEG_DIR") {
    let bin = PathBuf::from(&ffmpeg_dir).join("bin");
    if bin.is_dir() {
      let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
      let profile_dir = out_dir.ancestors().nth(3).unwrap().to_path_buf();
      for entry in fs::read_dir(&bin).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) == Some("dll") {
          fs::copy(&p, profile_dir.join(p.file_name().unwrap())).ok();
        }
      }
      println!("cargo:rerun-if-changed={}", bin.display());
    }
  }
}
