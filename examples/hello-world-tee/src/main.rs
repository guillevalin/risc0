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

    // Verify receipt, panic if it's wrong
    receipt.verify(MULTIPLY_ID).expect(
        "Code you have proven should successfully verify; did you specify the correct image ID?",
    );

    println!("✓ Receipt verified locally\n");

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
        "note": "Full receipt saved in binary format"
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

    let hash_path = format!("{}/receipt-hash.txt", output_dir);
    let mut hash_file = fs::File::create(&hash_path)
        .expect("Failed to create hash file");
    writeln!(hash_file, "Receipt SHA-256: {}", hex::encode(&receipt_hash))
        .expect("Failed to write hash");
    writeln!(
        hash_file,
        "\nThis hash should be included in TDX REPORTDATA to bind attestations"
    )
    .expect("Failed to write hash");

    println!("✓ Receipt hash saved to: {}", hash_path);
    println!("  Hash: {}", hex::encode(&receipt_hash));

    println!("\n🎉 All outputs generated successfully!");
    println!("\nGenerated files in {}:", output_dir);
    println!("  - risc0-receipt.json  (metadata)");
    println!("  - risc0-receipt.bin   (full receipt for verification)");
    println!("  - receipt-hash.txt    (SHA-256 for TDX binding)");
}
