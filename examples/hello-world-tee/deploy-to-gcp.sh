#!/bin/bash
# Script to deploy RISC Zero + TDX to Google Cloud Platform

set -euo pipefail

# Configuration
PROJECT_ID="${GCP_PROJECT_ID:-}"
INSTANCE_NAME="${INSTANCE_NAME:-risc0-tdx-vm}"
ZONE="${GCP_ZONE:-us-central1-a}"
MACHINE_TYPE="${MACHINE_TYPE:-n2d-standard-4}"
BOOT_DISK_SIZE="${BOOT_DISK_SIZE:-50GB}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo -e "${GREEN}=== RISC Zero + TDX Deployment to GCP ===${NC}"
echo ""

# Check if gcloud is installed
if ! command -v gcloud &> /dev/null; then
    log_error "gcloud CLI not found. Please install it first:"
    log_error "https://cloud.google.com/sdk/docs/install"
    exit 1
fi

# Check project ID
if [ -z "$PROJECT_ID" ]; then
    log_warn "GCP_PROJECT_ID not set. Attempting to use default project..."
    PROJECT_ID=$(gcloud config get-value project 2>/dev/null || echo "")
    if [ -z "$PROJECT_ID" ]; then
        log_error "No GCP project found. Please set GCP_PROJECT_ID or configure default project"
        exit 1
    fi
fi

log_info "Using GCP Project: ${PROJECT_ID}"
log_info "Instance Name: ${INSTANCE_NAME}"
log_info "Zone: ${ZONE}"
echo ""

# Check if instance already exists
if gcloud compute instances describe "$INSTANCE_NAME" --zone="$ZONE" --project="$PROJECT_ID" &>/dev/null; then
    log_warn "Instance ${INSTANCE_NAME} already exists"
    read -p "Do you want to delete and recreate it? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_info "Deleting existing instance..."
        gcloud compute instances delete "$INSTANCE_NAME" \
            --zone="$ZONE" \
            --project="$PROJECT_ID" \
            --quiet
    else
        log_info "Using existing instance"
        INSTANCE_EXISTS=true
    fi
fi

# Create TDX-enabled VM
if [ "${INSTANCE_EXISTS:-false}" != "true" ]; then
    log_info "Creating TDX-enabled VM..."
    
    gcloud compute instances create "$INSTANCE_NAME" \
        --project="$PROJECT_ID" \
        --zone="$ZONE" \
        --machine-type="$MACHINE_TYPE" \
        --network-interface=network-tier=PREMIUM,subnet=default \
        --maintenance-policy=MIGRATE \
        --provisioning-model=STANDARD \
        --confidential-compute \
        --confidential-compute-type=TDX \
        --image-family=ubuntu-2204-lts \
        --image-project=ubuntu-os-cloud \
        --boot-disk-size="$BOOT_DISK_SIZE" \
        --boot-disk-type=pd-balanced \
        --boot-disk-device-name="$INSTANCE_NAME" \
        --no-shielded-secure-boot \
        --shielded-vtpm \
        --shielded-integrity-monitoring \
        --reservation-affinity=any
    
    log_info "✓ VM created successfully"
    
    # Wait for VM to be ready
    log_info "Waiting for VM to be ready..."
    sleep 30
fi

# Get instance IP
INSTANCE_IP=$(gcloud compute instances describe "$INSTANCE_NAME" \
    --zone="$ZONE" \
    --project="$PROJECT_ID" \
    --format='get(networkInterfaces[0].accessConfigs[0].natIP)')

log_info "Instance IP: ${INSTANCE_IP}"

# Create temporary directory for deployment
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Copy files to temp directory
log_info "Preparing deployment files..."
cp -r . "$TEMP_DIR/hello-world"

# Create deployment script
cat > "$TEMP_DIR/setup.sh" << 'EOF'
#!/bin/bash
set -euo pipefail

echo "Setting up RISC Zero + TDX environment..."

# Update system
sudo apt-get update
sudo apt-get upgrade -y

# Install dependencies
sudo apt-get install -y \
    build-essential \
    curl \
    wget \
    git \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip

# Install Rust
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Verify TDX
echo "Verifying TDX environment..."
if [ -e /dev/tdx_guest ]; then
    echo "✓ TDX device found"
else
    echo "✗ TDX device not found - may need kernel module"
    sudo modprobe tdx_guest || echo "Could not load tdx_guest module"
fi

# Navigate to hello-world directory
cd ~/hello-world

# Make scripts executable
chmod +x run-in-tdx.sh verify-attestation.sh

echo ""
echo "=== Setup Complete ==="
echo ""
echo "To run the example:"
echo "  cd ~/hello-world"
echo "  ./run-in-tdx.sh"
echo ""
echo "To verify attestations:"
echo "  ./verify-attestation.sh"
echo ""
EOF

chmod +x "$TEMP_DIR/setup.sh"

# Upload files to VM
log_info "Uploading files to VM..."
gcloud compute scp \
    --recurse \
    "$TEMP_DIR/"* \
    "${INSTANCE_NAME}:~/" \
    --zone="$ZONE" \
    --project="$PROJECT_ID"

# Run setup script
log_info "Running setup script on VM..."
gcloud compute ssh "$INSTANCE_NAME" \
    --zone="$ZONE" \
    --project="$PROJECT_ID" \
    --command="bash ~/setup.sh"

# Display instructions
echo ""
log_info "=== Deployment Complete ==="
echo ""
echo "To connect to the VM:"
echo "  gcloud compute ssh $INSTANCE_NAME --zone=$ZONE --project=$PROJECT_ID"
echo ""
echo "To run the example on the VM:"
echo "  cd ~/hello-world"
echo "  ./run-in-tdx.sh"
echo ""
echo "To copy results back:"
echo "  gcloud compute scp --recurse \\"
echo "    ${INSTANCE_NAME}:~/hello-world/tdx-output \\"
echo "    ./ \\"
echo "    --zone=$ZONE \\"
echo "    --project=$PROJECT_ID"
echo ""
echo "To delete the VM when done:"
echo "  gcloud compute instances delete $INSTANCE_NAME --zone=$ZONE --project=$PROJECT_ID"
echo ""

