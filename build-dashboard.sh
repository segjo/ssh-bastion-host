#!/bin/bash

# SSH Bastion Dashboard Build Script

set -e

echo "🔨 Building SSH Bastion Web Dashboard..."

cd "$(dirname "$0")/web-dashboard" || exit 1

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Install from https://rustup.rs/"
    exit 1
fi

echo "📦 Building with Cargo..."
cargo build --release

BINARY="./target/release/ssh-bastion-dashboard"

if [ -f "$BINARY" ]; then
    SIZE=$(du -h "$BINARY" | cut -f1)
    echo "✅ Build successful!"
    echo "   Binary: $BINARY ($SIZE)"
    echo ""
    echo "To run the dashboard:"
    echo "  cd web-dashboard"
    echo "  ./target/release/ssh-bastion-dashboard"
    echo ""
    echo "Or use Docker Compose:"
    echo "  docker-compose up -d"
else
    echo "❌ Build failed"
    exit 1
fi
