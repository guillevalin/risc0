# Cryptographic Binding: Proving RISC Zero Was Executed in TDX

## The Problem

How do you prove that a RISC Zero proof was actually generated inside an Intel TDX TEE, and not just copied from somewhere else?

## The Solution: Cryptographic Binding

We bind the RISC Zero receipt to the TDX attestation by including the receipt's SHA-256 hash in the TDX `REPORTDATA` field.

### Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Step 1: Generate RISC Zero Proof Inside TDX                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐         ┌───────────────┐                │
│  │  Guest Code  │────────▶│  RISC Zero    │                │
│  │  (multiply)  │         │  Prover       │                │
│  └──────────────┘         └───────┬───────┘                │
│                                    │                         │
│                                    ▼                         │
│                           ┌────────────────┐                │
│                           │  Receipt       │                │
│                           │  (Proof)       │                │
│                           └────────┬───────┘                │
│                                    │                         │
│                                    ▼                         │
│                           ┌────────────────┐                │
│                           │  SHA-256 Hash  │                │
│                           │  (32 bytes)    │                │
│                           └────────┬───────┘                │
└────────────────────────────────────┼────────────────────────┘
                                     │
┌────────────────────────────────────┼────────────────────────┐
│ Step 2: Bind Hash to TDX Attestation                       │
├────────────────────────────────────┼────────────────────────┤
│                                    │                         │
│                                    ▼                         │
│                           ┌────────────────┐                │
│                           │  REPORTDATA    │                │
│                           │  [0:32] = hash │                │
│                           │  [32:64] = meta│                │
│                           └────────┬───────┘                │
│                                    │                         │
│                                    ▼                         │
│                           ┌────────────────┐                │
│                           │  TDX Report    │                │
│                           │  (TDCALL)      │                │
│                           └────────┬───────┘                │
│                                    │                         │
│                                    ▼                         │
│                           ┌────────────────┐                │
│                           │  TDX Quote     │                │
│                           │  (Signed by    │                │
│                           │   Intel CPU)   │                │
│                           └────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

## Verification Process

A verifier receives two artifacts:
1. **TDX Quote** - Hardware attestation (signed by Intel CPU)
2. **RISC Zero Receipt** - Computational proof

### Verification Steps

```rust
// 1. Verify the TDX quote signature
tdx_quote.verify()?;  // Verifies against Intel PKI

// 2. Verify the RISC Zero receipt
receipt.verify(IMAGE_ID)?;  // Verifies the computation proof

// 3. Extract REPORTDATA from TDX quote
let reportdata = tdx_quote.body.reportdata;  // 64 bytes
let tdx_hash = &reportdata[0..32];  // First 32 bytes

// 4. Calculate receipt hash
let receipt_bytes = bincode::serialize(&receipt)?;
let receipt_hash = sha256(&receipt_bytes);

// 5. Verify binding
if tdx_hash == receipt_hash {
    println!("✓ BINDING VERIFIED!");
    println!("The RISC Zero proof was generated inside this specific TDX environment");
} else {
    println!("✗ BINDING FAILED - Receipt does not match TDX attestation");
}
```

## What This Proves

### Without Binding
- ❌ **Separate attestations**: You have a TDX quote AND a RISC Zero receipt, but no proof they're related
- ❌ **Vulnerable**: Someone could take a TDX quote from one machine and a receipt from another

### With Binding
- ✅ **Cryptographically linked**: The TDX quote contains the receipt hash
- ✅ **Non-repudiable**: The quote is signed by Intel's CPU with the receipt hash inside
- ✅ **Unforgeable**: Cannot create a TDX quote with a different receipt hash without access to that specific TDX TEE

## Security Properties

### 1. Integrity
The TDX quote is signed by the CPU's attestation key. Any modification to:
- The REPORTDATA (including receipt hash)
- The TD measurements (MRTD, RTMRs)
- Any other quote fields

...will invalidate the signature.

### 2. Authenticity
The signature chain links back to Intel's root CA, proving:
- The quote came from genuine Intel hardware
- The hardware has TDX enabled
- The TCB (Trusted Computing Base) is up to date

### 3. Freshness
The REPORTDATA includes a timestamp, preventing replay attacks where an old quote is reused.

### 4. Binding
The receipt hash in REPORTDATA proves:
- This specific receipt was present when the quote was generated
- The receipt was generated in this TDX environment
- No other receipt can be substituted

## Implementation Details

### REPORTDATA Structure (64 bytes)

```
Byte Range | Content                    | Description
-----------|----------------------------|----------------------------------
0-31       | Receipt SHA-256 hash       | Binds to RISC Zero receipt
32-36      | "RISC0" (ASCII)           | Protocol identifier
37-44      | Unix timestamp (uint64_t) | Freshness/replay protection
45-63      | Zero padding              | Reserved for future use
```

### Code in run-in-tdx.sh

```python
# First 32 bytes: RISC Zero receipt hash
receipt_hash_bytes = bytes.fromhex(receipt_hash_hex)

# Next 32 bytes: Metadata
timestamp = struct.pack('<Q', int(time.time()))
metadata = b'RISC0' + timestamp + b'\x00' * 19

# Combine into 64-byte REPORTDATA
report_data = receipt_hash_bytes + metadata
```

### Verification in verify-tdx tool

```rust
// Extract from TDX quote
let reportdata = &quote.body.reportdata[..32];

// Calculate from receipt
let receipt_binary = bincode::serialize(&receipt)?;
let mut hasher = Sha256::new();
hasher.update(&receipt_binary);
let receipt_hash = hasher.finalize();

// Compare
if reportdata == &receipt_hash[..] {
    println!("✓ BINDING VERIFIED");
}
```

## Real-World Usage

### Use Case 1: Confidential ML Inference

```
1. Load ML model into TDX
2. Run inference with private data
3. Generate RISC Zero proof of correct computation
4. Generate TDX quote with receipt hash in REPORTDATA
5. Client verifies:
   - TDX quote: Model ran in secure hardware
   - RISC Zero receipt: Inference was computed correctly
   - Binding: They're linked together
```

### Use Case 2: Blockchain Oracle

```
1. Fetch data from API inside TDX
2. Process data and generate RISC Zero proof
3. Generate TDX quote binding to receipt
4. Submit both to smart contract
5. Contract verifies:
   - TDX quote: Data fetched in TEE
   - RISC Zero receipt: Processing was correct
   - Binding: Proof they're connected
```

### Use Case 3: Regulatory Compliance

```
1. Process PII in TDX environment
2. Generate compliance report with RISC Zero
3. Generate TDX quote with receipt hash
4. Auditor verifies:
   - TDX quote: Data processed in compliant environment
   - RISC Zero receipt: Compliance rules followed
   - Binding: Proof of secure processing
```

## Verification Command

Using the verify-tdx tool:

```bash
cd verify-tdx
cargo run --release -- --verify-tdx --verify-binding

# Output will show:
# ✓ TDX REPORTDATA (first 32 bytes): c6a015a4...
# ✓ RISC Zero Receipt Hash:          c6a015a4...
# ✓ BINDING VERIFIED
#   The TDX quote and RISC Zero receipt are cryptographically bound!
```

## Attack Scenarios (Prevented by Binding)

### Attack 1: Receipt Substitution
**Attack**: Attacker takes TDX quote from computation A, tries to use with receipt from computation B

**Prevention**: Receipt hash in TDX quote won't match receipt B's hash

### Attack 2: Quote Reuse
**Attack**: Attacker reuses old TDX quote with new receipt

**Prevention**: Receipt hash won't match, timestamp will be stale

### Attack 3: Fake TEE
**Attack**: Attacker generates receipt outside TEE, creates fake quote

**Prevention**: Cannot forge TDX quote signature without Intel CPU's attestation key

## Comparison with Other Approaches

### Approach 1: No Binding (Insecure)
```
❌ Separate TDX quote and receipt
❌ No cryptographic link
❌ Vulnerable to substitution
```

### Approach 2: Receipt in Quote Signature Data (Complex)
```
⚠️ Requires custom signature scheme
⚠️ May not be supported by verifiers
⚠️ Increases quote size
```

### Approach 3: REPORTDATA Binding (Our Approach) ✓
```
✅ Uses standard TDX REPORTDATA field
✅ Simple and elegant
✅ Verifiable with standard tools
✅ Minimal overhead
```

## Summary

By including the RISC Zero receipt hash in the TDX REPORTDATA:

1. **Cryptographic Proof**: The TDX quote is signed by hardware with the receipt hash inside
2. **Non-Repudiation**: Cannot claim a different receipt was generated
3. **Attestation Link**: Proves the receipt was generated in that specific TDX environment
4. **Unforgeable**: Cannot be created without access to the TDX TEE

This creates a **chain of trust** from:
- Intel Root CA → Platform Cert → TDX Quote → Receipt Hash → RISC Zero Receipt

Anyone can verify this chain and be certain the computation happened in secure hardware.

## References

- [Intel TDX REPORTDATA Specification](https://cdrdv2.intel.com/v1/dl/getContent/733568)
- [RISC Zero Receipt Verification](https://dev.risczero.com/api/zkvm/receipts)
- [Cryptographic Binding Patterns](https://en.wikipedia.org/wiki/Commitment_scheme)

