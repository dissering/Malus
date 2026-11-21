# Malus

<img src="data/icons/hicolor/128x128/apps/io.github.dissering.Tiltframe.png" width="88" height="88" alt="Malus logo">

Roblox Player and Studio on Linux through Rosemary. Forked from [Quadra](https://github.com/QuadraHQ/Quadra).

## Install

Build on x86-64 Linux with stable Rust, a C compiler, pkg-config, `glib-compile-resources`, GTK 4.18+, libadwaita 1.7+, and Pango 1.56+ development headers. Clang is optional for VR.

```sh
git clone https://github.com/dissering/Malus.git
cd Malus
./install.sh
```

Open Malus from the app menu. The first launch downloads Roblox and Rosemary.

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
- **Library** lists favorites and recent servers. Player counts are for the whole experience.
- **Mods** imports ZIPs containing sounds, textures, and fonts. Turn packs on or off individually. Changes apply next launch.
- **General → Input & windows** sets the initial Player window size and title bar style.
- **General → Discord** lets you edit activity text, hide the timer or game link, and choose whether experiences can override their activity through BloxstrapRPC. Use `{game}` in either text field for the experience name.

Open the game log with `malus logs`.

[GPL-3.0-only](LICENSE)
