use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

const RAPIDSNARK_DOWNLOAD_SCRIPT: &str = include_str!("./download_rapidsnark.sh");

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let arch = target.split('-').next().unwrap();

    // See: https://github.com/zkmopro/chkstk_stub
    chkstk_stub::build();

    // Try to list contents of the target directory
    let rapidsnark_path = Path::new(&out_dir).join(Path::new("rapidsnark"));
    // If the rapidsnark repo is not downloaded, download it
    if !rapidsnark_path.exists() {
        let rapidsnark_script_path = Path::new(&out_dir).join(Path::new("download_rapidsnark.sh"));
        fs::write(&rapidsnark_script_path, RAPIDSNARK_DOWNLOAD_SCRIPT)
            .expect("Failed to write build script");
        let child_process = Command::new("sh")
            .arg(rapidsnark_script_path.to_str().unwrap())
            .spawn();
        if let Err(e) = child_process {
            panic!("Failed to spawn rapidsnark download: {e}");
        }
        let status = child_process.unwrap().wait();
        if let Err(e) = status {
            panic!("Failed to wait for rapidsnark download: {e}");
        } else if !status.unwrap().success() {
            panic!("Failed to wait for rapidsnark download");
        }
    }
    let absolute_lib_path = if rapidsnark_path.join(&target).exists() {
        rapidsnark_path.join(&target)
    } else {
        rapidsnark_path.join(arch)
    };
    let compiler = cc::Build::new().get_compiler();
    let cpp_stdlib = if compiler.is_like_clang() {
        "c++"
    } else {
        "stdc++"
    };

    println!(
        "cargo:rustc-link-search=native={}",
        absolute_lib_path.clone().display()
    );

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

    // refer to https://github.com/bbqsrc/cargo-ndk to see how to link the libc++_shared.so file in Android
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        android();
    }
}

fn android() {
    println!("cargo:rustc-link-lib=c++_shared");

    if let Ok(output_path) = env::var("CARGO_NDK_OUTPUT_PATH") {
        let sysroot_libs_path = PathBuf::from(env::var_os("CARGO_NDK_SYSROOT_LIBS_PATH").unwrap());
        let lib_path = sysroot_libs_path.join("libc++_shared.so");
        assert!(
            lib_path.exists(),
            "Error: Source file {lib_path:?} does not exist"
        );
        let dest_dir = Path::new(&output_path).join(env::var("CARGO_NDK_ANDROID_TARGET").unwrap());
        println!("cargo:rerun-if-changed={dest_dir:?}");
        if !dest_dir.exists() {
            fs::create_dir_all(&dest_dir).unwrap();
        }
        fs::copy(lib_path, Path::new(&dest_dir).join("libc++_shared.so")).unwrap();
    }
}
