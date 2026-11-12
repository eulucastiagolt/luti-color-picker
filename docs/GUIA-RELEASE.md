# Luti Color Picker 1.1.0

## Build release packages

Run `make build` or `make dist` from the repository root. The script attempts each format, reports missing tools and build failures, and writes the artifacts to `dist/`. Pass a second argument to the packaging script to use a different output directory.

| Format | Command | Additional tools |
| --- | --- | --- |
| AppImage | `./scripts/build-package.sh appimage` | `appimagetool` |
| Debian and Ubuntu | `./scripts/build-package.sh deb` | `dpkg-deb` |
| Fedora and derivatives | `./scripts/build-package.sh rpm` | `rpmbuild` and the distribution's RPM tooling |
| Arch and derivatives | `./scripts/build-package.sh arch` | `makepkg` and `base-devel`, as a regular user |
| Portable AppRun | `./scripts/build-package.sh run` | GNU tar and gzip |
| Flatpak | `./scripts/build-package.sh flatpak` | `flatpak`, `flatpak-builder`, and a user Flathub remote |
| Snap | `./scripts/build-package.sh snap` | Snapcraft configured for core24 |

Native packages require Rust, Cargo, GTK 4 development files, `pkg-config`, and GNU binutils. Build on the oldest distribution you intend to support and repeat the build for each target architecture. The build host determines the minimum glibc version. AppImage and the `.run` package use GTK and other shared libraries from the host. If `appimagetool` is itself an AppImage and FUSE is unavailable, set `APPIMAGE_EXTRACT_AND_RUN=1`.

Flatpak uses the GNOME 49 SDK and vendored Rust dependencies. To add the user Flathub remote, run:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

On Wayland, Flatpak and Snap can fall back to the desktop portal when Spectacle and grim are unavailable. Verify capture, clipboard, audio feedback, global shortcuts, and tray behavior on the target desktops.

## Test the packages

| Format | How to run or install |
| --- | --- |
| AppImage | Mark the file executable with `chmod +x` and run it. |
| Portable AppRun | Mark the `.run` file executable with `chmod +x` and run it. |
| Debian and Ubuntu | `sudo apt install ./luti-color-picker_*.deb` |
| Fedora | `sudo dnf install ./luti-color-picker-*.rpm` |
| Arch | `sudo pacman -U ./luti-color-picker-*.pkg.tar.zst` |
| Flatpak | `flatpak install --user ./luti-color-picker-*.flatpak` |
| Local Snap | `sudo snap install --dangerous ./luti-color-picker-*.snap` |

Confirm the version under **About** or by running `luti-color-picker --version`. Capture a pixel, check the copied value, and exercise both history-to-palette modes.

## Tag and publish

Review and commit the release changes. Create an annotated tag and push it together with the release commit.

```sh
git tag -a v1.1.0 -m "Luti Color Picker 1.1.0"
git push origin main v1.1.0
```

Use the notes below for the GitHub release and attach the packages built for the tag.

## GitHub release notes

Luti Color Picker 1.1.0 brings global color capture, a configurable magnifier, and a faster way to add colors from your history to a palette.

### Capture from anywhere

Enable **Register a global shortcut to capture a color** in **Settings**. The suggested combination is **Ctrl+Alt+P**. On GNOME, native installations can register a desktop shortcut that also launches capture when the app is closed. On KDE Plasma and other supported desktops, registration uses the XDG GlobalShortcuts portal and the app must be running. **Settings** also shows the command for desktops that use custom keyboard shortcuts.

### Magnifier and palettes

- The magnifier opens at the pointer even when the mouse is stationary. Set the minimum and maximum zoom in **Settings**. The default range is **2× to 7×**.
- **Palette** on a history card now adds the color directly to the palette selected in **Palettes**. Enable **Ask which palette to add history colors to** to choose the destination for each color.
- Numeric controls now have separate input and button outlines, with spacing between the field and the step buttons.

### Desktop integration and interface

- The app now uses GTK 4. The header has one app icon and simpler window controls.
- **Settings** controls automatic copying, sound feedback, staying in the tray after closing the window, and startup at login on supported installations.
- Capture and copy play distinct built-in sounds through the system's default audio output.
- X11 screen capture uses the X11 connection directly. On Wayland, magnifier availability depends on Spectacle or grim, with a portal color picker available on supported desktops.

### Linux downloads

The x86_64 downloads include AppImage, a portable `.run` launcher, Debian/Ubuntu (`.deb`), Fedora-compatible RPM (`.rpm`), and Arch Linux (`.pkg.tar.zst`). These builds require GTK 4 and glibc 2.39 or newer. The SHA-256 manifest covers all five packages. Flatpak and Snap build recipes are available in the source tree.

Mark the AppImage or `.run` file executable before launching it. For distro packages, use your system package manager. See the [release guide](https://github.com/eulucastiagolt/luti-color-picker/blob/v1.1.0/docs/GUIA-RELEASE.md#test-the-packages) for installation commands.
