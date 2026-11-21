#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"
bin_home=${XDG_BIN_HOME:-"$HOME/.local/bin"}
data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
app_id=io.github.dissering.Tiltframe

cargo build --release --locked
install -Dm755 "${CARGO_TARGET_DIR:-target}/release/tiltframe" "$bin_home/tiltframe"

for icon in data/icons/hicolor/*/apps/"$app_id".*; do
    install -Dm644 "$icon" "$data_home/icons/${icon#data/icons/}"
done

mkdir -p "$data_home/applications"
escaped_exec=$(printf '%s' "$bin_home/tiltframe" | sed 's/[\\"\$\x60]/\\\\\\\\&/g; s/%/%%/g')
while IFS= read -r line; do
    if [[ "$line" == Exec=* ]]; then
        printf 'Exec="%s"\n' "$escaped_exec"
    else
        printf '%s\n' "$line"
    fi
done < "data/$app_id.desktop" > "$data_home/applications/$app_id.desktop"

printf 'Installed %s. Open Tiltframe from your app menu.\n' "$bin_home/tiltframe"
