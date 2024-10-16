use cmake::Config;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};
use std::fs;

#[derive(Debug)]
struct DoxygenCallback;

impl bindgen::callbacks::ParseCallbacks for DoxygenCallback {
    fn process_comment(&self, comment: &str) -> Option<String> {
        Some(doxygen_rs::transform(comment))
    }
}

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let path = env::current_dir().unwrap();
    println!("We are in: {:?}",  path);
    
    // Read the directory contents and unwrap it
    let entries = fs::read_dir(&path).unwrap();
    println!("Files in the current directory:");
    for entry in entries {
        let entry = entry.unwrap();
        println!("{:?}", entry.path());
    }

        let git_version = Command::new("git")
        .arg("--version")
        .output()
        .expect("Failed to execute git command");

    if git_version.status.success() {
        println!("cargo:warning=Git found. Initializing submodules...");

        // Execute the git submodule update command
        let output = Command::new("git")
            .args(&["submodule", "update", "--init", "--recursive"])
            .output()
            .expect("Failed to update git submodules");

        if output.status.success() {
            println!("cargo:warning=Git submodule updated successfully.");
        } else {
            eprintln!("cargo:warning=Failed to update git submodules: {}", String::from_utf8_lossy(&output.stderr));
        }
    } else {
        println!("cargo:warning=Git is not available in the environment.");
    }

   // Read the directory contents and unwrap it
    let entries = fs::read_dir(&format!("{}/lightgbm",path.display())).unwrap();
    println!("Files in the lightgbm directory:"); // did it pull the submodule?
    for entry in entries {
        let entry = entry.unwrap();
        println!("{:?}", entry.path());
    }
    
    
    println!("Using OUT_DIR: {}", out_dir);
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
        .parse_callbacks(Box::new(DoxygenCallback))
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
