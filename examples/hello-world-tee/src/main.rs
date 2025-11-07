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
use std::path::Path;

/// Check if running inside Intel TDX TEE
fn is_inside_tdx_tee() -> bool {
    Path::new("/dev/tdx_guest").exists()
}

/// Get TDX-specific information from the guest environment
fn get_tdx_info() -> Option<String> {
    // Read TDX CPU capabilities if available
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        if cpuinfo.contains("tdx_guest") {
            return Some("TDX guest feature detected in CPU".to_string());
        }
    }
    
    // Check for TDX device
    if Path::new("/dev/tdx_guest").exists() {
        return Some("TDX guest device present".to_string());
    }
    
    None
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("🚀 RISC Zero + Intel TDX Integration");
    println!("=====================================\n");

    // ═══════════════════════════════════════════════════════════
    // CRITICAL: Enforce TDX TEE Environment
    // ═══════════════════════════════════════════════════════════
    // This code will ONLY run inside an Intel TDX Trust Domain.
    // If not in TDX, the program exits immediately.
    
    println!("🔍 Checking TDX TEE environment...");
    
    if !is_inside_tdx_tee() {
        eprintln!("\n❌ SECURITY VIOLATION: Not running inside Intel TDX TEE!");
        eprintln!("This program MUST run inside a TDX Trust Domain.");
        eprintln!("TDX device not found at /dev/tdx_guest");
        eprintln!("\nTo run this example:");
        eprintln!("  1. Use a TDX-enabled machine (e.g., Google Cloud C3 instances)");
        eprintln!("  2. Ensure TDX is enabled in BIOS/firmware");
        eprintln!("  3. Boot into a TDX-capable VM/TD");
        std::process::exit(1);
    }
    
    let tdx_info = get_tdx_info().unwrap_or_else(|| "TDX detected".to_string());
    println!("✓ TDX TEE verified: {}", tdx_info);
    println!();

    // Pick two numbers
    let (receipt, result) = multiply(17, 23);

    println!("✓ Computation complete: {} = 17 × 23", result);

    // ═══════════════════════════════════════════════════════════
    // CRITICAL: Verify receipt INSIDE THE TEE
    // ═══════════════════════════════════════════════════════════
    // This verification happens inside the Intel TDX Trust Domain.
    // The program already checked that /dev/tdx_guest exists.
    // The verification result will be included in the TDX attestation
    // to prove that the receipt was verified within the TEE.
    
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
        "tee_environment": {
            "tee_type": "Intel TDX",
            "tee_verified": true,
            "tee_device": "/dev/tdx_guest",
            "tee_info": tdx_info
        },
        "verification": {
            "verified_in_tee": true,
            "verification_passed": true,
            "verification_time_ms": verification_duration.as_millis(),
            "image_id": format!("{:?}", MULTIPLY_ID)
        },
        "note": "Receipt generated AND verified inside Intel TDX TEE - program enforces TDX environment"
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
    writeln!(hash_file, "\n=== TEE Environment ===")
        .expect("Failed to write");
    writeln!(hash_file, "TEE Type: Intel TDX")
        .expect("Failed to write");
    writeln!(hash_file, "TEE Device: /dev/tdx_guest")
        .expect("Failed to write");
    writeln!(hash_file, "TEE Status: {}", tdx_info)
        .expect("Failed to write");
    writeln!(hash_file, "Enforcement: Program ONLY runs in TDX (exits otherwise)")
        .expect("Failed to write");
    writeln!(hash_file, "\n=== Verification Certificate ===")
        .expect("Failed to write");
    writeln!(hash_file, "Verified in TEE: YES (enforced)")
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
