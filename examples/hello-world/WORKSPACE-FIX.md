# Quick Fix: Workspace Build Error

## Issue
```
error: current package believes it's in a workspace when it's not
```

## Solution

The `Cargo.toml` has been updated to exclude this package from the parent workspace.

### On Your TDX Machine

**Option 1: Pull the Fix** (Recommended)
```bash
cd ~/risc0/examples/hello-world-tee
git pull origin guillevalin/intel-tdx
```

**Option 2: Manual Fix**

Edit `Cargo.toml`:
```bash
nano ~/risc0/examples/hello-world-tee/Cargo.toml
```

Add this line after the `[package]` section:
```toml
[package]
name = "hello-world"
version = "0.1.0"
edition = "2021"

# Exclude from parent workspace to allow standalone builds
[workspace]

[dependencies]
...
```

Save and exit (Ctrl+X, then Y, then Enter).

**Option 3: Quick Patch Command**

```bash
cd ~/risc0/examples/hello-world-tee

# Backup original
cp Cargo.toml Cargo.toml.backup

# Add workspace exclusion
sed -i '/\[package\]/a\\n# Exclude from parent workspace to allow standalone builds\n[workspace]\n' Cargo.toml
```

### Verify the Fix

```bash
cd ~/risc0/examples/hello-world-tee
cargo check
```

Should now compile successfully!

### Run Again

```bash
./run-in-tdx.sh
```

## What This Does

Adding an empty `[workspace]` section tells Cargo that this is its own workspace root, preventing it from being treated as part of the parent workspace at `/home/guillevalin/risc0/examples/`.

This is necessary for standalone projects within a larger repository.

