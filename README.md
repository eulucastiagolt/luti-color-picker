# Luti Color Picker

Linux color picker with a magnifier, scrollable visual history, and 11 copy formats, including HEX, RGB, HSL, and LAB.

## Installation

Extract the Linux archive and run `./install.sh` from the extracted directory. The installer puts the application and desktop entry under `~/.local`. Add `~/.local/bin` to your `PATH` or run `~/.local/bin/luti-color-picker` directly.

The application requires GTK 4 and system libraries compatible with the build. On Wayland, the magnifier uses Spectacle on KDE Plasma or grim on compatible compositors. The XDG Desktop Portal fallback depends on the compositor and returns the selected color without a context thumbnail. Feedback sounds use the session's default PulseAudio or PipeWire output, with `pw-play` and `aplay` as fallbacks.

## Usage

Click **Capture color**, then click the pixel you want. Scroll to zoom or press Escape or right-click to cancel. Set the magnifier's minimum and maximum zoom under **Settings**.

Click a history card to copy its color in the current format. **Palette** adds the color to the palette selected under **Palettes**. To choose a destination each time, enable **Ask which palette to add history colors to** under **Settings**.

- **Palettes** stores named, editable palettes. Add colors from recent captures and export to PNG, SVG, TXT, CSS variables, Tailwind CSS v3 and v4, JSON, or GPL for GIMP and Inkscape.
- **Gradients** lets you edit colors, stop positions, and angles, copy CSS, and export PNGs.
- **Image colors** extracts dominant colors from an image and saves them as a palette.

Open **Settings** using the gear button to control automatic copying, sound feedback, tray behavior, and startup at login. Startup at login is available for native installations, AppImage, and the `.run` package.

```sh
luti-color-picker --pick
luti-color-picker --about
luti-color-picker --background
```

The application saves history, palettes, the current gradient, and preferences in `${XDG_DATA_HOME:-$HOME/.local/share}/luti-color-picker/state.json`. If that file is absent, it looks for legacy history in `${XDG_CACHE_HOME:-$HOME/.cache}/colour-picker.json`.

### Global capture shortcut

Enable **Register a global shortcut to capture a color** under **Settings**. The suggested shortcut is **Ctrl+Alt+P**.

- **GNOME** registers a custom keyboard shortcut for native installations and AppImage. It can start a capture while the app is closed. Change the key combination in GNOME's keyboard settings.
- **KDE Plasma**, sandboxed installations, and other supported desktops use the XDG GlobalShortcuts portal. The desktop may ask you to confirm or choose the combination. Keep the app running to use a portal shortcut.

On a desktop without the portal, copy the command shown under **Settings** and register it as a custom keyboard shortcut. For a native installation on your `PATH`, use `luti-color-picker --pick`.

## Building and packaging

Install Rust 1.92 or newer, Cargo, a native build toolchain, and the GTK 4 development files.

```sh
cargo test --locked
cargo run --locked
./scripts/package-linux.sh
```

Build on a distribution compatible with your intended users. The package is written to `dist/`.

Run `make build` or `make dist` to build all available formats. The packaging script reports missing tools and build failures separately. Run `./scripts/build-package.sh FORMAT` to build a single format. See the [release guide](docs/GUIA-RELEASE.md) for tooling and installation instructions.

Rust version author **Lucas Tiago**. See [LICENSE](LICENSE) for the MIT terms and copyright notices.
