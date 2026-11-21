#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "$0")/../.." && pwd)
output_dir=${1:-"$repo_dir/dist"}
mkdir -p "$output_dir"
output_dir=$(realpath "$output_dir")

. /etc/os-release
if [[ $ID != ubuntu || $VERSION_ID != 26.04 || $(dpkg --print-architecture) != amd64 ]]; then
    printf 'Build this package on Ubuntu 26.04 amd64.\n' >&2
    exit 1
fi

package_name=malus
app_id=io.github.dissering.Tiltframe
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$repo_dir/Cargo.toml" | head -n 1)
dpkg --validate-version "$version"
package_version="$version-1+ubuntu26.04"
archive="$output_dir/${package_name}_${package_version}_amd64.deb"

build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT
package="$build_dir/debian/$package_name"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$build_dir/target}"
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

cargo build --release --locked --manifest-path "$repo_dir/Cargo.toml"
install -Dm755 "$CARGO_TARGET_DIR/release/$package_name" "$package/usr/bin/$package_name"
strip --strip-unneeded "$package/usr/bin/$package_name"
desktop-file-validate "$repo_dir/data/$app_id.desktop"
install -Dm644 "$repo_dir/data/$app_id.desktop" "$package/usr/share/applications/$app_id.desktop"
install -Dm644 "$repo_dir/data/$app_id.xml" "$package/usr/share/mime/packages/$app_id.xml"
install -Dm644 "$repo_dir/flatpak/$app_id.metainfo.xml" "$package/usr/share/metainfo/$app_id.metainfo.xml"
doc="$package/usr/share/doc/$package_name"
install -Dm644 "$repo_dir/LICENSE" "$doc/copyright"
install -Dm644 "$repo_dir/assets/lucide/LICENSE" "$doc/Lucide-LICENSE"
for notice in "$repo_dir"/assets/fonts/*-OFL.txt; do
    install -Dm644 "$notice" "$doc/${notice##*/}"
done
for icon in "$repo_dir"/data/icons/hicolor/*/apps/"$app_id".*; do
    install -Dm644 "$icon" "$package/usr/share/icons/${icon#"$repo_dir"/data/icons/}"
done

cd "$build_dir"
mkdir -p debian
cat > debian/control <<CONTROL
Source: $package_name
Section: games
Priority: optional
Maintainer: Malus contributors <dissering@users.noreply.github.com>

Package: $package_name
Architecture: amd64
Description: Launch Roblox on Linux
CONTROL

dependencies=$(dpkg-shlibdeps -O -e"$package/usr/bin/$package_name")
dependencies=${dependencies#shlibs:Depends=}
installed_size=$(du --apparent-size --block-size=1024 --summarize "$package" | cut -f1)
mkdir "$package/DEBIAN"
cat > "$package/DEBIAN/control" <<CONTROL
Package: $package_name
Version: $package_version
Architecture: amd64
Section: games
Priority: optional
Maintainer: Malus contributors <dissering@users.noreply.github.com>
Installed-Size: $installed_size
Depends: ca-certificates, shared-mime-info, $dependencies
Recommends: xdg-utils
Homepage: https://github.com/dissering/Malus
Description: Launch Roblox on Linux
 Malus launches Roblox Player and Studio through the Rosemary runtime.
CONTROL

find "$package" -exec touch -h -d "@$SOURCE_DATE_EPOCH" {} +
dpkg-deb --root-owner-group --build "$package" "$archive"
