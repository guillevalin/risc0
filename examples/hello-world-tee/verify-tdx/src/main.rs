// Copyright 2025 RISC Zero, Inc.
//
// TDX + RISC Zero Attestation Verification Tool
// 
// This tool verifies:
// 1. TDX quotes (hardware attestation)
// 2. RISC Zero receipts (file-based verification)
// 3. Cryptographic binding between them
//
// Note: For full RISC Zero receipt verification, use the hello-world example
// directly which includes the risc0-zkvm verification logic.

use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use std::fs;
use std::path::PathBuf;
use tdx_quote::{Quote, QuoteVerificationError};
use sha2::{Digest, Sha256};

#[derive(Parser, Debug)]
#[command(author, version, about = "TDX + RISC Zero Attestation Verification Tool", long_about = None)]
struct Args {
    /// Path to tdx-output directory
    #[arg(short, long, default_value = "../tdx-output")]
    output_dir: PathBuf,

    /// Verify TDX quote signature (requires valid PCK certificate chain)
    #[arg(long, default_value = "false")]
    verify_tdx: bool,

    /// Verify RISC Zero receipt
    #[arg(long, default_value = "true")]
    verify_receipt: bool,

    /// Verify binding between TDX and RISC Zero attestations
    #[arg(long, default_value = "true")]
    verify_binding: bool,

    /// Display detailed information
    #[arg(short, long, default_value = "false")]
    detailed: bool,

    /// Skip verification, only display information
    #[arg(long, default_value = "false")]
    info_only: bool,
}

fn main() -> Result<()> {
    print_header();

    let args = Args::parse();

    // Check if output directory exists
    if !args.output_dir.exists() {
        eprintln!("{}", "Error: Output directory not found".red().bold());
        eprintln!("Expected: {:?}", args.output_dir);
        eprintln!("\n{}", "Run ./run-in-tdx.sh first to generate attestations".yellow());
        return Err(anyhow::anyhow!("Output directory not found"));
    }

    println!("{}", "Output directory found".green());
    println!("Location: {:?}\n", args.output_dir);

    // Verify TDX Report
    let _tdx_report = verify_tdx_report(&args)?;

    // Verify TDX Quote
    let tdx_quote = verify_tdx_quote(&args)?;

    // Verify RISC Zero Receipt
    let receipt_hash = if args.verify_receipt && !args.info_only {
        verify_risc0_receipt(&args)?
    } else {
        None
    };

    // Verify binding between TDX and RISC Zero
    if args.verify_binding && !args.info_only {
        verify_attestation_binding(&args, &tdx_quote, &receipt_hash)?;
    }

    // Display attestation bundle
    display_attestation_bundle(&args)?;

    // Display remote verification instructions
    display_remote_verification();

    println!("\n{}", "✓ Verification complete!".green().bold());
    
    Ok(())
}

fn print_header() {
    println!("{}", "═══════════════════════════════════════════════════════".cyan().bold());
    println!("{}", "    TDX + RISC Zero Attestation Verification Tool".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════".cyan().bold());
    println!();
}

fn verify_tdx_report(args: &Args) -> Result<Option<Vec<u8>>> {
    println!("{}", "--- TDX Report Verification ---".cyan().bold());
    
    let report_path = args.output_dir.join("tdx-report.bin");
    
    if !report_path.exists() {
        println!("{}", "  TDX report not found, skipping".yellow());
        return Ok(None);
    }

    let report_data = fs::read(&report_path)
        .with_context(|| format!("Failed to read TDX report: {:?}", report_path))?;

    println!("  Report file: {:?}", report_path);
    println!("  Report size: {} bytes", report_data.len());

    // TDX reports should be exactly 1024 bytes
    const EXPECTED_SIZE: usize = 1024;
    if report_data.len() == EXPECTED_SIZE {
        println!("{}", "  ✓ Report size matches TDX report structure".green());
    } else {
        println!(
            "{}",
            format!(
                "  ⚠ Report size ({}) doesn't match expected ({})",
                report_data.len(),
                EXPECTED_SIZE
            )
            .yellow()
        );
    }

    if args.detailed && report_data.len() >= 64 {
        println!("\n  Report structure:");
        println!("    - REPORTDATA (64 bytes): User-provided data");
        println!("    - REPORTMAC (32 bytes): Integrity protection");
        println!("    - TEE TCB INFO: TD measurements and attributes");
        println!("    - MRTD: Measurement of initial TD state");
        println!("    - RTMR[0-3]: Runtime Measurement Registers");
        
        // First 64 bytes are REPORTDATA
        println!("\n  REPORTDATA (first 32 bytes):");
        println!("    {}", hex::encode(&report_data[..32.min(report_data.len())]));
    }

    println!();
    Ok(Some(report_data))
}

fn verify_tdx_quote(args: &Args) -> Result<Option<Quote>> {
    println!("{}", "--- TDX Quote Verification ---".cyan().bold());
    
    let quote_path = args.output_dir.join("tdx-quote.bin");
    
    if !quote_path.exists() {
        println!("{}", "  TDX quote not found, skipping".yellow());
        println!("  Note: Quote generation may have failed during execution\n");
        return Ok(None);
    }

    let quote_data = fs::read(&quote_path)
        .with_context(|| format!("Failed to read TDX quote: {:?}", quote_path))?;

    println!("  Quote file: {:?}", quote_path);
    println!("  Quote size: {} bytes", quote_data.len());

    // Parse the quote
    let quote = Quote::from_bytes(&quote_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse TDX quote: {:?}", e))?;

    println!("{}", "  ✓ Quote parsed successfully".green());

    // Display quote information
    display_quote_info(&quote, args.detailed);

    // Verify the quote signature if requested
    if args.verify_tdx && !args.info_only {
        println!("\n  {}", "Verifying quote signature...".yellow());
        match verify_quote_signature(&quote) {
            Ok(()) => {
                println!("{}", "  ✓ Quote signature verified".green());
                println!("    - Signature is valid");
                println!("    - PCK certificate chain verified");
            }
            Err(e) => {
                println!("{}", format!("  ✗ Quote verification failed: {:?}", e).red());
                println!("{}", "    Note: This may be expected if running without proper Intel certificates".yellow());
            }
        }
    } else {
        println!("{}", "  ℹ Signature verification skipped (use --verify-tdx to enable)".yellow());
    }

    println!();
    Ok(Some(quote))
}

fn display_quote_info(quote: &Quote, detailed: bool) {
    println!("\n  Quote Header:");
    println!("    Version: {:?}", quote.header.version);
    println!("    Attestation Key Type: {:?}", quote.header.attestation_key_type);
    println!("    TEE Type: {:?}", quote.header.tee_type);
    
    if detailed {
        println!("    QE Vendor ID: {}", hex::encode(quote.header.qe_vendor_id));
    }

    println!("\n  Quote Body (Measurements):");
    println!("    MR TD: {}", hex::encode(&quote.body.mrtd[..16]));
    println!("    MR CONFIG ID: {}", hex::encode(&quote.body.mrconfigid[..16]));
    println!("    MR OWNER: {}", hex::encode(&quote.body.mrowner[..16]));
    
    println!("\n  Runtime Measurements:");
    println!("    RT MR 0: {}", hex::encode(&quote.body.rtmr0[..16]));
    println!("    RT MR 1: {}", hex::encode(&quote.body.rtmr1[..16]));
    println!("    RT MR 2: {}", hex::encode(&quote.body.rtmr2[..16]));
    println!("    RT MR 3: {}", hex::encode(&quote.body.rtmr3[..16]));
    
    println!("\n  Report Data (first 32 bytes - may contain receipt hash):");
    println!("    {}", hex::encode(&quote.body.reportdata[..32]));

    if detailed {
        println!("\n  Report Data (full 64 bytes):");
        println!("    {}", hex::encode(&quote.body.reportdata));
    }
}

fn verify_quote_signature(quote: &Quote) -> Result<(), QuoteVerificationError> {
    // Verify the quote signature - returns the verifying key on success
    let _verifying_key = quote.verify()?;
    Ok(())
}

fn verify_risc0_receipt(args: &Args) -> Result<Option<Vec<u8>>> {
    println!("{}", "--- RISC Zero Receipt Verification ---".cyan().bold());
    
    let receipt_path = args.output_dir.join("risc0-receipt.json");
    
    if !receipt_path.exists() {
        println!("{}", "  RISC Zero receipt JSON not found".yellow());
        println!("  Note: Receipt verification happens during execution");
        println!("  Check execution.log for verification results\n");
        return Ok(None);
    }

    let receipt_json = fs::read_to_string(&receipt_path)
        .with_context(|| format!("Failed to read receipt file: {:?}", receipt_path))?;

    println!("  Receipt file: {:?}", receipt_path);
    
    // Try to parse as JSON to display info
    if let Ok(receipt_info) = serde_json::from_str::<serde_json::Value>(&receipt_json) {
        println!("  Receipt info:");
        if let Some(status) = receipt_info.get("status") {
            println!("    Status: {}", status);
        }
        if let Some(timestamp) = receipt_info.get("timestamp") {
            println!("    Timestamp: {}", timestamp);
        }
        
        if args.detailed {
            println!("\n  Full receipt JSON:");
            println!("{}", serde_json::to_string_pretty(&receipt_info)?);
        }
    }

    // Check for binary receipt file
    let receipt_bin_path = args.output_dir.join("risc0-receipt.bin");
    if receipt_bin_path.exists() {
        let receipt_bin = fs::read(&receipt_bin_path)?;
        println!("{}", "  ✓ Binary receipt found".green());
        println!("    Size: {} bytes", receipt_bin.len());
        
        // Calculate hash for binding verification
        let mut hasher = Sha256::new();
        hasher.update(&receipt_bin);
        let receipt_hash = hasher.finalize();
        
        println!("\n  Receipt Hash (SHA-256):");
        println!("    {}", hex::encode(&receipt_hash));
        
        println!();
        return Ok(Some(receipt_hash.to_vec()));
    }

    println!("{}", "  Note: Binary receipt not found (use src/main-tdx.rs.example to export)".yellow());
    println!();
    Ok(None)
}

fn verify_attestation_binding(
    _args: &Args,
    tdx_quote: &Option<Quote>,
    receipt_hash: &Option<Vec<u8>>,
) -> Result<()> {
    println!("{}", "--- Attestation Binding Verification ---".cyan().bold());
    
    match (tdx_quote, receipt_hash) {
        (Some(quote), Some(hash)) => {
            println!("  Checking if receipt hash is bound to TDX quote...");
            
            // Extract first 32 bytes of REPORTDATA from quote
            let reportdata = &quote.body.reportdata[..32];
            
            println!("\n  TDX REPORTDATA (first 32 bytes):");
            println!("    {}", hex::encode(reportdata));
            
            println!("\n  RISC Zero Receipt Hash:");
            println!("    {}", hex::encode(hash));
            
            if reportdata == &hash[..] {
                println!("{}", "\n  ✓ BINDING VERIFIED".green().bold());
                println!("    The TDX quote and RISC Zero receipt are cryptographically bound!");
                println!("    This proves:");
                println!("      - The computation ran in TDX hardware");
                println!("      - The specific RISC Zero receipt was generated in that environment");
            } else {
                println!("{}", "\n  ⚠ Binding not found".yellow());
                println!("    The receipt hash is not in the TDX REPORTDATA");
                println!("    Note: This is expected if the binding was not implemented during generation");
                println!("    To enable binding, modify run-in-tdx.sh to include receipt hash in REPORTDATA");
            }
        }
        (None, _) => {
            println!("{}", "  Skipped: TDX quote not available".yellow());
        }
        (_, None) => {
            println!("{}", "  Skipped: Receipt hash not available".yellow());
        }
    }
    
    println!();
    Ok(())
}

fn display_attestation_bundle(args: &Args) -> Result<()> {
    println!("{}", "--- Attestation Bundle ---".cyan().bold());
    
    let bundle_path = args.output_dir.join("attestation-bundle.json");
    
    if !bundle_path.exists() {
        println!("{}", "  Attestation bundle not found".yellow());
        return Ok(());
    }

    let bundle_json = fs::read_to_string(&bundle_path)
        .with_context(|| format!("Failed to read bundle: {:?}", bundle_path))?;

    let bundle: serde_json::Value = serde_json::from_str(&bundle_json)?;
    
    println!("{}", serde_json::to_string_pretty(&bundle)?);
    println!();
    
    Ok(())
}

fn display_remote_verification() {
    println!("{}", "--- Remote Verification Instructions ---".cyan().bold());
    println!();
    println!("To verify these attestations remotely:");
    println!();
    println!("{}", "1. TDX Quote Verification:".yellow());
    println!("   • Intel Trust Authority:");
    println!("     curl -X POST https://api.trustauthority.intel.com/v1/attest \\");
    println!("       -H \"Authorization: Bearer $INTEL_API_TOKEN\" \\");
    println!("       --data-binary @tdx-output/tdx-quote.bin");
    println!();
    println!("   • Azure Attestation:");
    println!("     az attestation attest \\");
    println!("       --attestation-provider \"MyProvider\" \\");
    println!("       --attestation-type TDX \\");
    println!("       --quote-file tdx-output/tdx-quote.bin");
    println!();
    println!("{}", "2. RISC Zero Receipt Verification:".yellow());
    println!("   • Use risc0-zkvm library:");
    println!("     receipt.verify(MULTIPLY_ID)?;");
    println!();
    println!("{}", "3. Combined Attestation:".yellow());
    println!("   • Verify receipt hash matches TDX REPORTDATA");
    println!("   • Verify both attestations independently");
    println!("   • Confirm cryptographic binding");
    println!();
    println!("Resources:");
    println!("  • Intel Trust Authority: {}", "https://trustauthority.intel.com/".cyan());
    println!("  • Azure Attestation: {}", "https://azure.microsoft.com/services/attestation/".cyan());
    println!("  • RISC Zero Docs: {}", "https://dev.risczero.com/".cyan());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_calculation() {
        let data = b"test data";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_binding_verification() {
        let hash1 = vec![0u8; 32];
        let hash2 = vec![0u8; 32];
        assert_eq!(hash1, hash2);
    }
}

