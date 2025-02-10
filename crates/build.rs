use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

const RAPIDSNARK_DOWNLOAD_SCRIPT: &str = include_str!("./download_rapidsnark.sh");

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let lib_dir = Path::new(&out_dir)
        .join("rapidsnark")
        .join("package")
        .join("lib");

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
            panic!("Failed to spawn rapidsnark download: {}", e);
        }
        let status = child_process.unwrap().wait();
        if let Err(e) = status {
            panic!("Failed to wait for rapidsnark download: {}", e);
        } else if !status.unwrap().success() {
            panic!("Failed to wait for rapidsnark download");
        }
    }

    println!("Detected target: {}", target);
    //For possible options see rapidsnark/build_gmp.sh
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

    let gmp_lib_folder = match target.as_str() {
        "aarch64-apple-ios" => "package_ios_arm64",
        "aarch64-apple-ios-sim" => "package_iphone_simulator_arm64",
        "x86_64-apple-ios" => "package_iphone_simulator_x86_64",
        "x86_64-linux-android" => "package_android_x86_64",
        "i686-linux-android" => "package_android_x86_64",
        "armv7-linux-androideabi" => "package_android_arm64",
        "aarch64-linux-android" => "package_android_arm64",
        _ => "package",
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

    // If the gmp library is not built, build it
    let gmp_dir = rapidsnark_path.join("depends").join("gmp");
    let target_dir = gmp_dir.join(gmp_lib_folder);
    if !target_dir.exists() {
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

    let compiler = cc::Build::new().get_compiler();
    let cpp_stdlib = if compiler.is_like_clang() {
        "c++"
    } else {
        "stdc++"
    };

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

    // Specify the path to the rapidsnark library for the linker
    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_string_lossy()
    );

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
