//! Rust bindings for rapidsnark proving.
//!
//! Prebuilt binaries are provided for the following platforms:
//! - aarch64-apple-ios
//! - aarch64-apple-ios-sim
//! - x86_64-apple-ios
//! - aarch64-apple-darwin
//! - x86_64-apple-darwin
//! - aarch64-linux-android
//! - x86_64-linux-android
//! - x86_64 linux
//! - arm64 linux
//!
//! If a specific target is not included the sysytem will fallback to
//! the generic architecture, which may cause problems. e.g. if you compile
//! for aarch64-linux-generic, the system will fallback to aarch64.
//!

use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use num_bigint::BigInt;

/// A function that converts named inputs to a full witness. This should be generated using e.g.
/// [rust-witness](https://crates.io/crates/rust-witness).
pub type WtnsFn = fn(HashMap<String, Vec<BigInt>>) -> Vec<BigInt>;

/// A structure representing a proof and public signals.
#[derive(Debug)]
pub struct ProofResult {
    pub proof: String,
    pub public_signals: String,
}

#[link(name = "rapidsnark", kind = "static")]
extern "C" {
    pub fn groth16_prover_zkey_file(
        zkey_file_path: *const std::os::raw::c_char,
        wtns_buffer: *const std::os::raw::c_void,
        wtns_size: std::ffi::c_ulong,
        proof_buffer: *mut std::os::raw::c_char,
        proof_size: *mut std::ffi::c_ulong,
        public_buffer: *mut std::os::raw::c_char,
        public_size: *mut std::ffi::c_ulong,
        error_msg: *mut std::os::raw::c_char,
        error_msg_maxsize: std::ffi::c_ulong,
    ) -> i32;

    pub fn groth16_verify(
        proof: *const std::os::raw::c_char,
        inputs: *const std::os::raw::c_char,
        verification_key: *const std::os::raw::c_char,
        error_msg: *mut std::os::raw::c_char,
        error_msg_maxsize: std::ffi::c_ulong,
    ) -> i32;
}

use num_traits::ops::bytes::ToBytes;
use std::io::{self};

/// Parse bigints to `wtns` format.<br/>
/// Reference: [witnesscalc/src/witnesscalc.cpp](https://github.com/0xPolygonID/witnesscalc/blob/4a789880727aa0df50f1c4ef78ec295f5a30a15e/src/witnesscalc.cpp)
pub fn parse_bigints_to_witness(bigints: Vec<BigInt>) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let version: u32 = 2;
    let n_sections: u32 = 2;
    let n8: u32 = 32;
    let q = BigInt::from_str(
        "21888242871839275222246405745257275088548364400416034343698204186575808495617",
    )
    .unwrap();
    let n_witness_values: u32 = bigints.len() as u32;

    // Write the format bytes (4 bytes)
    let wtns_format = "wtns".as_bytes();
    buffer.extend_from_slice(wtns_format);

    // Write version (4 bytes)
    buffer.extend_from_slice(&version.to_le_bytes());

    // Write number of sections (4 bytes)
    buffer.extend_from_slice(&n_sections.to_le_bytes());

    // Iterate through sections to write the data
    // Section 1 (Field parameters)
    let section_id_1: u32 = 1;
    let section_length_1: u64 = 8 + n8 as u64;
    buffer.extend_from_slice(&section_id_1.to_le_bytes());
    buffer.extend_from_slice(&section_length_1.to_le_bytes());

    // Write n8 (4 bytes), q (32 bytes), and n_witness_values (4 bytes)
    buffer.extend_from_slice(&n8.to_le_bytes());
    buffer.extend_from_slice(&q.to_signed_bytes_le());
    buffer.extend_from_slice(&n_witness_values.to_le_bytes());

    // Section 2 (Witness data)
    let section_id_2: u32 = 2;
    let section_length_2: u64 = bigints.len() as u64 * n8 as u64; // Witness data size
    buffer.extend_from_slice(&section_id_2.to_le_bytes());
    buffer.extend_from_slice(&section_length_2.to_le_bytes());

    // Write the witness data (each BigInt to n8 bytes)
    for bigint in bigints {
        let mut bytes = bigint.to_le_bytes();
        bytes.resize(n8 as usize, 0); // Ensure each BigInt is padded to n8 bytes
        buffer.extend_from_slice(&bytes);
    }

    // Return the buffer containing the complete witness data
    Ok(buffer)
}

/// Wrapper for `groth16_prover_zkey_file`
pub fn groth16_prover_zkey_file_wrapper(
    zkey_path: &str,
    wtns_buffer: Vec<u8>,
) -> Result<ProofResult> {
    let formatted_zkey_path = zkey_path.to_string();
    let wtns_size = wtns_buffer.len() as u64;

    let mut proof_buffer = vec![0u8; 4 * 1024 * 1024]; // Adjust size as needed
    let mut proof_size: u64 = 4 * 1024 * 1024;
    let proof_ptr = proof_buffer.as_mut_ptr() as *mut std::ffi::c_char;

    let mut public_buffer = vec![0u8; 4 * 1024 * 1024]; // Adjust size as needed
    let mut public_size: u64 = 4 * 1024 * 1024;
    let public_ptr = public_buffer.as_mut_ptr() as *mut std::ffi::c_char;

    let mut error_msg = vec![0u8; 256]; // Error message buffer
    let error_msg_ptr = error_msg.as_mut_ptr() as *mut std::ffi::c_char;

    unsafe {
        let result = groth16_prover_zkey_file(
            formatted_zkey_path.as_ptr() as *const std::ffi::c_char,
            wtns_buffer.as_ptr() as *const std::os::raw::c_void, // Witness buffer
            wtns_size,
            proof_ptr,
            &mut proof_size,
            public_ptr,
            &mut public_size,
            error_msg_ptr,
            error_msg.len() as u64,
        );
        if result != 0 {
            let error_string = std::ffi::CStr::from_ptr(error_msg_ptr)
                .to_string_lossy()
                .into_owned();
            return Err(anyhow::anyhow!("Proof generation failed: {}", error_string));
        }
        // Convert both strings
        let proof = std::ffi::CStr::from_ptr(proof_ptr)
            .to_string_lossy()
            .into_owned();
        let public_signals = std::ffi::CStr::from_ptr(public_ptr)
            .to_string_lossy()
            .into_owned();
        Ok(ProofResult {
            proof,
            public_signals,
        })
    }
}

/// Wrapper for `groth16_verify`
pub fn groth16_verify_wrapper(proof: &str, inputs: &str, verification_key: &str) -> Result<bool> {
    let proof_cstr = std::ffi::CString::new(proof).unwrap();
    let inputs_cstr = std::ffi::CString::new(inputs).unwrap();
    let verification_key_cstr = std::ffi::CString::new(verification_key).unwrap();

    let mut error_msg = vec![0u8; 256]; // Error message buffer
    let error_msg_ptr = error_msg.as_mut_ptr() as *mut std::ffi::c_char;
    unsafe {
        let result = groth16_verify(
            proof_cstr.as_ptr() as *const std::ffi::c_char,
            inputs_cstr.as_ptr() as *const std::ffi::c_char,
            verification_key_cstr.as_ptr() as *const std::ffi::c_char,
            error_msg_ptr,
            error_msg.len() as u64,
        );
        if result == 2 {
            let error_string = std::ffi::CStr::from_ptr(error_msg_ptr)
                .to_string_lossy()
                .into_owned();
            return Err(anyhow::anyhow!(
                "Proof verification failed: {}",
                error_string
            ));
        }
        Ok(result == 0)
    }
}
