use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    if std::env::var("RUST_RAPIDSNARK_LINK_TEST_WITNESS").is_ok() {
        rust_witness::transpile::transpile_wasm("./test-vectors".to_string());
    }

    let target = std::env::var("TARGET").unwrap();
    let arch = target.split('-').next().unwrap();

    // Try to list contents of the target directory
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let rapidsnark_dir = manifest_dir.join("rapidsnark");
    let absolute_lib_path = if rapidsnark_dir.join(&target).exists() {
        rapidsnark_dir.join(&target)
    } else {
        rapidsnark_dir.join(arch)
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
    println!("cargo:rustc-link-lib={}", cpp_stdlib);
    if target.contains("android") {
        // pthread is included in libc in android
        println!("cargo:rustc-link-lib=c");
    } else {
        println!("cargo:rustc-link-lib=pthread");
    }
    println!("cargo:rustc-link-lib=static=fr");
    println!("cargo:rustc-link-lib=static=fq");
    println!("cargo:rustc-link-lib=static=gmp");

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
            "Error: Source file {:?} does not exist",
            lib_path
        );
        let dest_dir = Path::new(&output_path).join(env::var("CARGO_NDK_ANDROID_TARGET").unwrap());
        println!("cargo:rerun-if-changed={}", dest_dir.display());
        if !dest_dir.exists() {
            fs::create_dir_all(&dest_dir).unwrap();
        }
        fs::copy(lib_path, Path::new(&dest_dir).join("libc++_shared.so")).unwrap();
    }
}
