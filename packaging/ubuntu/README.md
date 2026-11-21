# Ubuntu package

Build the x86-64 package for Ubuntu 26.04 LTS from the repository root.

```sh
mkdir -p dist
podman build -t malus-ubuntu:26.04 packaging/ubuntu
podman run --rm --security-opt label=disable \
    -v "$PWD:/src:ro" -v "$PWD/dist:/out" malus-ubuntu:26.04
```

Install the resulting local package on Ubuntu 26.04 with `sudo apt install ./dist/malus_*_amd64.deb`. The package uses Ubuntu's GTK libraries. Malus downloads Rosemary and Roblox on the first launch.
