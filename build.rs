use std::path::PathBuf;

fn main() {
    // #[cfg(test)]
    rust_witness::transpile::transpile_wasm("./test-vectors".to_string());

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
}
