# Luti Color Picker

Linux color picker with a magnifier, scrollable visual history, and 11 copy formats, including HEX, RGB, HSL, and LAB.

## Installation

Download the package for your distribution from [GitHub Releases](https://github.com/eulucastiagolt/luti-color-picker/releases/latest). The commands below use the Linux x86_64 files from version 1.8.3. Substitute the filename you downloaded.

### AppImage

```sh
chmod +x luti-color-picker-1.8.3-linux-x86_64.AppImage
./luti-color-picker-1.8.3-linux-x86_64.AppImage
```

The AppImage includes GitHub update information. Compatible update managers use the matching `.AppImage.zsync` file. If you have an AppImage older than 1.2.0, download the new AppImage once to switch to a build with embedded update information.

### Portable launcher

```sh
chmod +x luti-color-picker-1.8.3-linux-x86_64.run
./luti-color-picker-1.8.3-linux-x86_64.run
```

### Debian and Ubuntu

```sh
sudo apt install ./luti-color-picker_1.8.3-1_amd64.deb
```

### Fedora and RPM-based distributions

```sh
sudo dnf install ./luti-color-picker-1.8.3-1.x86_64.rpm
```

### Arch Linux

```sh
sudo pacman -U ./luti-color-picker-1.8.3-1-x86_64.pkg.tar.zst
```

### Flatpak and Snap

Build these formats using the instructions in the [release guide](docs/RELEASE-GUIDE.md#build-release-packages). Install the resulting package with `flatpak install --user ./luti-color-picker-*.flatpak` or `sudo snap install --dangerous ./luti-color-picker-*.snap`.

### Verify the download

Download `luti-color-picker-1.8.3-SHA256SUMS.txt` alongside your package to verify its SHA-256 checksum. If you downloaded every asset listed in the checksum file, run `sha256sum -c luti-color-picker-1.8.3-SHA256SUMS.txt`.

### Linux archive built from source

If you built a Linux archive from source, extract it and run `./install.sh` from the extracted directory. The installer puts the application and desktop entry under `~/.local`. Add `~/.local/bin` to your `PATH` or run `~/.local/bin/luti-color-picker` directly.

The application requires GTK 4 and system libraries compatible with the build. On Wayland, the magnifier uses Spectacle on KDE Plasma or grim on compatible compositors. The XDG Desktop Portal fallback depends on the compositor and returns the selected color without a context thumbnail. Feedback sounds use the session's default PulseAudio or PipeWire output, with `pw-play` and `aplay` as fallbacks.

## Usage

Click **Capture color**, then click the pixel you want. Scroll to zoom or press Escape or right-click to cancel. Set the magnifier's minimum and maximum zoom under **Settings**.

Click a history card to open the Shades dialog and browse eleven tones from 50 to 950. The source color is marked **Original**. The **Copy** button on the card copies its color in the current format. In the dialog, use **Copy** or **Add to palette** on the row of the shade you want. In **Palettes**, use **Copy**, **Edit**, or **Shades** on a color to copy its HEX, edit its name and code, or open its shades. **Palette** adds the color to the palette selected under **Palettes**. To choose a destination each time, enable **Ask which palette to add history colors to** under **Settings**.

- **Palettes** stores named, editable palettes. Add colors from recent captures and export to PNG, SVG, TXT, CSS variables, Tailwind CSS v3 and v4, JSON, or GPL for GIMP and Inkscape.
- **Color Contrast Checker** compares text and background colors, with a live preview and WCAG 2.2 AA and AAA results for normal and large text.
- **Gradients** lets you edit colors, stop positions, and angles, copy CSS, and export PNGs.
- **Image colors** extracts dominant colors from an image and saves them as a palette.

Open **Settings** using the gear button to control automatic copying, sound feedback, tray behavior, and startup at login. Startup at login is available for native installations, AppImage, and the `.run` package.
**Start in the system tray at login** is enabled by default and applies to automatic startup at login. Open the app from your application menu to show the main window. On supported installations, **Start at login** is enabled on first launch when no startup entry exists. You can change both options in **Settings**.

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

## Updates

The AppImage includes GitHub Releases metadata for compatible AppImage managers. Install the AppImage from version 1.2.0 or later to enable update discovery. While running, the app checks for newer stable versions at startup and every six hours. Select **View release** in the desktop notification to open the release page.

## Building and packaging

Install Rust 1.92 or newer, Cargo, a native build toolchain, and the GTK 4 development files.

```sh
cargo test --locked
cargo run --locked
./scripts/package-linux.sh
```

Build on a distribution compatible with your intended users. The package is written to `dist/`.

Run `make build` or `make dist` to build all available formats. The packaging script reports missing tools and build failures separately. Run `./scripts/build-package.sh FORMAT` to build a single format. See the [release guide](docs/RELEASE-GUIDE.md) for packaging tools and release verification.

Rust version author **Lucas Tiago**. See [LICENSE](LICENSE) for the MIT terms and copyright notices.
