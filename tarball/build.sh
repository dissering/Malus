#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

name=Tiltframe-x86_64
staging="$PWD/staging"
root="$staging/$name"

cargo build --release --locked --manifest-path ../Cargo.toml

rm -rf "$staging" "$name.tar.gz"
install -Dm755 ../target/release/tiltframe "$root/bin/tiltframe"
install -Dm644 ../data/io.github.dissering.Tiltframe.desktop \
    "$root/share/applications/io.github.dissering.Tiltframe.desktop"
mkdir -p "$root/share/icons"
cp -r ../data/icons/hicolor "$root/share/icons/"
install -Dm644 ../LICENSE "$root/LICENSE"

tar --sort=name --owner=0 --group=0 --numeric-owner \
    -C "$staging" -czf "$name.tar.gz" "$name"
rm -rf "$staging"
