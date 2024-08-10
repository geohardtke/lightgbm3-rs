use cmake::Config;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let lgbm_root = Path::new(&out_dir).join("lightgbm");

    // copy source code
    if !lgbm_root.exists() {
        let status = if target.contains("windows") {
            Command::new("cmd")
                .args(&[
                    "/C",
                    "echo D | xcopy /S /Y lightgbm",
                    lgbm_root.to_str().unwrap(),
                ])
                .status()
        } else {
            Command::new("cp")
                .args(&["-r", "lightgbm", lgbm_root.to_str().unwrap()])
                .status()
        };
        if let Some(err) = status.err() {
            panic!(
                "Failed to copy ./lightgbm to {}: {}",
                lgbm_root.display(),
                err
            );
        }
    }

    // CMake
    let mut cfg = Config::new(&lgbm_root);
    let cfg = cfg
        .profile("Release")
        .uses_cxx11()
        .cxxflag("-std=c++11")
        .define("BUILD_STATIC_LIB", "ON");
    #[cfg(not(feature = "openmp"))]
    let cfg = cfg.define("USE_OPENMP", "OFF");
    #[cfg(feature = "gpu")]
    let cfg = cfg.define("USE_GPU", "1");
    #[cfg(feature = "cuda")]
    let cfg = cfg.define("USE_CUDA", "1");
    let dst = cfg.build();

    // bindgen build
    let bindings = bindgen::Builder::default()
        .header("lightgbm/include/LightGBM/c_api.h")
        .allowlist_file("lightgbm/include/LightGBM/c_api.h")
        .clang_args(&["-x", "c++", "-std=c++11"])
        .clang_arg(format!("-I{}", lgbm_root.join("include").display()))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap_or_else(|err| panic!("Couldn't write bindings: {err}"));
    // link to appropriate C++ lib
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }
    #[cfg(feature = "openmp")]
    {
        println!("cargo:rustc-link-args=-fopenmp");
        if target.contains("apple") {
            println!("cargo:rustc-link-lib=dylib=omp");
            // Link to libomp
            // If it fails to compile in MacOS, try:
            // `brew install libomp`
            // `brew link --force libomp`
            #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
            println!("cargo:rustc-link-search=/usr/local/opt/libomp/lib");
            #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
            println!("cargo:rustc-link-search=/opt/homebrew/opt/libomp/lib");
        } else if target.contains("linux") {
            println!("cargo:rustc-link-lib=dylib=gomp");
        }
    }
    println!("cargo:rustc-link-search={}", out_path.join("lib").display());
    println!("cargo:rustc-link-search=native={}", dst.display());
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=static=lib_lightgbm");
    } else {
        println!("cargo:rustc-link-lib=static=_lightgbm");
    }
}
