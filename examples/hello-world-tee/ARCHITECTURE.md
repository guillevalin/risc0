# TDX + RISC Zero Architecture

## System Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                        Intel TDX Trust Domain                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                     User Application                          │  │
│  │  ┌────────────────┐         ┌─────────────────┐             │  │
│  │  │  Host Program  │────────▶│  Guest Program  │             │  │
│  │  │   (main.rs)    │         │  (zkVM code)    │             │  │
│  │  └────────┬───────┘         └────────┬────────┘             │  │
│  │           │                          │                       │  │
│  │           │                          │                       │  │
│  │           ▼                          ▼                       │  │
│  │  ┌────────────────┐         ┌─────────────────┐             │  │
│  │  │ RISC Zero      │         │  Computation    │             │  │
│  │  │ Prover         │◀────────│  Execution      │             │  │
│  │  └────────┬───────┘         └─────────────────┘             │  │
│  │           │                                                  │  │
│  │           ▼                                                  │  │
│  │  ┌────────────────┐                                         │  │
│  │  │  RISC Zero     │                                         │  │
│  │  │  Receipt       │                                         │  │
│  │  └────────┬───────┘                                         │  │
│  └───────────┼──────────────────────────────────────────────────┘  │
│              │                                                     │
│              ▼                                                     │
│     ┌────────────────┐                                            │
│     │  Receipt Hash  │                                            │
│     └────────┬───────┘                                            │
│              │                                                     │
│              ▼                                                     │
│     ┌────────────────┐         TDX Runtime                        │
│     │  REPORTDATA    │◀────────(Measurements)                     │
│     │  (64 bytes)    │                                            │
│     └────────┬───────┘                                            │
│              │                                                     │
│              ▼                                                     │
│     ┌────────────────┐                                            │
│     │  TDX Report    │                                            │
│     │  (1024 bytes)  │                                            │
│     └────────┬───────┘                                            │
└──────────────┼────────────────────────────────────────────────────┘
               │
               ▼
    ┌────────────────────┐
    │  Quoting Enclave   │
    │  (Intel SGX)       │
    └──────────┬─────────┘
               │
               ▼
    ┌────────────────────┐
    │   TDX Quote        │
    │   (Signed)         │
    └──────────┬─────────┘
               │
               │
┌──────────────┴───────────────────────────────────────────────────┐
│                    Verification (Off-TEE)                         │
│                                                                   │
│  ┌──────────────────┐              ┌──────────────────┐         │
│  │  Quote Verifier  │              │ Receipt Verifier │         │
│  │  (Intel/Cloud)   │              │  (RISC Zero)     │         │
│  └────────┬─────────┘              └────────┬─────────┘         │
│           │                                 │                    │
│           ▼                                 ▼                    │
│  ┌──────────────────┐              ┌──────────────────┐         │
│  │ - Verify Sig     │              │ - Verify Proof   │         │
│  │ - Check TCB      │              │ - Check Journal  │         │
│  │ - Validate Cert  │              │ - Verify ImageID │         │
│  │ - Extract Report │              │                  │         │
│  └────────┬─────────┘              └────────┬─────────┘         │
│           │                                 │                    │
│           └──────────────┬──────────────────┘                    │
│                          ▼                                       │
│               ┌───────────────────┐                              │
│               │  Attestation OK   │                              │
│               │ - Hardware: ✓     │                              │
│               │ - Computation: ✓  │                              │
│               └───────────────────┘                              │
└───────────────────────────────────────────────────────────────────┘
```

## Component Interaction

### 1. Execution Phase (Inside TDX)

```
┌─────────────────────────────────────────────────────────────┐
│  Step 1: Initialize TDX Trust Domain                        │
│  ┌──────────────────────────────────────────────────┐       │
│  │ - Load TD (Trust Domain)                         │       │
│  │ - Measure initial state → MRTD                   │       │
│  │ - Initialize RTMRs (Runtime Measurement Registers│       │
│  └──────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Step 2: Execute RISC Zero Computation                      │
│  ┌──────────────────────────────────────────────────┐       │
│  │ - Load guest program (ELF)                       │       │
│  │ - Execute in zkVM                                │       │
│  │ - Generate proof of execution                    │       │
│  │ - Create receipt with journal                    │       │
│  └──────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Step 3: Bind Attestations                                  │
│  ┌──────────────────────────────────────────────────┐       │
│  │ - Serialize RISC Zero receipt                    │       │
│  │ - Compute SHA-256 hash of receipt                │       │
│  │ - Place hash in TDX REPORTDATA                   │       │
│  └──────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Step 4: Generate TDX Report                                │
│  ┌──────────────────────────────────────────────────┐       │
│  │ - Call TDCALL[TDG.MR.REPORT]                     │       │
│  │ - Include REPORTDATA (with receipt hash)         │       │
│  │ - Get report with MRTD, RTMRs, etc.              │       │
│  └──────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Step 5: Generate TDX Quote                                 │
│  ┌──────────────────────────────────────────────────┐       │
│  │ - Send report to Quoting Enclave (QE)           │       │
│  │ - QE signs report with attestation key           │       │
│  │ - Attach certificate chain                       │       │
│  │ - Return signed quote                            │       │
│  └──────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

### 2. Verification Phase (Outside TDX)

```
┌─────────────────────────────────────────────────────────────┐
│  Verifier receives:                                         │
│  - TDX Quote                                                │
│  - RISC Zero Receipt                                        │
│  - Attestation Bundle (metadata)                            │
└─────────────────────────────────────────────────────────────┘
                           ↓
          ┌────────────────┴────────────────┐
          ↓                                 ↓
┌──────────────────────┐        ┌──────────────────────┐
│  Verify TDX Quote    │        │ Verify RISC Zero     │
│                      │        │ Receipt              │
│  1. Verify signature │        │                      │
│     with Intel PKI   │        │ 1. Call receipt.     │
│                      │        │    verify(IMAGE_ID)  │
│  2. Check TCB level  │        │                      │
│                      │        │ 2. Verify proof      │
│  3. Validate cert    │        │    integrity         │
│     chain to Intel   │        │                      │
│     root CA          │        │ 3. Decode journal    │
│                      │        │    (outputs)         │
│  4. Extract report   │        │                      │
│     from quote       │        │ 4. Check expected    │
│                      │        │    results           │
│  5. Get REPORTDATA   │        │                      │
│     (receipt hash)   │        │ 5. Compute receipt   │
│                      │        │    hash              │
└──────────┬───────────┘        └──────────┬───────────┘
           │                               │
           └───────────┬───────────────────┘
                       ↓
           ┌───────────────────────┐
           │  Compare Hashes       │
           │                       │
           │  TDX REPORTDATA[0:32] │
           │         ==            │
           │  SHA256(receipt)      │
           └───────────┬───────────┘
                       ↓
           ┌───────────────────────┐
           │   Attestation Valid   │
           │                       │
           │  ✓ Hardware verified  │
           │  ✓ Computation proven │
           │  ✓ Cryptographically  │
           │    bound              │
           └───────────────────────┘
```

## Data Structures

### TDX Report Structure (1024 bytes)

```
┌─────────────────────────────────────────┐
│  REPORTDATA (64 bytes)                  │  ← Receipt hash goes here
├─────────────────────────────────────────┤
│  REPORTMAC (32 bytes)                   │  ← Integrity protection
├─────────────────────────────────────────┤
│  TEE TCB INFO (112 bytes)               │
│  - TEE_TCB_SVN                          │
│  - MRSEAM / MRSIGNERSEAM                │
│  - SEAMATTRIBUTES                       │
├─────────────────────────────────────────┤
│  MRTD (48 bytes)                        │  ← Measurement of TD
├─────────────────────────────────────────┤
│  MRCONFIGID (48 bytes)                  │
├─────────────────────────────────────────┤
│  MROWNER (48 bytes)                     │
├─────────────────────────────────────────┤
│  MROWNERCONFIG (48 bytes)               │
├─────────────────────────────────────────┤
│  RTMR[0] (48 bytes)                     │  ← Runtime measurements
├─────────────────────────────────────────┤
│  RTMR[1] (48 bytes)                     │
├─────────────────────────────────────────┤
│  RTMR[2] (48 bytes)                     │
├─────────────────────────────────────────┤
│  RTMR[3] (48 bytes)                     │
├─────────────────────────────────────────┤
│  ... (additional fields)                │
└─────────────────────────────────────────┘
```

### TDX Quote Structure

```
┌─────────────────────────────────────────┐
│  Quote Header                           │
│  - Version (4 bytes)                    │
│  - Attestation Key Type (2 bytes)       │
│  - TEE Type = TDX (4 bytes)             │
│  - QE SVN (2 bytes)                     │
│  - PCE SVN (2 bytes)                    │
│  - QE Vendor ID (16 bytes)              │
│  - User Data (20 bytes)                 │
├─────────────────────────────────────────┤
│  TD Report Body (584 bytes)             │
│  - Contains the full TDX report         │
├─────────────────────────────────────────┤
│  Quote Signature Data                   │
│  - Signature (64 bytes, ECDSA-256)      │
│  - Public Key (64 bytes)                │
│  - Certification Data (variable)        │
│    * PCK Certificate                    │
│    * PCK Certificate Chain              │
│    * QE Report & Signature              │
│    * TCB Info                           │
├─────────────────────────────────────────┤
│  QE Report                              │
│  - Attests to Quoting Enclave           │
└─────────────────────────────────────────┘
```

### RISC Zero Receipt Structure

```
┌─────────────────────────────────────────┐
│  Journal                                │
│  - Committed outputs from guest         │
│  - Can be decoded to expected types     │
├─────────────────────────────────────────┤
│  Seal                                   │
│  - Contains the actual proof            │
│  - Cryptographic evidence               │
│  - Can be Groth16, STARK, etc.          │
├─────────────────────────────────────────┤
│  Metadata                               │
│  - Image ID (program identifier)        │
│  - Prover info                          │
│  - Verification keys                    │
└─────────────────────────────────────────┘
```

## Security Model

### Trust Anchors

```
┌─────────────────────────────────────────────────────────┐
│                    Intel Root CA                         │
│              (Hardcoded in verifiers)                    │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│              Intel SGX Intermediate CA                   │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│         Platform Certification Key (PCK) Cert            │
│                 (Per-CPU certificate)                    │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                    TDX Quote                             │
│              (Signed by PCK key)                         │
└─────────────────────────────────────────────────────────┘


┌─────────────────────────────────────────────────────────┐
│             RISC Zero Image ID                           │
│         (Hash of guest program code)                     │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│              RISC Zero Receipt                           │
│         (Proof that program executed)                    │
└─────────────────────────────────────────────────────────┘
```

### Threat Model

| Threat | Mitigation |
|--------|------------|
| **Malicious Host OS** | TDX isolates TD from host, measurements in MRTD |
| **Memory Snooping** | TDX encrypts TD memory with CPU-generated keys |
| **Forged Attestation** | TDX quote signed by CPU, verified against Intel PKI |
| **Modified Guest Code** | MRTD measurement changes if code modified |
| **Incorrect Computation** | RISC Zero proof verifies correct execution |
| **Replay Attacks** | Include timestamps in REPORTDATA |
| **Man-in-the-Middle** | TLS with quote-bound keys, verify quote freshness |

## Implementation Details

### File Flow

```
run-in-tdx.sh execution:

1. check_tdx_environment()
   ↓
   Verify /dev/tdx_guest exists
   ↓
2. build_example()
   ↓
   cargo build --release --features prove
   ↓
3. run_risc0_example()
   ↓
   Execute computation → Generate receipt
   ↓
4. generate_tdx_report()
   ↓
   Python script → ioctl(/dev/tdx_guest) → tdx-report.bin
   ↓
5. generate_tdx_quote()
   ↓
   configfs-tsm or attestation service → tdx-quote.bin
   ↓
6. generate_combined_report()
   ↓
   Create attestation-bundle.json with all metadata
```

### Verification Flow

```
verify-attestation.sh execution:

1. verify_tdx_report()
   ↓
   Check file size (1024 bytes)
   ↓
   Display structure info
   ↓
2. verify_tdx_quote()
   ↓
   Try tdx-verify or dcap-verify tools
   ↓
   If not available, show manual instructions
   ↓
3. verify_risc0_receipt()
   ↓
   Check execution logs for verification results
   ↓
4. display_attestation_bundle()
   ↓
   Show combined attestation metadata
```

## Performance Characteristics

| Operation | Time | Notes |
|-----------|------|-------|
| TDX Boot | ~500ms | One-time TD initialization |
| RISC Zero Proof | 1-10s | Depends on computation complexity |
| TDX Report Gen | <1ms | Fast, local operation |
| TDX Quote Gen | 100-500ms | Requires QE communication |
| Quote Verification | 100-200ms | Network + crypto operations |
| Receipt Verification | 10-100ms | Depends on proof type |

## Deployment Options

### 1. Google Cloud Platform
- Native TDX support on N2D instances
- Integrated attestation services
- Easy deployment with `deploy-to-gcp.sh`

### 2. Azure Confidential Computing
- DCasv5/ECasv5 VM series with TDX
- Azure Attestation service integration
- Enterprise support

### 3. On-Premises
- Requires TDX-capable hardware (4th Gen Xeon+)
- Self-hosted verification with Intel DCAP
- Full control over infrastructure

### 4. Docker/Kubernetes
- Container-based deployment
- Requires host TDX support
- See `Dockerfile.tdx`

## Future Enhancements

1. **Continuous Attestation**: Periodic re-attestation for long-running services
2. **Key Provisioning**: Derive keys from TDX seal keys
3. **Sealed Storage**: Encrypt data that only this TD can decrypt
4. **Network Attestation**: TLS with quote-bound certificates
5. **Multi-Party Computation**: Multiple TDs with mutual attestation
6. **Blockchain Integration**: Submit quotes and receipts on-chain

## References

- [Intel TDX Architecture](https://cdrdv2.intel.com/v1/dl/getContent/690419)
- [TDX Module Spec](https://cdrdv2.intel.com/v1/dl/getContent/733568)
- [RISC Zero Architecture](https://dev.risczero.com/zkvm)
- [Linux TDX Support](https://www.kernel.org/doc/html/latest/x86/tdx.html)

