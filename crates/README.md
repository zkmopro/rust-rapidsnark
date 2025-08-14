# Rust Rapidsnark

[![Crates.io](https://img.shields.io/crates/v/rust-rapidsnark?label=rust-rapidsnark)](https://crates.io/crates/rust-rapidsnark)

This project provides a Rust adapter for compiling and linking [Rapidsnark](https://github.com/iden3/rapidsnark) into a native library for target platforms (e.g., mobile devices). It includes macros and functions to facilitate the integration of proof generation into Rust codebases.

## Requirements

### Rust toolchain

```
cargo 1.89.0 (c24e10642 2025-06-23)
```

## Usage

Include the crate in your `Cargo.toml`:

```toml
[dependencies]
rust-rapidsnark = "0.1"

[build-dependencies]
rust-rapidsnark = "0.1"
```

It doesn't include the witness generation functions, you need to use one of the following crates to generate the witness:

-   [rust-witness](https://github.com/chancehudson/rust-witness)
-   [witnesscalc-adapter](https://github.com/zkmopro/witnesscalc_adapter)
-   [circom-witnesscalc](https://github.com/iden3/circom-witnesscalc)
-   [wasmer](https://github.com/wasmerio/wasmer)

For example, building witness with `witnesscalc-adapter`:

```rust
witnesscalc_adapter::witness!(multiplier2);
let json_input_string = "{\"a\": [\"2\"], \"b\": [\"3\"]}";;
let wtns_buffer = multiplier2_witness(json_input_string).unwrap();
```

### Calculate the proof

Calculate the proof by using the `groth16_prover_zkey_file_wrapper` function.
It will take a `wtns` bytes array like the output of [witnesscalc](https://github.com/0xPolygonID/witnesscalc) or [snarkjs](https://github.com/iden3/snarkjs).

```rust
let zkey_path = "./test-vectors/multiplier2_final.zkey";
let proof = rust_rapidsnark::groth16_prover_zkey_file_wrapper(zkey_path, wtns_buffer).unwrap();
```

### Verify the proof

Verify the proof by using the `groth16_verifier_zkey_file_wrapper` function.

```rust
let vkey = std::fs::read_to_string("./test-vectors/keccak256_256_test.vkey.json")?;
let valid = rust_rapidsnark::groth16_verify_wrapper(
    &proof.proof,
    &proof.public_signals,
    &vkey,
)?;
```

## Supported platforms

### Linux

-   x86_64 linux
-   arm64 linux

### MacOS

-   aarch64-apple-darwin
-   x86_64-apple-darwin

### iOS

-   aarch64-apple-ios
-   aarch64-apple-ios-sim
-   x86_64-apple-ios

### Android

-   aarch64-linux-android
-   x86_64-linux-android

## Community

-   Website: [zkmopro.com](https://zkmopro.com)
-   X account: <a href="https://twitter.com/zkmopro"><img src="https://img.shields.io/twitter/follow/zkmopro?style=flat-square&logo=x&label=zkmopro"></a>
-   Telegram group: <a href="https://t.me/zkmopro"><img src="https://img.shields.io/badge/telegram-@zkmopro-blue.svg?style=flat-square&logo=telegram"></a>

## Acknowledgements

-   The project is sponsored by [PSE](https://pse.dev/).
