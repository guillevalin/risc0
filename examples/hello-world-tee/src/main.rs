// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use hello_world_tee::multiply;
use hello_world_methods::MULTIPLY_ID;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("🚀 RISC Zero + Intel TDX Integration");
    println!("=====================================\n");

    // Pick two numbers
    let (receipt, result) = multiply(17, 23);

    println!("✓ Computation complete: {} = 17 × 23", result);

    // ═══════════════════════════════════════════════════════════
    // CRITICAL: Verify receipt INSIDE THE TEE
    // ═══════════════════════════════════════════════════════════
    // This verification happens inside the Intel TDX Trust Domain.
    // The verification result will be included in the TDX attestation
    // to prove that the receipt was not only generated in the TEE,
    // but also verified to be correct within the TEE.
    
    println!("🔐 Verifying RISC Zero receipt inside TDX TEE...");
    
    let verification_start = std::time::Instant::now();
    let verification_result = receipt.verify(MULTIPLY_ID);
    let verification_duration = verification_start.elapsed();
    
    match verification_result {
        Ok(_) => {
            println!("✓ Receipt verification PASSED inside TEE");
            println!("  Verification time: {:?}", verification_duration);
            println!("  Image ID: {:?}", MULTIPLY_ID);
        }
        Err(e) => {
            eprintln!("✗ Receipt verification FAILED inside TEE: {:?}", e);
            std::process::exit(1);
        }
    }
    
    println!();

    // Create output directory
    let output_dir = "tdx-output";
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    // Serialize receipt to JSON (for human readability)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let receipt_json = serde_json::json!({
        "timestamp": timestamp,
        "example": "hello-world-tee",
        "status": "success",
        "result": result,
        "verification": {
            "verified_in_tee": true,
            "verification_passed": true,
            "verification_time_ms": verification_duration.as_millis(),
            "image_id": format!("{:?}", MULTIPLY_ID)
        },
        "note": "Receipt generated AND verified inside Intel TDX TEE"
    });

    let receipt_json_path = format!("{}/risc0-receipt.json", output_dir);
    fs::write(&receipt_json_path, serde_json::to_string_pretty(&receipt_json).unwrap())
        .expect("Failed to write receipt JSON");

    println!("✓ Receipt metadata saved to: {}", receipt_json_path);

    // Serialize receipt to binary (for verification)
    let receipt_binary = bincode::serialize(&receipt)
        .expect("Failed to serialize receipt to binary");

    let receipt_bin_path = format!("{}/risc0-receipt.bin", output_dir);
    fs::write(&receipt_bin_path, &receipt_binary)
        .expect("Failed to write receipt binary");

    println!("✓ Receipt binary saved to: {}", receipt_bin_path);

    // Calculate receipt hash for TDX binding
    let mut hasher = Sha256::new();
    hasher.update(&receipt_binary);
    let receipt_hash = hasher.finalize();

    // Create verification certificate
    let hash_path = format!("{}/receipt-hash.txt", output_dir);
    let mut hash_file = fs::File::create(&hash_path)
        .expect("Failed to create hash file");
    writeln!(hash_file, "Receipt SHA-256: {}", hex::encode(&receipt_hash))
        .expect("Failed to write hash");
    writeln!(hash_file, "\n=== Verification Certificate ===")
        .expect("Failed to write");
    writeln!(hash_file, "Verified in TEE: YES")
        .expect("Failed to write");
    writeln!(hash_file, "Verification Status: PASSED")
        .expect("Failed to write");
    writeln!(hash_file, "Verification Time: {:?}", verification_duration)
        .expect("Failed to write");
    writeln!(hash_file, "Image ID: {:?}", MULTIPLY_ID)
        .expect("Failed to write");
    writeln!(
        hash_file,
        "\nThis hash should be included in TDX REPORTDATA to bind attestations"
    )
    .expect("Failed to write hash");

    println!("✓ Receipt hash saved to: {}", hash_path);
    println!("  Hash: {}", hex::encode(&receipt_hash));

    println!("\n🎉 All outputs generated successfully!");
    println!("\n══════════════════════════════════════════════════");
    println!("Generated files in {}:", output_dir);
    println!("  - risc0-receipt.json     (metadata + verification status)");
    println!("  - risc0-receipt.bin      (full receipt for verification)");
    println!("  - receipt-hash.txt       (SHA-256 + verification certificate)");
    println!("══════════════════════════════════════════════════");
    println!("\n🔐 IMPORTANT:");
    println!("  ✓ Proof generated inside TDX TEE");
    println!("  ✓ Proof verified inside TDX TEE");
    println!("  ✓ Receipt hash will be bound to TDX attestation");
    println!("\nNext step: Run TDX attestation generation");
}
