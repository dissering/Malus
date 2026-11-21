<img src="data/icons/hicolor/128x128/apps/io.github.dissering.Tiltframe.png" width="88" height="88" alt="Tiltframe logo">

# Tiltframe

Tiltframe launches the macOS Roblox Player and Studio on x86-64 Linux through Rosemary. It adds a dedicated FastFlags editor and caps how much live log output the launcher keeps in memory.

## Install on Linux

You’ll need a current stable Rust toolchain, a C toolchain, pkg-config, and development packages for GTK 4.18+, libadwaita 1.7+, and Pango 1.56+. Clang is optional for VR support.

```sh
git clone --branch tiltframe https://github.com/dissering/Tiltframe.git
cd Tiltframe
bash install.sh
```

Open **Tiltframe** from your app menu or run `~/.local/bin/tiltframe`. On first use, the launcher downloads the game client and its runtime. Tiltframe uses its own XDG directories, so you can install it alongside Quadra.

## FastFlags

Open **Settings → FastFlags** while Roblox Player is selected.

- Search by name or value, or select **Overrides only** to review your changes.
- Use **Quick settings** for low or default texture quality.
- Use **Custom overrides** to edit individual flags, import from a JSON file or pasted text, and copy or save your current settings.
- Select **Reset all** to remove every override and return to Roblox defaults.

Roblox controls which local overrides it applies. [FastFlags allowlist announcement](https://devforum.roblox.com/t/allowlist-for-local-client-configuration-via-fast-flags/3966569)

## Graphics and logs

Choose **Metal (via Vulkan)** or **OpenGL** in the launcher’s graphics settings. Enable **MangoHud** there to compare FPS, frame times, and GPU load during the same gameplay route. Tiltframe’s log handling reduces launcher overhead during busy sessions. Gameplay FPS gains are unmeasured.

The live viewer keeps recent output. The full log for the current launch is at `${XDG_DATA_HOME:-~/.local/share}/tiltframe/logs/roblox.log`.

## Development and credits

Run `cargo run --locked` for a development build and `cargo test --locked` for the test suite. See [CONTRIBUTING.md](CONTRIBUTING.md).

Based on [Quadra 0.50.4](https://github.com/QuadraHQ/Quadra/tree/v0.50.4), powered by the [QuadraHQ Rosemary fork](https://github.com/QuadraHQ/rosemary-fork), and inspired by [Fishstrap](https://github.com/fishstrap/fishstrap). Tiltframe is licensed under [GPL-3.0-only](LICENSE).

The Tiltframe icon is a teal retint of the [original Quadra icon](https://github.com/QuadraHQ/Quadra/tree/v0.50.4/data/icons/hicolor).
