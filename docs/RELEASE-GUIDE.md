# Luti Color Picker 1.8.3

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

Install or launch the package using the [README](../README.md#installation).

Confirm the version under **About** or by running `luti-color-picker --version`. Capture a pixel, check the copied value, and exercise both history-to-palette modes.

## Tag and publish

Review the changes since the previous public tag. Write the release commit in English, with `Release Luti Color Picker X.Y.Z` as its subject. In the body, summarize the user-facing features and fixes, list any changes to packaging or updates, and record the validation performed. Create an annotated tag and push it together with the release commit.

```sh
git tag -a v1.8.3 -m "Luti Color Picker 1.8.3"
git push origin main v1.8.3
```

Write the GitHub release notes in English. Use the notes below as a model. Group changes by feature and describe what users can do, where to find each feature, and what changed. Include fixes and verified behavior. Link to the README for installation commands. Attach the packages built for the tag. Upload the AppImage and its matching `.AppImage.zsync` file to the same stable release so AppImage managers can find the update for the installed architecture.

## GitHub release notes

Luti Color Picker 1.8.3 fixes how the app starts at login and how it opens from the application menu.

### Startup fixes

- **Start in the tray at login.** Enable **Start in the system tray at login** in Settings to keep the main window off the taskbar during automatic startup. If a compatible tray is unavailable, the app opens the main window.
- **Open the window from the application menu.** Launching Luti Color Picker manually shows the main window, including when the app is already running in the tray.
- **Preserve existing startup preferences.** On the first run after updating, the app migrates startup entries created by earlier versions and preserves whether startup at login is enabled.

### Verification

An isolated desktop session verified tray startup, reopening from the application menu, startup with the tray option disabled, and fallback on a desktop without a tray. All 11 automated tests passed.

### Downloads

See the [README](https://github.com/eulucastiagolt/luti-color-picker/blob/main/README.md#installation) for installation instructions.
