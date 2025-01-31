use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use num_bigint::BigInt;
pub type WtnsFn = fn(HashMap<String, Vec<BigInt>>) -> Vec<BigInt>;

pub struct ProofResult {
    #[allow(unused)] // TODO: Remove this once we have a proper way to handle this
    proof: String,
    #[allow(unused)] // TODO: Remove this once we have a proper way to handle this
    public_signals: String,
}

extern "C" {
    fn groth16_prover_zkey_file(
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
            zkey_path.as_ptr() as *const std::ffi::c_char,
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use num_bigint::BigInt;
    use std::{collections::HashMap, str::FromStr};

    use crate::{parse_bigints_to_witness, WtnsFn};

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

    fn compute_witness(
        inputs: HashMap<String, Vec<String>>,
        witness_fn: WtnsFn,
    ) -> Result<Vec<u8>> {
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

        let wtns: Vec<BigInt> = witness_fn(bigint_inputs);
        let witnesscalc_wtns = parse_bigints_to_witness(wtns)?;
        Ok(witnesscalc_wtns)
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

        let wtns_buffer = compute_witness(inputs, multiplier2_witness)?;
        let proof_result = super::groth16_prover_zkey_file_wrapper(&zkey_path, wtns_buffer)?;
        println!("{}", proof_result.proof);
        println!("{}", proof_result.public_signals);
        // let valid = super::verify_proof(&zkey_path, proof_json)?;
        // if !valid {
        //     bail!("Proof is invalid");
        // }
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

        // Generate Witness Buffer
        let wtns_buffer = compute_witness(inputs, keccak256256test_witness)?;
        let wtns_data = std::fs::read("./test-vectors/keccak256_256_test.wtns")?;
        assert_eq!(wtns_buffer, wtns_data);

        // Generate Proof
        let proof_result = super::groth16_prover_zkey_file_wrapper(&zkey_path, wtns_buffer)?;
        println!("{}", proof_result.proof);
        println!("{}", proof_result.public_signals);
        // let valid = super::verify_proof(&zkey_path, proof_json)?;
        // if !valid {
        //     bail!("Proof is invalid");
        // }
        Ok(())
    }
}
