//! Rust bindings for rapidsnark proving.
//!
//! Prebuilt binaries are provided for the following platforms:
//! - aarch64-apple-ios
//! - aarch64-apple-ios-sim
//! - aarch64-apple-darwin
//! - x86_64-apple-ios
//! - x86_64
//! - aarch64
//!
//! If a specific target is not included the sysytem will fallback to
//! the generic architecture, which may cause problems. e.g. if you compile
//! for aarch64-linux-generic, the system will fallback to aarch64.
//!

use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::os::raw::c_char;
use std::os::raw::c_uint;
use std::str::FromStr;

use anyhow::Context;
use anyhow::Result;
use ark_bn254::Bn254;
use ark_circom::read_proving_key;
use ark_circom::ZkeyHeaderReader;
use num_bigint::BigInt;
use serde::Deserialize;
use serde::Serialize;

/// A function that converts named inputs to a full witness. This should be generated using e.g.
/// [rust-witness](https://crates.io/crates/rust-witness).
pub type WtnsFn = fn(HashMap<String, Vec<BigInt>>) -> Vec<BigInt>;

// match what rapidsnark expects
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct VerificationKey {
    protocol: String,
    curve: String,
    nPublic: u32,
    vk_alpha_1: [String; 3],
    vk_beta_2: [[String; 2]; 3],
    vk_gamma_2: [[String; 2]; 3],
    vk_delta_2: [[String; 2]; 3],
    IC: Vec<[String; 3]>,
}

/// A structure representing a proof and public signals.
#[repr(C)]
pub struct ProofResult {
    proof: *mut c_char,
    public_signals: *mut c_char,
}

extern "C" {
    fn groth16_api_prove(
        zkeyFilename: *const c_char,
        wtnsData: *mut u8,
        wtnsDataLen: c_uint,
    ) -> *mut ProofResult;
    fn groth16_api_verify(proof: *mut ProofResult, key_json: *const c_char) -> bool;
    fn free_proof_result(result: *mut ProofResult);
}

/// Verify a proof using a zkey. The proof is expected to be encoded as json.
pub fn verify_proof(zkey_path: &str, proof: String) -> Result<bool> {
    let mut header_reader = ZkeyHeaderReader::new(zkey_path);
    header_reader.read();
    let file = File::open(zkey_path)?;
    let mut reader = std::io::BufReader::new(file);
    let proving_key = read_proving_key::<_, Bn254>(&mut reader)?;
    // convert out proving key to json so we can
    // use it with rapidsnark
    let vk = proving_key.vk;
    // let v = proving_key.vk.alpha_g1.to_string();
    let vkey = VerificationKey {
        protocol: "groth16".to_string(),
        curve: "bn128".to_string(),
        nPublic: 0, // this is unused in the rapidsnark verifier
        vk_alpha_1: [
            vk.alpha_g1.x.to_string(),
            vk.alpha_g1.y.to_string(),
            "1".to_string(),
        ],
        vk_beta_2: [
            [vk.beta_g2.x.c0.to_string(), vk.beta_g2.x.c1.to_string()],
            [vk.beta_g2.y.c0.to_string(), vk.beta_g2.y.c1.to_string()],
            ["1".to_string(), "0".to_string()],
        ],
        vk_gamma_2: [
            [vk.gamma_g2.x.c0.to_string(), vk.gamma_g2.x.c1.to_string()],
            [vk.gamma_g2.y.c0.to_string(), vk.gamma_g2.y.c1.to_string()],
            ["1".to_string(), "0".to_string()],
        ],
        vk_delta_2: [
            [vk.delta_g2.x.c0.to_string(), vk.delta_g2.x.c1.to_string()],
            [vk.delta_g2.y.c0.to_string(), vk.delta_g2.y.c1.to_string()],
            ["1".to_string(), "0".to_string()],
        ],
        IC: vk
            .gamma_abc_g1
            .iter()
            .map(|p| [p.x.to_string(), p.y.to_string(), "1".to_string()])
            .collect(),
    };
    let vkey_json = serde_json::to_string(&vkey)?;
    let vkey_json_cstr = CString::new(vkey_json)?;
    let v: serde_json::Value = serde_json::from_str(&proof)?;
    let proof = v["proof"].to_string();
    let signals = v["signals"].to_string();
    unsafe {
        let result = groth16_api_verify(
            &mut ProofResult {
                proof: CString::new(proof).unwrap().into_raw(),
                public_signals: CString::new(signals).unwrap().into_raw(),
            },
            vkey_json_cstr.as_ptr(),
        );
        Ok(result)
    }
}

/// Generate a groth16 proof using a specific zkey. Inputs are expected to be base 10 encoded
/// strings. Returns a json encoded proof and public signals.
pub fn generate_proof(
    zkey_path: &str,
    inputs: std::collections::HashMap<String, Vec<String>>,
    witness_fn: WtnsFn,
) -> Result<String> {
    // Form the inputs
    let bigint_inputs = inputs
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                v.into_iter()
                    .map(|i| BigInt::from_str(&i).unwrap())
                    .collect(),
            )
        })
        .collect();

    let mut wtns = witness_fn(bigint_inputs)
        .into_iter()
        .map(|w| w.to_biguint().unwrap())
        .flat_map(|v| {
            let mut bytes = v.to_bytes_le();
            bytes.resize(32, 0);
            bytes
        })
        .collect::<Vec<_>>();

    // Convert Rust strings to C strings
    let zkey_cstr = CString::new(zkey_path).context("Failed to create CString for zkey path")?;

    unsafe {
        let proof_ptr =
            groth16_api_prove(zkey_cstr.as_ptr(), wtns.as_mut_ptr(), wtns.len() as c_uint);

        if proof_ptr.is_null() {
            return Err(anyhow::anyhow!("Proof generation failed"));
        }

        // Convert both strings
        let result = &*proof_ptr;
        let proof = std::ffi::CStr::from_ptr(result.proof)
            .to_string_lossy()
            .into_owned();
        let public_signals = std::ffi::CStr::from_ptr(result.public_signals)
            .to_string_lossy()
            .into_owned();
        free_proof_result(proof_ptr);
        Ok(format!(
            "{{ \"proof\": {proof},\"signals\": {public_signals}}}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::bail;
    use anyhow::Result;
    use num_bigint::BigInt;
    use std::collections::HashMap;
    use std::str::FromStr;

    rust_witness::witness!(multiplier2);
    rust_witness::witness!(keccak256256test);

    fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
        let mut bits = Vec::new();
        for &byte in bytes {
            for j in 0..8 {
                let bit = (byte >> j) & 1;
                bits.push(bit == 1);
            }
        }
        bits
    }

    fn bytes_to_circuit_inputs(input_vec: &[u8]) -> HashMap<String, Vec<String>> {
        let bits = bytes_to_bits(input_vec);
        let converted_vec: Vec<String> = bits
            .into_iter()
            .map(|bit| (bit as i32).to_string())
            .collect();
        let mut inputs = HashMap::new();
        inputs.insert("in".to_string(), converted_vec);
        inputs
    }

    #[test]
    fn test_prove_rapidsnark() -> Result<()> {
        // Create a new MoproCircom instance
        let zkey_path = "./test-vectors/multiplier2_final.zkey".to_string();

        let mut inputs = HashMap::new();
        let a = BigInt::from_str(
            "21888242871839275222246405745257275088548364400416034343698204186575808495616",
        )
        .unwrap();
        let b = BigInt::from(1u8);
        // let c = a.clone() * b.clone();
        inputs.insert("a".to_string(), vec![a.to_string()]);
        inputs.insert("b".to_string(), vec![b.to_string()]);

        let proof_json = super::generate_proof(&zkey_path, inputs, multiplier2_witness)?;
        let valid = super::verify_proof(&zkey_path, proof_json)?;
        if !valid {
            bail!("Proof is invalid");
        }
        Ok(())
    }

    #[test]
    fn test_prove_rapidsnark_keccak() -> Result<()> {
        // Create a new MoproCircom instance
        let zkey_path = "./test-vectors/keccak256_256_test_final.zkey".to_string();
        // Prepare inputs
        let input_vec = vec![
            116, 101, 115, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];

        let inputs = bytes_to_circuit_inputs(&input_vec);

        // Generate Proof
        let proof_json = super::generate_proof(&zkey_path, inputs, keccak256256test_witness)?;
        let valid = super::verify_proof(&zkey_path, proof_json)?;
        if !valid {
            bail!("Proof is invalid");
        }
        Ok(())
    }
}
