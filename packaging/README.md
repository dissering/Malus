# Linux packages

These builds target x86-64 and bundle the Malus Rosemary runtime. The versioned runtime archive and its source are available on [GitHub Releases](https://github.com/dissering/Malus/releases).

## Arch

```sh
cd packaging/arch
makepkg -s
```

## Fedora 44

Run from the repository root.

```sh
sudo dnf install rpm-build dnf5-plugins
sudo dnf builddep packaging/fedora/malus.spec
version=$(awk '/^Version:/ { print $2 }' packaging/fedora/malus.spec)
rosemary=$(awk '/^%global rosemary_archive / { print $3 }' packaging/fedora/malus.spec)
sources="$HOME/rpmbuild/SOURCES"
mkdir -p "$sources"
archive="$sources/malus-$version.tar.gz"
curl -fL "https://codeload.github.com/dissering/Malus/tar.gz/refs/tags/v$version" -o "$archive"
curl -fL "https://github.com/dissering/Malus/releases/download/v$version/$rosemary" -o "$sources/$rosemary"
scratch=$(mktemp -d)
tar -xf "$archive" -C "$scratch"
(cd "$scratch/Malus-$version" && cargo fetch --locked --target x86_64-unknown-linux-gnu)
rpmbuild -bb packaging/fedora/malus.spec
```

The RPM is under `~/rpmbuild/RPMS/x86_64/`.

## Ubuntu 26.04

[Build the Ubuntu package](ubuntu/README.md) with Podman.

## Flatpak

The manifest uses GNOME Platform 51. From the repository root with Flatpak Builder and Flathub configured:

```sh
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)
runtime=Malus-Rosemary-0.5.0-quadra.3-malus.1-x86_64.tar.gz
curl -fL "https://github.com/dissering/Malus/releases/download/v$version/$runtime" -o packaging/rosemary.tar.gz
manifest=$(find flatpak -maxdepth 1 -name '*.yml' -print -quit)
app_id=$(awk '/^id:/ { print $2 }' "$manifest")
flatpak-builder --user --install-deps-from=flathub --repo=repo build-dir "$manifest"
flatpak build-bundle repo Malus-x86_64.flatpak "$app_id"
```
