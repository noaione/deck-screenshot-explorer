#!/bin/sh
set -e

echo "Container's IP address: `awk 'END{print $1}' /etc/hosts`"

cd /backend

echo "Updating rust stable..."
rustup update stable

echo "Building Vue.js frontend view..."
cd frontend

npm ci
npm run build-only
cd ..

echo "Compiling Rust backend..."
cargo build --release --all

chmod +x ./target/release/backend

echo "Copying backend files to output"
mkdir -p out
cp ./target/release/backend ./out/backend
