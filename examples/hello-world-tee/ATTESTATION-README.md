# Intel TDX + RISC Zero Attestation Guide

This guide explains how to run the RISC Zero hello-world example inside an Intel TDX (Trust Domain Extensions) TEE (Trusted Execution Environment) and generate attestation quotes for remote verification.

## Overview

This integration combines two powerful technologies:

1. **Intel TDX**: Hardware-based confidential computing that provides isolated execution environments
2. **RISC Zero zkVM**: Zero-knowledge virtual machine for verifiable computation

Together, they provide:
- **Confidential Execution**: Code runs in isolated TDX Trust Domain
- **Hardware Attestation**: TDX quotes prove execution environment integrity
- **Computational Proof**: RISC Zero receipts prove correct execution
- **Remote Verification**: Both attestations can be verified independently

## Prerequisites

### Hardware Requirements

- Intel CPU with TDX support (4th Gen Xeon Scalable or newer)
- TDX enabled in BIOS/UEFI
- Minimum 4GB RAM
- 20GB disk space

### Software Requirements

- Linux kernel 5.19+ with TDX support
- Intel TDX guest kernel module
- Rust toolchain (1.70+)
- Build essentials
- Optional: Intel SGX DCAP libraries for quote verification

### Google Cloud Setup

If using Google Cloud's TDX-enabled instances:

```bash
# Create a TDX-enabled VM
gcloud compute instances create tdx-risc0-test \
  --zone=us-central1-a \
  --machine-type=n2d-standard-4 \
  --confidential-compute \
  --confidential-compute-type=TDX \
  --image-family=ubuntu-2204-lts \
  --image-project=ubuntu-os-cloud \
  --boot-disk-size=50GB
```

## Quick Start

### 1. Clone and Navigate to Example

```bash
cd /path/to/risc0/examples/hello-world
```

### 2. Make Scripts Executable

```bash
chmod +x run-in-tdx.sh verify-attestation.sh
```

### 3. Run Inside TDX

```bash
./run-in-tdx.sh
```

This script will:
- ✓ Verify TDX environment
- ✓ Install dependencies
- ✓ Build RISC Zero example
- ✓ Generate TDX report
- ✓ Generate TDX attestation quote
- ✓ Execute RISC Zero proof
- ✓ Create attestation bundle

### 4. Verify Attestations

```bash
./verify-attestation.sh
```

## Output Files

All generated files are in the `tdx-output/` directory:

```
tdx-output/
├── tdx-report.bin              # TDX report (1024 bytes)
├── tdx-quote.bin               # TDX attestation quote
├── risc0-receipt.json          # RISC Zero receipt summary
├── execution.log               # Detailed execution logs
└── attestation-bundle.json     # Combined attestation metadata
```

## Understanding TDX Attestation

### TDX Report Structure

The TDX report (`tdx-report.bin`) contains:

- **REPORTDATA** (64 bytes): User-provided data, can include RISC Zero receipt hash
- **MRTD**: Measurement of the initial TD (Trust Domain) state
- **RTMR[0-3]**: Runtime Measurement Registers
- **MRCONFIGID**: Software-defined ID for additional configuration
- **MROWNER**: Software-defined ID for TD owner
- **MROWNERCONFIG**: Software-defined ID for owner-defined configuration
- **TCB Info**: Trusted Computing Base version information

### TDX Quote Structure

The TDX quote (`tdx-quote.bin`) contains:

- **Quote Header**: Version, attestation key type, TEE type (TDX)
- **TD Report**: The TDX report data
- **Quote Signature**: ECDSA signature over header + report
- **Certificate Chain**: QE (Quoting Enclave) certificates
- **QE Report**: Quote generation enclave attestation

### Verification Flow

```
┌─────────────────┐
│  TDX Quote      │
│  (Generated)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Verify Signature│
│ with Intel PKI  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Check TCB Level │
│ & Security      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Validate MRTD   │
│ & RTMRs         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Extract         │
│ REPORTDATA      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Verify RISC Zero│
│ Receipt Hash    │
└─────────────────┘
```

## Remote Verification

### Option 1: Intel Trust Authority

```bash
# Upload quote for verification
curl -X POST https://api.trustauthority.intel.com/v1/attest \
  -H "Authorization: Bearer $API_TOKEN" \
  -H "Content-Type: application/json" \
  -d @tdx-output/tdx-quote.bin
```

### Option 2: Azure Attestation

```bash
# Using Azure Attestation Service
az attestation attest \
  --attestation-provider "MyProvider" \
  --attestation-type TDX \
  --quote-file tdx-output/tdx-quote.bin
```

### Option 3: Intel DCAP Libraries

```c
#include <sgx_dcap_quoteverify.h>

quote3_error_t verify_quote(
    const uint8_t *quote,
    uint32_t quote_size
) {
    sgx_ql_qv_result_t quote_verification_result;
    uint32_t collateral_expiration_status;
    
    quote3_error_t ret = sgx_qv_verify_quote(
        quote, quote_size,
        NULL,  // PCK certificate chain
        NULL,  // TCB info
        NULL,  // QE identity
        time(NULL),
        &collateral_expiration_status,
        &quote_verification_result,
        NULL,  // QVE report info
        0      // Supplemental data size
    );
    
    return ret;
}
```

### Option 4: RISC Zero Receipt Verification

```rust
use risc0_zkvm::Receipt;
use hello_world_methods::MULTIPLY_ID;

fn verify_receipt(receipt: &Receipt) -> Result<(), String> {
    // Verify the receipt
    receipt.verify(MULTIPLY_ID)
        .map_err(|e| format!("Receipt verification failed: {}", e))?;
    
    // Extract and verify journal
    let result: u64 = receipt.journal.decode()
        .map_err(|e| format!("Failed to decode journal: {}", e))?;
    
    println!("Verified computation result: {}", result);
    
    Ok(())
}
```

## Binding TDX and RISC Zero Attestations

To create a cryptographic link between TDX attestation and RISC Zero proof:

```rust
use sha2::{Sha256, Digest};

fn bind_attestations(receipt: &Receipt) -> [u8; 64] {
    // Serialize receipt
    let receipt_bytes = bincode::serialize(receipt).unwrap();
    
    // Hash the receipt
    let mut hasher = Sha256::new();
    hasher.update(&receipt_bytes);
    let receipt_hash = hasher.finalize();
    
    // Create REPORTDATA for TDX
    let mut report_data = [0u8; 64];
    report_data[..32].copy_from_slice(&receipt_hash);
    // Remaining 32 bytes can include additional metadata
    
    report_data
}
```

Then use this `report_data` when generating the TDX report, ensuring that the TDX quote is bound to the specific RISC Zero receipt.

## Advanced Configuration

### Custom Report Data

Modify `run-in-tdx.sh` to include custom report data:

```bash
# In generate_tdx_report() function
local receipt_hash=$(sha256sum "${RECEIPT_FILE}" | cut -d' ' -f1)
local report_data="${receipt_hash}$(printf '%.32s' '')"
```

### Continuous Attestation

For long-running services, implement periodic re-attestation:

```bash
while true; do
    ./run-in-tdx.sh
    sleep 3600  # Re-attest every hour
done
```

### Integration with RISC Zero Host

Modify `src/main.rs` to export the receipt:

```rust
use std::fs;
use hello_world_methods::MULTIPLY_ID;

fn main() {
    // ... existing code ...
    
    let (receipt, _) = multiply(17, 23);
    
    // Serialize and save receipt
    let receipt_json = serde_json::to_string_pretty(&receipt).unwrap();
    fs::write("tdx-output/risc0-receipt.json", receipt_json)
        .expect("Failed to write receipt");
    
    // Verify receipt
    receipt.verify(MULTIPLY_ID).expect("Verification failed");
}
```

## Troubleshooting

### TDX Device Not Found

```bash
# Check if TDX is enabled
dmesg | grep -i tdx

# Check kernel module
lsmod | grep tdx

# Load module manually
sudo modprobe tdx_guest
```

### Quote Generation Fails

```bash
# Check for attestation service
ls -la /var/run/tdx-attest.sock

# Check configfs-tsm
ls -la /sys/kernel/config/tsm/report/

# Install Intel DCAP libraries
wget https://download.01.org/intel-sgx/sgx-dcap/1.18/linux/distro/ubuntu22.04-server/sgx_linux_x64_sdk_2.21.100.1.bin
chmod +x sgx_linux_x64_sdk_2.21.100.1.bin
./sgx_linux_x64_sdk_2.21.100.1.bin
```

### Build Failures

```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build --release --features prove
```

## Security Considerations

1. **Measurement Validation**: Always verify MRTD matches expected value
2. **TCB Level**: Check that TCB (Trusted Computing Base) is up to date
3. **Certificate Chain**: Verify quote signature chain to Intel root CA
4. **Freshness**: Include timestamp in REPORTDATA to prevent replay attacks
5. **Side Channels**: Consider additional protections for sensitive data

## References

- [Intel TDX Documentation](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-trust-domain-extensions.html)
- [RISC Zero Documentation](https://dev.risczero.com/)
- [Intel SGX DCAP](https://github.com/intel/SGXDataCenterAttestationPrimitives)
- [Linux TDX Guest Support](https://www.kernel.org/doc/html/latest/x86/tdx.html)

## License

Copyright 2025 RISC Zero, Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

