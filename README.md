# Malus

<img src="data/icons/hicolor/128x128/apps/io.github.dissering.Tiltframe.png" width="88" height="88" alt="Malus logo">

Roblox Player and Studio on Linux through Rosemary. Forked from [Quadra](https://github.com/QuadraHQ/Quadra).

## Install

Download the package for your x86-64 Linux distribution. Release packages include Rosemary and download Roblox on first launch. [GitHub Releases](https://github.com/dissering/Malus/releases/latest)

Build on x86-64 Linux with stable Rust, a C compiler, pkg-config, and development headers for GTK 4.18+ and libadwaita 1.7+. Clang is optional for VR.

```sh
git clone https://github.com/dissering/Malus.git
cd Malus
./install.sh
```

Open Malus from the app menu. Source installs download the same verified Rosemary build on first launch. [Linux package build instructions](packaging/README.md)

## Commands

```sh
malus launch
malus launch --profile "Low Latency"
malus join PLACE_ID
malus rejoin
malus studio "my place.rbxl"
malus settings
malus logs
malus help
```

## Settings

- **Profiles** saves FastFlags, graphics settings, and mod settings. Applying a profile makes it your current settings.
- **Library** searches favorites and recent experiences by name or place ID. Use the row menu to copy a Roblox link or remove a recent entry.
- **Studio** opens local `.rbxl` and `.rbxlx` files. **Studio Plugins** imports and exports local `.rbxm` and `.rbxmx` plugins in the Malus-managed prefix.
- **Mods** imports ZIPs containing sounds, textures, and fonts. Turn packs on or off individually. Changes apply next launch.
- **General → Discord** controls Rich Presence, which is enabled for new setups. Edit the activity text, hide the timer or link, or turn off BloxstrapRPC overrides. Use `{game}` in either text field for the experience name.
- **General → Updates** checks GitHub. Portable builds and AppImages can update automatically when Roblox and Studio are closed. Update installed packages through your package manager.

The bundled Rosemary targets 15 FPS while Roblox Player is minimized and resumes normal rendering when restored. On Wayland, the compositor must report the window as suspended or occluded. Set `ROSEMARY_MINIMIZED_FPS` in Advanced to change the limit or use `0` to disable it.

Open the game log with `malus logs`.

[GPL-3.0-only](LICENSE)
