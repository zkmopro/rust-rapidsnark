use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RAPIDSNARK_DOWNLOAD_SCRIPT: &str = include_str!("./download_rapidsnark.sh");

/// Directory of prebuilt libraries to link instead of downloading any.
const LIB_DIR_ENV: &str = "RAPIDSNARK_LIB_DIR";
/// Release URL to download from, for mirrors and caches.
const BASE_URL_ENV: &str = "RAPIDSNARK_DOWNLOAD_BASE_URL";

fn main() {
    println!("cargo:rerun-if-env-changed={LIB_DIR_ENV}");
    println!("cargo:rerun-if-env-changed={BASE_URL_ENV}");

    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");

    // See: https://github.com/zkmopro/chkstk_stub
    chkstk_stub::build();

    let lib_path = match env::var_os(LIB_DIR_ENV) {
        Some(dir) => PathBuf::from(dir),
        None => download_prebuilt(&out_dir, &target),
    };

    let compiler = cc::Build::new().get_compiler();
    let cpp_stdlib = if compiler.is_like_clang() {
        "c++"
    } else {
        "stdc++"
    };

    println!("cargo:rustc-link-search=native={}", lib_path.display());

    println!("cargo:rustc-link-lib=static=rapidsnark");
    println!("cargo:rustc-link-lib={cpp_stdlib}");
    if target.contains("android") {
        // pthread is included in libc in android
        println!("cargo:rustc-link-lib=c");
    } else {
        println!("cargo:rustc-link-lib=pthread");
    }
    println!("cargo:rustc-link-lib=static=fr");
    println!("cargo:rustc-link-lib=static=fq");
    println!("cargo:rustc-link-lib=static=gmp");

    if !(env::var("CARGO_CFG_TARGET_OS").unwrap().contains("ios")
        || env::var("CARGO_CFG_TARGET_OS").unwrap().contains("android"))
    {
        println!("cargo:rustc-link-lib=dylib=rapidsnark");
        println!("cargo:rustc-link-lib=dylib=fr");
        println!("cargo:rustc-link-lib=dylib=fq");
        println!("cargo:rustc-link-lib=dylib=gmp");
    }
}

/// Populate `$OUT_DIR/rapidsnark/<target>` and return it. The script exits early
/// when the libraries are already there.
fn download_prebuilt(out_dir: &str, target: &str) -> PathBuf {
    let lib_path = Path::new(out_dir).join("rapidsnark").join(target);

    let script_path = Path::new(out_dir).join("download_rapidsnark.sh");
    fs::write(&script_path, RAPIDSNARK_DOWNLOAD_SCRIPT).expect("Failed to write build script");

    let status = Command::new("sh")
        .arg(&script_path)
        .status()
        .unwrap_or_else(|e| panic!("Failed to run the rapidsnark download script: {e}"));

    if !status.success() {
        panic!(
            "Failed to fetch the rapidsnark prebuilt for {target}. Set {BASE_URL_ENV} to fetch \
             from a mirror, or {LIB_DIR_ENV} to link libraries you already have."
        );
    }

    lib_path
}
