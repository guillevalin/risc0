# TDX Quick Start Guide

## 🚀 Fast Track: Deploy and Run

### On Google Cloud (Recommended)

```bash
# 1. Set your GCP project
export GCP_PROJECT_ID="your-project-id"

# 2. Deploy (from your Mac)
chmod +x deploy-to-gcp.sh
./deploy-to-gcp.sh

# 3. The script will:
#    - Create a TDX-enabled VM
#    - Upload all files
#    - Set up the environment

# 4. SSH into the VM (command will be shown after deployment)
gcloud compute ssh risc0-tdx-vm --zone=us-central1-a

# 5. Run the example (on the VM)
cd ~/hello-world
./run-in-tdx.sh

# 6. Verify attestations (on the VM)
./verify-attestation.sh

# 7. Copy results back (from your Mac)
gcloud compute scp --recurse \
  risc0-tdx-vm:~/hello-world/tdx-output \
  ./ \
  --zone=us-central1-a
```

### Manual Setup on TDX-Enabled Machine

```bash
# 1. Make scripts executable
chmod +x run-in-tdx.sh verify-attestation.sh

# 2. Run the example
./run-in-tdx.sh

# 3. Verify results
./verify-attestation.sh

# 4. Check output
ls -lh tdx-output/
```

## 📦 What Gets Generated?

```
tdx-output/
├── tdx-report.bin              # TDX hardware report (1024 bytes)
├── tdx-quote.bin               # Signed attestation quote
├── risc0-receipt.json          # Zero-knowledge proof receipt
├── execution.log               # Detailed logs
└── attestation-bundle.json     # Combined metadata
```

## 🔍 Verification

### Local Verification

**Option 1: Rust Tool** (Recommended)
```bash
cd verify-tdx
cargo run --release
```

**Option 2: Bash Script**
```bash
./verify-attestation.sh
```

### Remote Verification

#### Intel Trust Authority
```bash
curl -X POST https://api.trustauthority.intel.com/v1/attest \
  -H "Authorization: Bearer $INTEL_API_TOKEN" \
  --data-binary @tdx-output/tdx-quote.bin
```

#### Azure Attestation
```bash
az attestation attest \
  --attestation-provider "MyProvider" \
  --attestation-type TDX \
  --quote-file tdx-output/tdx-quote.bin
```

## 🐳 Using Docker

```bash
# Build image
docker build -f Dockerfile.tdx -t risc0-tdx .

# Run (requires TDX support on host)
docker run --device=/dev/tdx_guest \
  -v $(pwd)/tdx-output:/app/tdx-output \
  risc0-tdx

# Extract results
ls -lh tdx-output/
```

## 📊 Understanding the Output

### TDX Report
- **Purpose**: Cryptographic snapshot of TEE state
- **Size**: 1024 bytes
- **Contains**: MRTD, RTMRs, TCB info, custom REPORTDATA
- **Verifiable**: Against Intel's cryptographic keys

### TDX Quote
- **Purpose**: Remotely verifiable attestation
- **Contains**: Report + signatures + certificate chain
- **Verification**: Through Intel services or DCAP libraries

### RISC Zero Receipt
- **Purpose**: Proof of correct computation
- **Contains**: Journal (outputs) + cryptographic proof
- **Verification**: Using RISC Zero's verify() function

## 🔗 Binding Attestations

The TDX REPORTDATA can be bound to the RISC Zero receipt:

```
┌─────────────────────┐
│  RISC Zero Receipt  │
│  (Computation Proof)│
└──────────┬──────────┘
           │
           ▼
      SHA-256 Hash
           │
           ▼
┌─────────────────────┐
│   TDX REPORTDATA    │
│   (first 32 bytes)  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│     TDX Quote       │
│ (Hardware Attestation)│
└─────────────────────┘
```

This creates a cryptographic link:
- TDX proves: "This specific computation ran in secure hardware"
- RISC Zero proves: "The computation was executed correctly"

## 🛠️ Troubleshooting

### "TDX device not found"
```bash
# Check kernel module
lsmod | grep tdx_guest

# Load module
sudo modprobe tdx_guest

# Verify device
ls -la /dev/tdx_guest
```

### "Quote generation failed"
```bash
# Check for attestation service
ls -la /var/run/tdx-attest.sock
ls -la /sys/kernel/config/tsm/report/

# Check kernel version (needs 5.19+)
uname -r
```

### Build errors
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release --features prove
```

## 📚 Next Steps

1. **Customize**: Modify `src/main.rs` to run your own computation
2. **Integrate**: Add TDX attestation to your application
3. **Deploy**: Use the GCP deployment script for production
4. **Verify**: Set up remote verification with Intel or cloud providers

## 📖 Full Documentation

See `ATTESTATION-README.md` for:
- Detailed architecture
- Security considerations
- Advanced configuration
- API integration examples
- Troubleshooting guide

## 🔐 Security Notes

- Always verify TDX quotes against Intel's PKI
- Check TCB (Trusted Computing Base) status
- Validate MRTD matches expected measurements
- Include timestamps to prevent replay attacks
- Use production-grade key management for sensitive operations

## 💡 Example Use Cases

1. **Confidential ML**: Run machine learning models in TDX, prove correct inference with RISC Zero
2. **Private Computation**: Process sensitive data in TDX, generate verifiable results
3. **Blockchain Integration**: Create trustless bridges using TDX + ZK proofs
4. **Secure Enclaves**: Build TEE applications with hardware and computational guarantees

## 🆘 Support

- RISC Zero Docs: https://dev.risczero.com/
- Intel TDX Docs: https://www.intel.com/tdx
- GitHub Issues: https://github.com/risc0/risc0/issues

