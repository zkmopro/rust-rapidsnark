use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

const CLONE_RAPIDSNARK_SCRIPT: &str = include_str!("./clone_rapidsnark.sh");

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    if !dest.exists() {
        fs::create_dir_all(dest)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

fn main() {
    if std::env::var("RUST_RAPIDSNARK_LINK_TEST_WITNESS").is_ok() {
        rust_witness::transpile::transpile_wasm("./test-vectors".to_string());
    }

    let target = std::env::var("TARGET").unwrap();
    let arch = target.split('-').next().unwrap();

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let lib_dir = Path::new(&out_dir)
        .join("rapidsnark")
        .join("package")
        .join("lib");

    let rapidsnark_path = Path::new(&out_dir).join(Path::new("rapidsnark"));
    // If the rapidsnark repo is not cloned, clone it
    if !rapidsnark_path.exists() {
        let clone_script_path = Path::new(&out_dir).join(Path::new("clone_rapidsnark.sh"));
        fs::write(&clone_script_path, CLONE_RAPIDSNARK_SCRIPT)
            .expect("Failed to write build script");
        Command::new("sh")
            .arg(clone_script_path.to_str().unwrap())
            .spawn()
            .expect("Failed to spawn rapidsnark build")
            .wait()
            .expect("rapidsnark build errored");
    }

    println!("Detected target: {}", target);
    //For possible options see witnesscalc/build_gmp.sh
    let gmp_build_target = match target.as_str() {
        "aarch64-apple-ios" => "ios",
        "aarch64-apple-ios-sim" => "ios_simulator",
        "x86_64-apple-ios" => "ios_simulator",
        "x86_64-linux-android" => "android_x86_64",
        "i686-linux-android" => "android_x86_64",
        "armv7-linux-androideabi" => "android",
        "aarch64-linux-android" => "android",
        "aarch64-apple-darwin" => "host", //Use "host" for M Macs, macos_arm64 would fail the subsequent build
        _ => "host",
    };

    //For possible options see rapidsnark/Makefile
    let rapidsnark_build_target = match target.as_str() {
        "aarch64-apple-ios" => "ios",
        "aarch64-apple-ios-sim" => "ios_simulator_arm64",
        "x86_64-apple-ios" => "ios_simulator_x86_64",
        "x86_64-linux-android" => "android_x86_64",
        "i686-linux-android" => "android_x86_64",
        "armv7-linux-androideabi" => "android",
        "aarch64-linux-android" => "android",
        "aarch64-apple-darwin" => "arm64_host",
        _ => "host",
    };

    // If the rapidsnark library is not built, build it
    let gmp_dir = rapidsnark_path.join("depends").join("gmp");
    if !gmp_dir.exists() {
        Command::new("bash")
            .current_dir(&rapidsnark_path)
            .arg("./build_gmp.sh")
            .arg(gmp_build_target)
            .spawn()
            .expect("Failed to spawn build_gmp.sh")
            .wait()
            .expect("build_gmp.sh errored");
    }

    Command::new("make")
        .arg(rapidsnark_build_target)
        .current_dir(&rapidsnark_path)
        .spawn()
        .expect("Failed to spawn make arm64_host")
        .wait()
        .expect("make arm64_host errored");

    // Copy lib_dir to the current directory
    let lib_dir = PathBuf::from(lib_dir);
    let current_dir = std::env::current_dir().unwrap();
    let lib_dir_name = lib_dir.file_name().unwrap();
    let new_lib_dir = current_dir.join(lib_dir_name);
    println!("new_lib_dir: {}", new_lib_dir.to_string_lossy());
    if !new_lib_dir.exists() {
        std::fs::create_dir_all(&new_lib_dir).unwrap();
    }
    if lib_dir.is_dir() {
        copy_dir_recursive(&lib_dir, &new_lib_dir).unwrap();
    } else {
        fs::copy(&lib_dir, &new_lib_dir).unwrap();
    }

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

    // println!(
    //     "cargo:rustc-link-search=native={}",
    //     absolute_lib_path.clone().display()
    // );

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

    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_string_lossy()
    );
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
