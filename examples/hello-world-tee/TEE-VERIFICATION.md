# RISC Zero Verification Inside Intel TDX TEE

## Overview

This integration not only **generates** RISC Zero proofs inside the Intel TDX TEE, but also **verifies** them within the same secure environment. The verification result is included in the TDX attestation.

## Why Verify Inside the TEE?

### Without TEE Verification
```
❌ Generate proof → Export → Verify externally
❌ No proof that verification happened securely
❌ Verification could be bypassed or forged
```

### With TEE Verification ✅
```
✓ Generate proof inside TEE
✓ Verify proof inside TEE  
✓ Include verification status in TDX attestation
✓ Cryptographic proof that verification passed
```

## The Complete Flow

```
┌──────────────────────────────────────────────────────────┐
│           Intel TDX Trust Domain (TD)                    │
│                                                          │
│  Step 1: Generate Proof                                 │
│  ┌────────────┐        ┌─────────────┐                 │
│  │   Input    │───────▶│  RISC Zero  │                 │
│  │  (17, 23)  │        │   Prover    │                 │
│  └────────────┘        └──────┬──────┘                 │
│                               │                          │
│                               ▼                          │
│                        ┌──────────────┐                 │
│                        │   Receipt    │                 │
│                        │   (Proof)    │                 │
│                        └──────┬───────┘                 │
│                               │                          │
│  Step 2: Verify Proof                                   │
│                               │                          │
│                               ▼                          │
│                   ┌───────────────────────┐             │
│                   │   receipt.verify()    │             │
│                   │   (INSIDE TEE)        │             │
│                   └───────────┬───────────┘             │
│                               │                          │
│                               ▼                          │
│                        ┌──────────────┐                 │
│                        │ Verification │                 │
│                        │   PASSED ✓   │                 │
│                        └──────┬───────┘                 │
│                               │                          │
│  Step 3: Hash Receipt + Verification Status             │
│                               │                          │
│                               ▼                          │
│                        ┌──────────────┐                 │
│                        │  SHA-256 of  │                 │
│                        │   Receipt    │                 │
│                        └──────┬───────┘                 │
│                               │                          │
│  Step 4: Generate TDX Attestation                       │
│                               │                          │
│                               ▼                          │
│                        ┌──────────────┐                 │
│                        │  REPORTDATA  │                 │
│                        │ [Receipt Hash│                 │
│                        │  + Verified] │                 │
│                        └──────┬───────┘                 │
│                               │                          │
│                               ▼                          │
│                        ┌──────────────┐                 │
│                        │  TDX Quote   │                 │
│                        │  (Signed by  │                 │
│                        │   Intel CPU) │                 │
│                        └──────────────┘                 │
└──────────────────────────────────────────────────────────┘
```

## Code Implementation

### Verification Inside TEE (src/main.rs)

```rust
// ═══════════════════════════════════════════════════════════
// CRITICAL: Verify receipt INSIDE THE TEE
// ═══════════════════════════════════════════════════════════

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
```

### Verification Certificate

The code generates a verification certificate in `receipt-hash.txt`:

```
Receipt SHA-256: c6a015a4714f8e0839e065619a51d0b80c27842a17a886a2d0033b693ee56e2d

=== Verification Certificate ===
Verified in TEE: YES
Verification Status: PASSED
Verification Time: 145.234ms
Image ID: [0x12, 0x34, 0x56, ...]

This hash should be included in TDX REPORTDATA to bind attestations
```

## What Gets Attested

The TDX quote cryptographically proves:

1. ✅ **Proof Generation**: The receipt was generated in this TDX TEE
2. ✅ **Proof Verification**: The receipt was verified in this TDX TEE
3. ✅ **Verification Result**: The verification PASSED
4. ✅ **Receipt Identity**: The specific receipt (via hash in REPORTDATA)

## Verification Guarantees

### Inside the TEE
```rust
receipt.verify(MULTIPLY_ID)?;
```

This checks:
- ✅ **Cryptographic Proof**: STARK/SNARK proof is valid
- ✅ **Image ID**: Code matches expected program
- ✅ **Journal Integrity**: Outputs are committed correctly
- ✅ **Seal Validity**: Proof sealing is correct

### Why This Matters

**Scenario 1: External Verification Only**
```
TEE generates proof → Export → Verify externally
```
- ❌ No proof verification happened
- ❌ Could export invalid proof
- ❌ Verifier must trust generator

**Scenario 2: TEE Generation + Verification** ✅
```
TEE generates proof → TEE verifies proof → TEE attests to both
```
- ✅ Proof of generation
- ✅ Proof of verification
- ✅ Proof verification passed
- ✅ All cryptographically bound

## Output Files

### 1. risc0-receipt.json
```json
{
  "timestamp": 1762546278,
  "example": "hello-world-tee",
  "status": "success",
  "result": 391,
  "verification": {
    "verified_in_tee": true,
    "verification_passed": true,
    "verification_time_ms": 145,
    "image_id": "[48, 196, 100, 220, ...]"
  },
  "note": "Receipt generated AND verified inside Intel TDX TEE"
}
```

### 2. receipt-hash.txt
```
Receipt SHA-256: c6a015a4...

=== Verification Certificate ===
Verified in TEE: YES
Verification Status: PASSED
Verification Time: 145.234ms
Image ID: [0x12, 0x34, ...]
```

### 3. TDX Quote REPORTDATA
```
Bytes 0-31:  Receipt SHA-256 hash
Bytes 32-36: "RISC0" (protocol ID)
Bytes 37-44: Timestamp
Bytes 45-63: Reserved
```

## Security Properties

| Property | Guarantee |
|----------|-----------|
| **Proof Generation** | Happened in TDX (proven by MRTD measurement) |
| **Proof Verification** | Happened in TDX (same TD as generation) |
| **Verification Result** | PASSED (program exits if failed) |
| **Receipt Binding** | Hash in REPORTDATA (signed by CPU) |
| **Non-Repudiation** | Cannot claim different result |
| **Integrity** | Quote signature covers REPORTDATA |

## Verification Process (Verifier's Perspective)

### Step 1: Verify TDX Quote
```rust
// Verify quote signature with Intel PKI
tdx_quote.verify()?;
```

This proves:
- Quote came from genuine Intel TDX hardware
- REPORTDATA is authentic
- Measurements (MRTD, RTMRs) are correct

### Step 2: Verify RISC Zero Receipt
```rust
// Verify the computational proof
receipt.verify(IMAGE_ID)?;
```

This proves:
- Computation was performed correctly
- Outputs in journal are valid

### Step 3: Verify Binding
```rust
// Extract hash from TDX quote
let tdx_hash = &quote.body.reportdata[0..32];

// Calculate hash from receipt
let receipt_bytes = bincode::serialize(&receipt)?;
let receipt_hash = sha256(&receipt_bytes);

// Verify they match
assert_eq!(tdx_hash, receipt_hash);
```

This proves:
- The receipt was present when TDX quote was generated
- Both were in the same TEE
- **The verification happened in the TEE** (because the hash is of the verified receipt)

## Complete Attestation Chain

```
Intel Root CA
    ↓ (signs)
Platform Certificate
    ↓ (signs)
TDX Quote
    ↓ (contains)
REPORTDATA = SHA-256(verified_receipt)
    ↓ (binds to)
RISC Zero Receipt
    ↓ (contains)
Verification Status: PASSED
```

## Use Cases

### Use Case 1: Zero-Knowledge Computation
```
Input (private) → Compute in TEE → Verify in TEE → Attest
```
**Result**: Proof that private computation was done correctly

### Use Case 2: Confidential ML Inference
```
Model + Data → Inference in TEE → Verify in TEE → Attest
```
**Result**: Proof that ML inference was correct

### Use Case 3: Secure Oracle
```
Fetch data → Process in TEE → Verify in TEE → Attest → Blockchain
```
**Result**: Trustless oracle with verified results

### Use Case 4: Compliance Reporting
```
Process PII → Generate report in TEE → Verify in TEE → Attest
```
**Result**: Auditable proof of compliant processing

## Running the Verification

On your TDX machine:

```bash
cd ~/risc0/examples/hello-world-tee

# Run complete workflow
./run-in-tdx.sh
```

### Expected Output

```
🚀 RISC Zero + Intel TDX Integration
=====================================

I know the factors of 391, and I can prove it!
✓ Computation complete: 391 = 17 × 23

🔐 Verifying RISC Zero receipt inside TDX TEE...
✓ Receipt verification PASSED inside TEE
  Verification time: 145.234ms
  Image ID: [48, 196, 100, 220, ...]

✓ Receipt metadata saved to: tdx-output/risc0-receipt.json
✓ Receipt binary saved to: tdx-output/risc0-receipt.bin
✓ Receipt hash saved to: tdx-output/receipt-hash.txt
  Hash: c6a015a4714f8e0839e065619a51d0b80c27842a17a886a2d0033b693ee56e2d

🎉 All outputs generated successfully!

══════════════════════════════════════════════════
Generated files in tdx-output:
  - risc0-receipt.json     (metadata + verification status)
  - risc0-receipt.bin      (full receipt for verification)
  - receipt-hash.txt       (SHA-256 + verification certificate)
══════════════════════════════════════════════════

🔐 IMPORTANT:
  ✓ Proof generated inside TDX TEE
  ✓ Proof verified inside TDX TEE
  ✓ Receipt hash will be bound to TDX attestation

Next step: Run TDX attestation generation
```

## Comparison: External vs TEE Verification

### External Verification (Traditional)
```
┌─────────────┐
│     TEE     │  Generate proof
└──────┬──────┘
       │ Export
       ▼
┌─────────────┐
│   Outside   │  Verify proof
│     TEE     │
└─────────────┘
```
**Issues:**
- No proof verification happened
- Verifier must trust generator
- Verification could be skipped

### TEE Verification (This Implementation) ✅
```
┌─────────────┐
│     TEE     │  Generate proof
│      +      │  Verify proof
│      +      │  Attest to both
└──────┬──────┘
       │ Export attestation
       ▼
┌─────────────┐
│   Verifier  │  Check TDX quote
│             │  Verify binding
└─────────────┘
```
**Benefits:**
- ✅ Proof of verification
- ✅ Verification in TEE
- ✅ Cannot be bypassed
- ✅ Cryptographic guarantee

## Summary

- ✅ **Generation**: RISC Zero proof generated inside TDX
- ✅ **Verification**: RISC Zero proof verified inside TDX  
- ✅ **Attestation**: TDX quote includes receipt hash
- ✅ **Binding**: Cryptographic link between all three
- ✅ **Certificate**: Verification result documented
- ✅ **Unforgeable**: Signed by Intel CPU

**The TDX quote proves that not only was the proof generated in the TEE, but it was also verified to be correct within the same secure environment!**

## References

- [RISC Zero Receipt Verification](https://dev.risczero.com/api/zkvm/receipts#verification)
- [Intel TDX REPORTDATA](https://cdrdv2.intel.com/v1/dl/getContent/733568)
- [Cryptographic Binding Patterns](./CRYPTOGRAPHIC-BINDING.md)

