#[cfg(test)]
mod tests {
    use anyhow::Result;
    use num_bigint::BigInt;
    use std::{collections::HashMap, str::FromStr};

    use rust_rapidsnark::{parse_bigints_to_witness, WtnsFn};

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
        inputs.insert("a".to_string(), vec![a.to_string()]);
        inputs.insert("b".to_string(), vec![b.to_string()]);

        // Generate Witness Buffer
        let wtns_buffer = compute_witness(inputs, multiplier2_witness)?;

        // Generate Proof
        let proof_result =
            rust_rapidsnark::groth16_prover_zkey_file_wrapper(&zkey_path, wtns_buffer)?;

        let vkey = std::fs::read_to_string("./test-vectors/multiplier2.vkey.json")?;
        let valid = rust_rapidsnark::groth16_verify_wrapper(
            &proof_result.proof,
            &proof_result.public_signals,
            &vkey,
        )?;
        assert!(valid);
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
        let proof_result =
            rust_rapidsnark::groth16_prover_zkey_file_wrapper(&zkey_path, wtns_buffer)?;

        let vkey = std::fs::read_to_string("./test-vectors/keccak256_256_test.vkey.json")?;
        let valid = rust_rapidsnark::groth16_verify_wrapper(
            &proof_result.proof,
            &proof_result.public_signals,
            &vkey,
        )?;
        assert!(valid);
        Ok(())
    }
}
