#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

name=Malus-x86_64
staging="$PWD/staging"
root="$staging/$name"
binary="${MALUS_BINARY:-../target/release/malus}"
rosemary_archive="${MALUS_ROSEMARY_ARCHIVE:-../packaging/rosemary.tar.gz}"

if [[ -n "${MALUS_ROSEMARY_ARCHIVE:-}" || -f "$rosemary_archive" ]]; then
    printf '%s  %s\n' "$(<../packaging/rosemary.sha256)" "$rosemary_archive" | sha256sum --check
else
    rosemary_archive=
fi

if [[ -z "${MALUS_BINARY:-}" ]]; then
    cargo build --release --locked --manifest-path ../Cargo.toml
fi

rm -rf "$staging" "$name.tar.gz"
install -Dm755 "$binary" "$root/bin/malus"
if [[ -n "$rosemary_archive" ]]; then
    install -Dm644 "$rosemary_archive" "$root/share/malus/rosemary.tar.gz"
fi
printf 'Malus portable\n' > "$root/.malus-portable"
install -Dm644 ../data/io.github.dissering.Tiltframe.desktop \
    "$root/share/applications/io.github.dissering.Tiltframe.desktop"
mkdir -p "$root/share/icons"
cp -r ../data/icons/hicolor "$root/share/icons/"
install -Dm644 ../LICENSE "$root/LICENSE"
install -Dm644 -t "$root/licenses" ../assets/fonts/*-OFL.txt

tar --sort=name --owner=0 --group=0 --numeric-owner \
    -C "$staging" -czf "$name.tar.gz" "$name"
rm -rf "$staging"
