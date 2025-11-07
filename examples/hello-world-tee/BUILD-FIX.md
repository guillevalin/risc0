# Build Fix: Generic Array Deprecation Warnings

## Issue

The RISC Zero codebase on this branch uses deprecated `generic-array` 0.x APIs, causing build failures:

```
error: use of deprecated struct `digest::generic_array::GenericArray`: 
please upgrade to generic-array 1.x
```

## Solution

Allow deprecated warnings during compilation. Two fixes applied:

### 1. Cargo Configuration (Automatic)
**File:** `.cargo/config.toml`

```toml
[build]
rustflags = ["-A", "deprecated"]
```

This automatically allows deprecated warnings for all builds in this directory.

### 2. Build Script (Backup)
**File:** `run-in-tdx.sh`

```bash
export RUSTFLAGS="-A deprecated"
```

Sets the environment variable before building.

## Why This Works

- `-A deprecated` tells the Rust compiler to allow (not error on) deprecation warnings
- The actual deprecation is in `risc0-zkp`, not your code
- This is a temporary fix until the branch is updated to use `generic-array` 1.x
- Your code remains unaffected - only compilation warnings are suppressed

## Alternative: Manual Build

If you need to build manually without the script:

```bash
cd ~/risc0/examples/hello-world-tee

# Set environment variable
export RUSTFLAGS="-A deprecated"

# Build
cargo build --release --features prove
```

## On Your TDX Machine

Pull and run:

```bash
cd ~/risc0/examples/hello-world-tee
git pull origin guillevalin/intel-tdx
./run-in-tdx.sh
```

The build should now succeed! ✅

## Technical Details

The deprecation warnings come from:
- `risc0/zkp/src/core/hash/sha/cpu.rs`
- `risc0/zkp/src/core/hash/sha/rust_crypto.rs`

These files use `digest::generic_array::GenericArray` which is deprecated in favor of the newer `generic-array` 1.x crate.

This doesn't affect functionality - the code works perfectly, it just uses older APIs.

