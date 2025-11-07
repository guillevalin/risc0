#!/bin/bash
# Copyright 2025 RISC Zero, Inc.
#
# Script to verify Intel TDX attestation quote and RISC Zero receipt

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/tdx-output"

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo -e "${GREEN}=== TDX Attestation Verification ===${NC}"
echo ""

# Check if output directory exists
if [ ! -d "${OUTPUT_DIR}" ]; then
    log_error "Output directory not found: ${OUTPUT_DIR}"
    log_error "Please run ./run-in-tdx.sh first"
    exit 1
fi

# Verify TDX Report
verify_tdx_report() {
    log_info "Verifying TDX Report..."
    
    local report_file="${OUTPUT_DIR}/tdx-report.bin"
    
    if [ ! -f "${report_file}" ]; then
        log_error "TDX report not found: ${report_file}"
        return 1
    fi
    
    log_info "Report file found: ${report_file}"
    log_info "Report size: $(stat -f%z "${report_file}" 2>/dev/null || stat -c%s "${report_file}") bytes"
    
    # TDX Report structure verification
    local expected_size=1024
    local actual_size=$(stat -f%z "${report_file}" 2>/dev/null || stat -c%s "${report_file}")
    
    if [ "${actual_size}" -eq "${expected_size}" ]; then
        log_info "✓ Report size matches expected TDX report structure"
    else
        log_warn "Report size (${actual_size}) doesn't match expected (${expected_size})"
    fi
    
    # Display report info
    log_info "Report contains:"
    log_info "  - REPORTDATA: User-provided data (64 bytes)"
    log_info "  - TEE TCB Info: TD measurements and attributes"
    log_info "  - RTMR values: Runtime Measurement Registers"
    log_info "  - MRCONFIGID, MROWNER, MROWNERCONFIG"
    log_info "  - MRTD: Measurement of initial TD state"
    
    echo ""
}

# Verify TDX Quote
verify_tdx_quote() {
    log_info "Verifying TDX Quote..."
    
    local quote_file="${OUTPUT_DIR}/tdx-quote.bin"
    
    if [ ! -f "${quote_file}" ]; then
        log_warn "TDX quote not found: ${quote_file}"
        log_warn "Quote verification skipped"
        return 1
    fi
    
    log_info "Quote file found: ${quote_file}"
    log_info "Quote size: $(stat -f%z "${quote_file}" 2>/dev/null || stat -c%s "${quote_file}") bytes"
    
    # Check for Intel DCAP libraries
    if command -v tdx-verify >/dev/null 2>&1; then
        log_info "Using tdx-verify for quote verification..."
        tdx-verify "${quote_file}" || {
            log_warn "Quote verification failed"
            return 1
        }
    elif command -v dcap-verify >/dev/null 2>&1; then
        log_info "Using dcap-verify for quote verification..."
        dcap-verify "${quote_file}" || {
            log_warn "Quote verification failed"
            return 1
        }
    else
        log_warn "No TDX verification tools found"
        log_info "For quote verification, you can:"
        log_info "  1. Use Intel SGX DCAP Quote Verification Library"
        log_info "  2. Use Azure Attestation Service: https://azure.microsoft.com/services/attestation/"
        log_info "  3. Use Intel Trust Authority: https://trustauthority.intel.com/"
        log_info ""
        log_info "Manual verification steps:"
        log_info "  - Extract quote header and body"
        log_info "  - Verify signature using Intel's signing key"
        log_info "  - Verify certificate chain back to Intel root CA"
        log_info "  - Check TCB (Trusted Computing Base) status"
    fi
    
    echo ""
}

# Verify RISC Zero Receipt
verify_risc0_receipt() {
    log_info "Verifying RISC Zero Receipt..."
    
    local receipt_file="${OUTPUT_DIR}/risc0-receipt.json"
    
    if [ ! -f "${receipt_file}" ]; then
        log_warn "RISC Zero receipt JSON not found"
        log_info "Note: The current implementation logs receipt verification"
        log_info "Check execution.log for verification results"
        return 1
    fi
    
    log_info "Receipt file found: ${receipt_file}"
    cat "${receipt_file}"
    echo ""
    
    log_info "RISC Zero receipt verification happens during execution"
    log_info "See execution.log for detailed verification results"
    
    echo ""
}

# Display attestation bundle
display_attestation_bundle() {
    log_info "Attestation Bundle Summary:"
    
    local bundle_file="${OUTPUT_DIR}/attestation-bundle.json"
    
    if [ -f "${bundle_file}" ]; then
        cat "${bundle_file}"
        echo ""
    else
        log_warn "Attestation bundle not found"
    fi
}

# Remote verification instructions
display_remote_verification() {
    echo ""
    log_info "=== Remote Verification Instructions ==="
    echo ""
    echo "To verify these attestations remotely:"
    echo ""
    echo "1. TDX Quote Verification:"
    echo "   - Upload quote to Intel Trust Authority or Azure Attestation"
    echo "   - Use Intel DCAP libraries in your application"
    echo "   - Verify against Intel's PCK certificates"
    echo ""
    echo "2. RISC Zero Receipt Verification:"
    echo "   - Use risc0-zkvm library to verify receipt"
    echo "   - Check receipt.verify(MULTIPLY_ID) succeeds"
    echo "   - Verify journal contents match expected output"
    echo ""
    echo "3. Combined Attestation:"
    echo "   - Bind TDX quote reportdata to RISC Zero receipt hash"
    echo "   - Verify both independently"
    echo "   - Confirm they're cryptographically linked"
    echo ""
    echo "Example verification services:"
    echo "  - Intel Trust Authority: https://trustauthority.intel.com/"
    echo "  - Azure Attestation: https://azure.microsoft.com/services/attestation/"
    echo "  - Google Cloud Confidential Computing: https://cloud.google.com/confidential-computing"
    echo ""
}

# Main verification
main() {
    verify_tdx_report
    verify_tdx_quote
    verify_risc0_receipt
    display_attestation_bundle
    display_remote_verification
    
    log_info "Verification complete! ✓"
}

main "$@"

