#!/bin/bash
cargo clippy --target aarch64-apple-darwin --target x86_64-pc-windows-gnu
cargo build --release --target aarch64-apple-darwin --target x86_64-pc-windows-gnu

version=$(grep -e '^version =' Cargo.toml | cut -d '"' -f 2)
echo "Prepare release for version: $version"

cd target/x86_64-pc-windows-gnu/release/
zip sbsrf-update-$version-x86_64-windows.zip sbsrf-update.exe
mv sbsrf-update-$version-x86_64-windows.zip ~/Downloads
cd -

cd target/aarch64-apple-darwin/release/
tar czf sbsrf-update-$version-aarch64-apple-darwin.tar.gz sbsrf-update
mv sbsrf-update-$version-aarch64-apple-darwin.tar.gz ~/Downloads
cd -

echo 'Done!'