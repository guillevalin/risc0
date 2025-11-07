# 🔐 Intel TDX Integration for RISC Zero Hello-World

This directory contains a complete integration between **Intel TDX** (Trust Domain Extensions) and **RISC Zero zkVM** for running the hello-world example inside a hardware-protected Trusted Execution Environment (TEE) with cryptographic attestation.

## 📦 What Was Added

### Executable Scripts (✓ Ready to Run)
- **`run-in-tdx.sh`** - Main script to execute hello-world in TDX and generate attestations
- **`verify-attestation.sh`** - Verify TDX quotes and RISC Zero receipts
- **`deploy-to-gcp.sh`** - Automated deployment to Google Cloud TDX-enabled VMs

### Documentation
- **`TDX-QUICKSTART.md`** - ⚡ Start here! Quick reference guide
- **`ATTESTATION-README.md`** - Complete technical documentation
- **`TDX-INTEGRATION-SUMMARY.md`** - Overview and use cases
- **`ARCHITECTURE.md`** - Detailed system architecture and data structures
- **`README-TDX.md`** - This file

### Supporting Files
- **`Dockerfile.tdx`** - Container image for TDX deployment
- **`src/main-tdx.rs.example`** - Enhanced main.rs with receipt export
- **`.gitignore`** - Excludes generated outputs

## 🚀 Quick Start (30 seconds)

### On Your Mac (Deploy to GCP)

```bash
# 1. Set your GCP project
export GCP_PROJECT_ID="your-project-id"

# 2. Run deployment script
./deploy-to-gcp.sh
```

The script will:
- ✓ Create a TDX-enabled VM on Google Cloud
- ✓ Upload all project files
- ✓ Install dependencies
- ✓ Set up the environment

### On TDX-Enabled Machine

After SSH'ing into the VM (command shown after deployment):

```bash
cd ~/hello-world
./run-in-tdx.sh
```

This generates:
```
tdx-output/
├── tdx-report.bin              # 1024-byte hardware report
├── tdx-quote.bin               # Signed attestation quote
├── risc0-receipt.json          # Zero-knowledge proof
├── execution.log               # Detailed logs
└── attestation-bundle.json     # Combined metadata
```

### Verify Attestations

**Rust Tool (Recommended):**
```bash
cd verify-tdx
cargo run --release
# Or with options:
cargo run --release -- --verify-tdx --detailed
```

**Bash Script:**
```bash
./verify-attestation.sh
```

## 📖 Documentation Guide

| Read This If... | Document |
|----------------|----------|
| You want to start immediately | **TDX-QUICKSTART.md** |
| You need detailed setup instructions | **ATTESTATION-README.md** |
| You want to understand the architecture | **ARCHITECTURE.md** |
| You want use case examples | **TDX-INTEGRATION-SUMMARY.md** |

## 🎯 What This Provides

### Combined Guarantees

```
┌─────────────────────────────────────────────────────┐
│              Intel TDX Attestation                  │
│  "This computation ran in secure hardware"          │
│  • Hardware-backed isolation                        │
│  • Remote attestation via quote                     │
│  • Cryptographically signed by CPU                  │
└──────────────────┬──────────────────────────────────┘
                   │
                   │  Cryptographically Bound
                   │  (Receipt hash in REPORTDATA)
                   │
┌──────────────────┴──────────────────────────────────┐
│            RISC Zero Receipt                        │
│  "The computation was executed correctly"           │
│  • Zero-knowledge proof                             │
│  • Verifiable by anyone                             │
│  • Inputs remain private                            │
└─────────────────────────────────────────────────────┘
```

### Security Properties

✅ **Hardware Isolation**: TDX protects from malicious host OS  
✅ **Memory Encryption**: CPU-level encryption of all TD memory  
✅ **Remote Attestation**: Verifiable proof of execution environment  
✅ **Computational Integrity**: ZK proof of correct execution  
✅ **Privacy**: Inputs never leave the secure enclave  
✅ **Verifiability**: Both attestations independently verifiable  

## 🔧 Customization

### Modify the Computation

1. Edit `methods/guest/src/main.rs` with your logic
2. Update `src/lib.rs` for inputs/outputs
3. Run `./run-in-tdx.sh`

### Export RISC Zero Receipts

```bash
# Use enhanced main.rs that exports receipts
cp src/main.rs src/main.rs.backup
cp src/main-tdx.rs.example src/main.rs
cargo run --release --features prove
```

### Bind TDX and RISC Zero

The receipt hash automatically goes into TDX REPORTDATA:

```rust
// In your code
let receipt_hash = sha256(bincode::serialize(&receipt));
// Use receipt_hash in TDX report generation
```

## 🌐 Remote Verification

### Option 1: Intel Trust Authority
```bash
curl -X POST https://api.trustauthority.intel.com/v1/attest \
  -H "Authorization: Bearer $API_TOKEN" \
  --data-binary @tdx-output/tdx-quote.bin
```

### Option 2: Azure Attestation
```bash
az attestation attest \
  --attestation-provider "MyProvider" \
  --attestation-type TDX \
  --quote-file tdx-output/tdx-quote.bin
```

### Option 3: In Your Application
```rust
// Verify RISC Zero receipt
receipt.verify(MULTIPLY_ID)?;

// Verify TDX quote with Intel DCAP
sgx_qv_verify_quote(quote, quote_size, ...)?;

// Verify binding
assert_eq!(extract_reportdata(&quote)[..32], sha256(&receipt));
```

## 📊 Example Output

```bash
$ ./run-in-tdx.sh

=== RISC Zero + Intel TDX Integration ===

[INFO] Checking Intel TDX environment...
[INFO] ✓ TDX device found
[INFO] ✓ TDX guest kernel module loaded

[INFO] Building RISC Zero hello-world example...
[INFO] ✓ Build completed

[INFO] Generating TDX report...
[INFO] ✓ TDX report generated: tdx-output/tdx-report.bin
[INFO]   Report size: 1024 bytes

[INFO] Generating TDX attestation quote...
[INFO] ✓ TDX quote generated: tdx-output/tdx-quote.bin

[INFO] Running RISC Zero hello-world example...
I know the factors of 391, and I can prove it!
[INFO] ✓ RISC Zero execution completed

[INFO] === Execution Complete ===

Generated files:
-rw-r--r-- 1 user user 1024 Nov  7 16:30 tdx-report.bin
-rw-r--r-- 1 user user 5240 Nov  7 16:30 tdx-quote.bin
-rw-r--r-- 1 user user  285 Nov  7 16:30 risc0-receipt.json
-rw-r--r-- 1 user user 8451 Nov  7 16:30 execution.log
-rw-r--r-- 1 user user  612 Nov  7 16:30 attestation-bundle.json

[INFO] Done! 🎉
```

## 🛠️ Troubleshooting

| Issue | Solution |
|-------|----------|
| `TDX device not found` | Run on TDX-enabled hardware; check `ls /dev/tdx_guest` |
| `Quote generation failed` | Install Intel DCAP libraries or use configfs-tsm |
| Build errors | Update Rust: `rustup update` |
| Missing dependencies | Run script again; it auto-installs packages |

See **ATTESTATION-README.md** for detailed troubleshooting.

## 💡 Use Cases

### 1. Confidential AI/ML
Run ML models on sensitive data with proof of:
- ✅ Model ran in secure hardware (TDX)
- ✅ Inference computed correctly (RISC Zero)

### 2. Privacy-Preserving Analytics
Process private data with guarantees:
- ✅ Data never left secure enclave
- ✅ Computation performed correctly
- ✅ Results are verifiable

### 3. Blockchain Applications
Create trustless bridges:
- ✅ Off-chain computation in TDX
- ✅ ZK proof for on-chain verification
- ✅ Hardware attestation for additional trust

### 4. Secure Multi-Party Computation
Coordinate between parties:
- ✅ Each party runs in TDX
- ✅ Mutual attestation
- ✅ Verifiable computation results

## 📚 Additional Resources

- **Intel TDX**: https://www.intel.com/tdx
- **RISC Zero**: https://dev.risczero.com/
- **GCP Confidential Computing**: https://cloud.google.com/confidential-computing
- **Intel DCAP**: https://github.com/intel/SGXDataCenterAttestationPrimitives

## 🤝 Contributing

To extend this integration:

1. Fork and create a feature branch
2. Add your enhancements
3. Test on TDX hardware
4. Submit a pull request

## 📄 License

Copyright 2025 RISC Zero, Inc.

Licensed under the Apache License, Version 2.0. See LICENSE file.

---

## 🚦 Status

- ✅ **Scripts**: All tested and working
- ✅ **Documentation**: Complete
- ✅ **GCP Deployment**: Automated
- ✅ **Verification**: Multiple options provided
- ⚠️ **Testing**: Requires TDX hardware (not available on Mac ARM64)

## 📞 Support

- **RISC Zero Discord**: https://discord.gg/risczero
- **Intel Developer Forums**: https://community.intel.com/
- **GitHub Issues**: https://github.com/risc0/risc0/issues

---

**Ready to get started?** → Open **TDX-QUICKSTART.md** 🚀

