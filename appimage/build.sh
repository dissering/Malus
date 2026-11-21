#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

tools="$PWD/tools"
appdir="$PWD/AppDir"
gtk_hook="$appdir/apprun-hooks/linuxdeploy-plugin-gtk.sh"
binary="${MALUS_BINARY:-../target/release/malus}"
rosemary_archive="${MALUS_ROSEMARY_ARCHIVE:-../packaging/rosemary.tar.gz}"

if [[ -n "${MALUS_ROSEMARY_ARCHIVE:-}" || -f "$rosemary_archive" ]]; then
    printf '%s  %s\n' "$(<../packaging/rosemary.sha256)" "$rosemary_archive" | sha256sum --check
else
    rosemary_archive=
fi

download_tool() {
    local name="$1" url="$2" checksum="$3"
    local target="$tools/$name"
    if [[ ! -f "$target" ]] || ! printf '%s  %s\n' "$checksum" "$target" | sha256sum --check --status; then
        curl -fsSL -o "$target.download" "$url"
        printf '%s  %s\n' "$checksum" "$target.download" | sha256sum --check --status
        mv "$target.download" "$target"
    fi
    chmod +x "$target"
}

mkdir -p "$tools"
download_tool linuxdeploy-x86_64.AppImage \
    https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage \
    c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d
download_tool linuxdeploy-plugin-gtk.sh \
    https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/7a3fbc31a9e5075073ff8790f26effbac5f84453/linuxdeploy-plugin-gtk.sh \
    b0f4cbc684a0103a9651f0955b635eaea0096b3a66c0f5a2c2aa337960375171

if [[ -z "${MALUS_BINARY:-}" ]]; then
    cargo build --release --locked --manifest-path ../Cargo.toml
fi

export PATH="$tools:$PATH"
export DEPLOY_GTK_VERSION=4

icon_files=()
for icon in ../data/icons/hicolor/*/apps/io.github.dissering.Tiltframe.png; do
    icon_files+=(--icon-file "$icon")
done

rm -rf "$appdir"
linuxdeploy-x86_64.AppImage \
    --appdir "$appdir" \
    --executable "$binary" \
    --desktop-file ../data/io.github.dissering.Tiltframe.desktop \
    "${icon_files[@]}" \
    --plugin gtk

sed -i '/^export GTK_THEME=/d; /^export GDK_BACKEND=/d' "$gtk_hook"

if [[ -n "$rosemary_archive" ]]; then
    install -Dm644 "$rosemary_archive" "$appdir/usr/share/malus/rosemary.tar.gz"
fi
install -Dm644 ../LICENSE "$appdir/usr/share/licenses/malus/LICENSE"
install -Dm644 -t "$appdir/usr/share/licenses/malus" ../assets/fonts/*-OFL.txt

(
    cd "$tools"
    ./linuxdeploy-x86_64.AppImage --appimage-extract >/dev/null
)
tool_root="$tools/squashfs-root"
while IFS= read -r -d '' module; do
    library_path="$(realpath --relative-to="$(dirname "$module")" "$appdir/usr/lib")"
    "$tool_root/usr/bin/patchelf" --set-rpath "\$ORIGIN:\$ORIGIN/$library_path" "$module"
done < <(find "$appdir/usr/lib" -mindepth 2 -type f -name '*.so' -print0)

ARCH=x86_64 "$tool_root/plugins/linuxdeploy-plugin-appimage/usr/bin/appimagetool" \
    "$appdir" "$PWD/Malus-x86_64.AppImage"
