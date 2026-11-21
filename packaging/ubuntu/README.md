# Ubuntu package

Build the x86-64 package for Ubuntu 26.04 LTS from the repository root.

```sh
mkdir -p dist
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)
runtime=Malus-Rosemary-0.5.0-quadra.3-malus.1-x86_64.tar.gz
curl -fL "https://github.com/dissering/Malus/releases/download/v$version/$runtime" -o "dist/$runtime"
podman build -t malus-ubuntu:26.04 packaging/ubuntu
podman run --rm --security-opt label=disable \
    -e MALUS_ROSEMARY_ARCHIVE="/out/$runtime" \
    -v "$PWD:/src:ro" -v "$PWD/dist:/out" malus-ubuntu:26.04
```

Install the resulting local package on Ubuntu 26.04 with `sudo apt install ./dist/malus_*_amd64.deb`. The package uses Ubuntu's GTK libraries and bundles Rosemary. Malus downloads Roblox when needed.
