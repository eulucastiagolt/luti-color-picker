# Luti Color Picker 1.6.2

## Build release packages

Run `make build` or `make dist` from the repository root. The script attempts each format, reports missing tools and build failures, and writes the artifacts to `dist/`. Pass a second argument to the packaging script to use a different output directory.

| Format | Command | Additional tools |
| --- | --- | --- |
| AppImage | `./scripts/build-package.sh appimage` | `appimagetool` and `zsyncmake` |
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
git tag -a v1.6.2 -m "Luti Color Picker 1.6.2"
git push origin main v1.6.2
```

Use the notes below for the GitHub release and attach the packages built for the tag. Upload the AppImage and its matching `.AppImage.zsync` file to the same stable release. Keep their generated filenames so AppImage managers can find the right update for the installed architecture.

## GitHub release notes

Open Shades from History or Palettes to view, copy, and save individual tones. Each shade has its own row with its HEX code and actions to copy it or add it to a palette. Palette colors can be copied and edited by HEX.

Palette dialogs use the application title bar and consistent content spacing.

The magnifier now follows the pointer across monitors during one capture session.

### Linux downloads

The x86_64 release includes AppImage, a portable `.run` launcher, Debian/Ubuntu, RPM, and Arch Linux packages. Flatpak and Snap build recipes are available in the source tree.

If you currently use an AppImage from a release before 1.2.0, download the 1.6.2 AppImage once to switch to a build with embedded update information. The `.zsync` file is used by update tools. To launch the app, download the `.AppImage` file.
