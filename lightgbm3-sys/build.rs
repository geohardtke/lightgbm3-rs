use cmake::Config;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};
use bindgen::callbacks::ParseCallbacks;

#[derive(Debug)]
struct DoxygenCallback;

impl ParseCallbacks for DoxygenCallback {
    fn process_comment(&self, comment: &str) -> Option<String> {
        Some(doxygen_rs::transform(comment))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = env::var("TARGET")?.to_lowercase();
    let out_dir: PathBuf = env::var("BUILD_DIR")?.into();
    let lgbm_root = out_dir.join("lightgbm");

    // Copy source if needed
    if !lgbm_root.join("CMakeLists.txt").exists() {
        std::fs::create_dir_all(&lgbm_root)?;
        std::fs::copy("lightgbm/CMakeLists.txt", lgbm_root.join("CMakeLists.txt"))?;
        std::fs::copy("lightgbm/src", lgbm_root.join("src"))?;
        std::fs::copy("lightgbm/include", lgbm_root.join("include"))?;
    }

    // Configure CMake
    let mut cfg = Config::new(&lgbm_root)
        .profile("Release")
        .define("BUILD_STATIC_LIB", "ON")
        .define("CMAKE_CXX_COMPILER", "c++")
        .define("CMAKE_C_COMPILER", "cc");

    #[cfg(not(feature = "openmp"))]
    let cfg = cfg.define("USE_OPENMP", "OFF");

    #[cfg(feature = "gpu")]
    let cfg = cfg.define("USE_GPU", "ON");

    #[cfg(feature = "cuda")]
    let cfg = cfg.define("USE_CUDA", "ON");

    let dst = cfg.build()?;

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header(dst.join("include/LightGBM/c_api.h"))
        .allowlist_file("lightgbm/include/LightGBM/c_api.h")
        .parse_callbacks(Box::new(DoxygenCallback))
        .generate()?;

    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    // Link configuration
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }

    println!("cargo:rustc-link-search={}", out_path.join("lib").display());
    println!("cargo:rustc-link-search=native={}", dst.display());

    if target.contains("windows") {
        println!("cargo:rustc-link-lib=static=lib_lightgbm");
    } else {
        println!("cargo:rustc-link-lib=static=_lightgbm");
    }

    Ok(())
}
