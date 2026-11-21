# Tiltframe

Tiltframe is a Linux launcher for the macOS Roblox Player and Studio. It is based on [Quadra 0.50.4](https://github.com/QuadraHQ/Quadra/tree/v0.50.4), uses [Rosemary](https://github.com/QuadraHQ/rosemary-fork) for compatibility, and takes inspiration from [Fishstrap](https://github.com/fishstrap/fishstrap) for FastFlag management.

## Install from source

Use a Linux x86-64 system with a recent stable Rust toolchain, a C toolchain, pkg-config, GTK 4.18+, libadwaita 1.7+, and Pango 1.56+ development files. Clang is optional for VR support.

From this checkout, run:

```sh
bash install.sh
```

Open **Tiltframe** from your app menu. For a development build, use `cargo run --locked`.

Tiltframe uses separate `tiltframe` directories under your XDG config, data, and cache directories. It downloads the Roblox client and Rosemary on first use. You can transfer FastFlags from another launcher by exporting them as JSON and importing them from **Settings → FastFlags** in Tiltframe.

## FastFlags

Search by flag name or value. Turn on **Overrides only** to review what you changed. Use **Quick settings** to set low texture quality or return textures to their defaults. Under **Custom overrides**, you can edit individual entries or import and export JSON. **Reset all** restores Roblox defaults by removing all overrides.

Roblox decides which local overrides it applies. The launcher writes Player settings as `ClientAppSettings.json` and checks the macOS client's published allowlist.

## License

Tiltframe is a GPL-3.0-only fork of Quadra. The original project and contributors are credited in the source history. See [LICENSE](LICENSE).
