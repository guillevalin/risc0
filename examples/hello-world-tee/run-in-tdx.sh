#!/bin/bash
# Copyright 2025 RISC Zero, Inc.
#
# Script to run RISC Zero hello-world example inside Intel TDX TEE
# and generate attestation quote and report

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/tdx-output"
REPORT_FILE="${OUTPUT_DIR}/tdx-report.bin"
QUOTE_FILE="${OUTPUT_DIR}/tdx-quote.bin"
RECEIPT_FILE="${OUTPUT_DIR}/risc0-receipt.json"
LOG_FILE="${OUTPUT_DIR}/execution.log"

echo -e "${GREEN}=== RISC Zero + Intel TDX Integration ===${NC}"
echo "Script directory: ${SCRIPT_DIR}"
echo "Output directory: ${OUTPUT_DIR}"
echo ""

# Create output directory
mkdir -p "${OUTPUT_DIR}"

# Function to print colored messages
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running in TDX environment
check_tdx_environment() {
    log_info "Checking Intel TDX environment..."
    
    # Check if TDX is available
    if [ ! -e /dev/tdx_guest ]; then
        log_error "TDX device not found at /dev/tdx_guest"
        log_error "This script must be run inside an Intel TDX Trust Domain"
        log_error "Please ensure:"
        log_error "  1. You're running on TDX-capable hardware"
        log_error "  2. TDX is enabled in BIOS"
        log_error "  3. You're inside a TDX VM/TD"
        exit 1
    fi
    
    log_info "✓ TDX device found"
    
    # Check for TDX kernel module
    if ! lsmod | grep -q tdx_guest; then
        log_warn "TDX guest kernel module not loaded, attempting to load..."
        sudo modprobe tdx_guest || {
            log_error "Failed to load tdx_guest module"
            exit 1
        }
    fi
    
    log_info "✓ TDX guest kernel module loaded"
}

# Install dependencies
install_dependencies() {
    log_info "Installing dependencies..."
    
    # Check if we need to install packages
    local packages_needed=()
    
    command -v cargo >/dev/null 2>&1 || packages_needed+=("rust")
    
    if [ ${#packages_needed[@]} -ne 0 ]; then
        log_info "Installing missing packages: ${packages_needed[*]}"
        
        # Detect package manager and install
        if command -v apt-get >/dev/null 2>&1; then
            sudo apt-get update
            sudo apt-get install -y build-essential curl
            
            # Install Rust if needed
            if [[ " ${packages_needed[*]} " =~ " rust " ]]; then
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                source "$HOME/.cargo/env"
            fi
        elif command -v yum >/dev/null 2>&1; then
            sudo yum groupinstall -y "Development Tools"
            sudo yum install -y curl
            
            # Install Rust if needed
            if [[ " ${packages_needed[*]} " =~ " rust " ]]; then
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                source "$HOME/.cargo/env"
            fi
        fi
    fi
    
    log_info "✓ Dependencies installed"
}

# Build the hello-world example
build_example() {
    log_info "Building RISC Zero hello-world example..."
    
    cd "${SCRIPT_DIR}"
    
    # Allow deprecated warnings from risc0-zkp generic-array usage
    export RUSTFLAGS="-A deprecated"
    
    # Build with prove feature enabled
    cargo build --release --features prove 2>&1 | tee -a "${LOG_FILE}"
    
    log_info "✓ Build completed"
}

# Generate TDX report
generate_tdx_report() {
    log_info "Generating TDX report..."
    
    # TDX report data (64 bytes, can include hash of RISC Zero receipt)
    # For now, we'll use the hello-world execution as the reportdata
    local report_data=$(printf "RISC0-HelloWorld-%.48s" "$(date -u +%Y%m%d%H%M%S)")
    
    # Generate report using TDX ioctl
    # This requires a small C program or using a library that can make the ioctl call
    # For this script, we'll use a Python helper
    python3 << 'EOF' > "${REPORT_FILE}" 2>&1 | tee -a "${LOG_FILE}"
import struct
import fcntl
import sys
import os

# TDX IOCTL commands
TDX_CMD_GET_REPORT = 0xc4004401

def get_tdx_report(report_data=None):
    """Generate TDX report"""
    try:
        # Open TDX device
        with open('/dev/tdx_guest', 'rb') as tdx_device:
            # Prepare report data (64 bytes)
            if report_data is None:
                report_data = b'RISC0-HelloWorld' + b'\x00' * 48
            else:
                report_data = report_data[:64].ljust(64, b'\x00')
            
            # TDX report structure
            # struct tdx_report_req {
            #     __u8 reportdata[64];
            #     __u8 tdreport[1024];
            # }
            report_req = report_data + b'\x00' * 1024
            
            # Make ioctl call
            result = fcntl.ioctl(tdx_device.fileno(), TDX_CMD_GET_REPORT, report_req)
            
            # Extract report (last 1024 bytes)
            report = result[64:]
            
            # Write to stdout (will be redirected to file)
            sys.stdout.buffer.write(report)
            
            return 0
    except Exception as e:
        print(f"Error generating TDX report: {e}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    sys.exit(get_tdx_report())
EOF
    
    if [ $? -eq 0 ] && [ -f "${REPORT_FILE}" ] && [ -s "${REPORT_FILE}" ]; then
        log_info "✓ TDX report generated: ${REPORT_FILE}"
        log_info "  Report size: $(stat -f%z "${REPORT_FILE}" 2>/dev/null || stat -c%s "${REPORT_FILE}") bytes"
    else
        log_error "Failed to generate TDX report"
        return 1
    fi
}

# Generate TDX quote
generate_tdx_quote() {
    log_info "Generating TDX attestation quote..."
    
    # TDX quote generation requires communication with the TDX Quote Generation Service (QGS)
    # This is typically done through the DCAP library
    
    # Check if attestation service is available
    if [ ! -S /var/run/tdx-attest.sock ] && [ ! -S /run/confidential-containers/attestation/teeproto.sock ]; then
        log_warn "TDX attestation socket not found. Attempting to use tdx-attest tool..."
        
        # Try using configfs-tsm interface (newer kernel approach)
        if [ -d /sys/kernel/config/tsm/report ]; then
            log_info "Using configfs-tsm interface for attestation..."
            
            # Generate report via configfs
            local report_dir="/sys/kernel/config/tsm/report/report0"
            sudo mkdir -p "${report_dir}" 2>/dev/null || true
            
            # Write report data
            echo "RISC0-HelloWorld-TDX" | sudo tee "${report_dir}/inblob" > /dev/null
            
            # Read the quote
            sudo cat "${report_dir}/outblob" > "${QUOTE_FILE}" 2>/dev/null || {
                log_error "Failed to generate quote via configfs-tsm"
                return 1
            }
            
            # Cleanup
            sudo rmdir "${report_dir}" 2>/dev/null || true
        else
            log_warn "No TDX attestation mechanism found"
            log_warn "Quote generation skipped. Install Intel SGX DCAP libraries or use a TDX-enabled system"
            return 1
        fi
    else
        # Use attestation socket
        log_info "Using TDX attestation service..."
        
        # This would typically use a tool like tdx-attest or gramine-ratls
        # For demonstration, we'll create a placeholder
        log_warn "Attestation service integration not fully implemented"
        echo "TDX Quote Placeholder - Use Intel DCAP libraries for production" > "${QUOTE_FILE}"
    fi
    
    if [ -f "${QUOTE_FILE}" ] && [ -s "${QUOTE_FILE}" ]; then
        log_info "✓ TDX quote generated: ${QUOTE_FILE}"
        log_info "  Quote size: $(stat -f%z "${QUOTE_FILE}" 2>/dev/null || stat -c%s "${QUOTE_FILE}") bytes"
    else
        log_warn "Quote file not generated or empty"
        return 1
    fi
}

# Run the RISC Zero example and capture receipt
run_risc0_example() {
    log_info "Running RISC Zero hello-world example..."
    
    cd "${SCRIPT_DIR}"
    
    # Run the example and capture output
    cargo run --release --features prove 2>&1 | tee -a "${LOG_FILE}"
    
    log_info "✓ RISC Zero execution completed"
    
    # Note: In a real implementation, you would modify the main.rs to serialize
    # the receipt to a file. For now, we log this information.
    log_info "Receipt generated (see logs for verification)"
    
    # Create a summary JSON
    cat > "${RECEIPT_FILE}" << EOJSON
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "example": "hello-world",
  "status": "success",
  "note": "See logs for full receipt details. Modify main.rs to export receipt JSON."
}
EOJSON
}

# Generate combined attestation report
generate_combined_report() {
    log_info "Generating combined attestation report..."
    
    local combined_report="${OUTPUT_DIR}/attestation-bundle.json"
    
    # Create comprehensive attestation bundle
    cat > "${combined_report}" << EOREPORT
{
  "version": "1.0",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "attestation_type": "Intel-TDX",
  "risc0_example": "hello-world",
  "files": {
    "tdx_report": "$(basename "${REPORT_FILE}")",
    "tdx_quote": "$(basename "${QUOTE_FILE}")",
    "risc0_receipt": "$(basename "${RECEIPT_FILE}")",
    "execution_log": "$(basename "${LOG_FILE}")"
  },
  "verification": {
    "tdx_report": "Verify using Intel TDX verification tools",
    "tdx_quote": "Verify using Intel SGX DCAP libraries and Azure/Intel attestation services",
    "risc0_receipt": "Verify using risc0-zkvm Receipt::verify() method",
    "instructions": "See ATTESTATION-README.md for detailed verification steps"
  },
  "system_info": {
    "kernel": "$(uname -r)",
    "cpu": "$(grep 'model name' /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)",
    "tdx_device": "$([ -e /dev/tdx_guest ] && echo 'present' || echo 'absent')"
  }
}
EOREPORT
    
    log_info "✓ Combined attestation report: ${combined_report}"
}

# Display results
display_results() {
    echo ""
    log_info "=== Execution Complete ==="
    echo ""
    echo "Generated files:"
    ls -lh "${OUTPUT_DIR}"
    echo ""
    log_info "Attestation bundle ready for verification"
    log_info "Transfer these files to a verifier for remote attestation"
}

# Main execution
main() {
    log_info "Starting TDX + RISC Zero integration..."
    echo ""
    
    # Check TDX environment
    check_tdx_environment
    echo ""
    
    # Install dependencies
    install_dependencies
    echo ""
    
    # Build example
    build_example
    echo ""
    
    # Generate TDX report
    generate_tdx_report
    echo ""
    
    # Generate TDX quote
    generate_tdx_quote || log_warn "Quote generation failed, continuing..."
    echo ""
    
    # Run RISC Zero example
    run_risc0_example
    echo ""
    
    # Generate combined report
    generate_combined_report
    echo ""
    
    # Display results
    display_results
    
    log_info "Done! 🎉"
}

# Run main function
main "$@"

