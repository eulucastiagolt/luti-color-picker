#!/usr/bin/env bash
set -euo pipefail

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
format=${1:-}
output=${2:-"$root/dist"}
app=luti-color-picker
app_id=io.github.eulucastiagolt.LutiColorPicker
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$root/Cargo.toml")
arch=$(uname -m)
name="$app-$version-linux-$arch"

if [[ "$format" == all ]]; then
    built=()
    skipped=()
    failed=()
    for package in appimage run deb rpm arch flatpak snap; do
        printf '\n==> %s\n' "$package"
        if "$root/scripts/build-package.sh" "$package" "$output"; then
            built+=("$package")
        else
            result=$?
            if (( result == 77 )); then
                skipped+=("$package")
            else
                failed+=("$package")
            fi
        fi
    done

    if (( ${#built[@]} > 0 )); then
        printf '\nPackages created in %s: %s\n' "$output" "${built[*]}"
    fi
    if (( ${#skipped[@]} > 0 )); then
        printf 'Skipped (required tools missing): %s\n' "${skipped[*]}" >&2
    fi
    if (( ${#failed[@]} > 0 )); then
        printf 'Packaging failed: %s\n' "${failed[*]}" >&2
        exit 1
    fi
    if (( ${#built[@]} == 0 )); then
        printf 'No packages were created.\n' >&2
        exit 1
    fi
    exit 0
fi

case "$format" in
    appimage) required=(appimagetool) ;;
    run) required=(tar gzip) ;;
    deb) required=(dpkg-deb readelf) ;;
    rpm) required=(rpmbuild) ;;
    arch) required=(makepkg zstd readelf) ;;
    flatpak) required=(flatpak flatpak-builder) ;;
    snap) required=(snapcraft) ;;
    *) printf 'Usage: %s {all|appimage|run|deb|rpm|arch|flatpak|snap} [output-directory]\n' "$0" >&2; exit 2 ;;
esac

for tool in "${required[@]}"; do
    if ! command -v "$tool" >/dev/null; then
        printf 'Required tool for %s: %s\n' "$format" "$tool" >&2
        exit 77
    fi
done

trap 'printf "%s: packaging stopped at line %s (exit %s)\n" "$format" "$LINENO" "$?" >&2' ERR

mkdir -p "$output"
output=$(cd -- "$output" && pwd)
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
cd "$root"

copy_source() {
    mkdir -p "$stage/source"
    cp -a Cargo.toml Cargo.lock LICENSE src packaging snap "$stage/source/"
}

case "$format" in
    flatpak)
        copy_source
        mkdir -p "$stage/source/.cargo"
        cargo vendor --locked "$stage/source/vendor" |
            sed 's@^directory = .*@directory = "vendor"@' > "$stage/source/.cargo/config.toml"
        cp "packaging/recipes/$app_id.yml" "$stage/"
        flatpak-builder --state-dir="$stage/state" --user --install-deps-from=flathub --repo="$stage/repo" "$stage/build" "$stage/$app_id.yml"
        flatpak build-bundle "$stage/repo" "$output/$name.flatpak" "$app_id" \
            --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
        exit
        ;;
    snap)
        copy_source
        cd "$stage/source"
        mkdir "$stage/snap-output"
        snapcraft pack --output "$stage/snap-output"
        snaps=("$stage"/snap-output/*.snap)
        test "${#snaps[@]}" -eq 1
        mv -- "${snaps[0]}" "$output/$name.snap"
        exit
        ;;
esac

target_dir=${CARGO_TARGET_DIR:-"$root/target"}
cargo build --release --locked --target-dir "$target_dir"
mkdir -p "$stage/payload"
cp packaging/install.sh packaging/"$app_id".{desktop,png,metainfo.xml} LICENSE "$stage/payload/"
install -m755 "$target_dir/release/$app" "$stage/payload/$app"
strip --strip-unneeded "$stage/payload/$app"
if [[ "$format" == deb || "$format" == arch ]]; then
    glibc=$(LC_ALL=C readelf --version-info "$stage/payload/$app" |
        awk '/Name: GLIBC_[0-9]/{for(i=1;i<=NF;i++) if($i ~ /^GLIBC_[0-9]/) print substr($i,7)}' |
        sort -Vu | tail -n1)
    if [[ -z "$glibc" ]]; then
        printf '%s: could not determine the minimum glibc version of %s.\n' "$format" "$app" >&2
        exit 1
    fi
fi

appdir="$stage/AppDir"
PREFIX="$appdir/usr" sh "$stage/payload/install.sh"

case "$format" in
    appimage|run)
        install -m755 packaging/recipes/AppRun "$appdir/AppRun"
        cp "packaging/$app_id.desktop" "packaging/$app_id.png" "$appdir/"
        if [[ "$format" == appimage ]]; then
            mv "$appdir/usr/share/metainfo/$app_id.metainfo.xml" "$appdir/usr/share/metainfo/$app_id.appdata.xml"
            ARCH="$arch" appimagetool "$appdir" "$output/$name.AppImage"
        else
            tar -C "$appdir" -czf "$stage/appdir.tar.gz" .
            cat packaging/recipes/run-header.sh "$stage/appdir.tar.gz" > "$output/$name.run"
            chmod +x "$output/$name.run"
        fi
        ;;
    deb)
        case "$arch" in
            x86_64) deb_arch=amd64 ;;
            aarch64) deb_arch=arm64 ;;
            *) printf 'Unsupported Debian architecture: %s\n' "$arch" >&2; exit 2 ;;
        esac
        install -Dm644 LICENSE "$appdir/usr/share/doc/$app/copyright"
        rm -r -- "$appdir/usr/share/licenses"
        mkdir -p "$appdir/DEBIAN"
        cat > "$appdir/DEBIAN/control" <<CONTROL
Package: $app
Version: $version-1
Section: graphics
Priority: optional
Architecture: $deb_arch
Maintainer: Lucas Tiago <eulucastiago@outlook.com>
Depends: libc6 (>= $glibc), libgtk-4-1
Recommends: libpulse0
Suggests: xdg-desktop-portal, spectacle | grim
Homepage: https://github.com/eulucastiagolt/luti-color-picker
Description: Screen color picker with a magnifier and visual history
 Capture and copy screen colors in eleven formats.
CONTROL
        dpkg-deb --root-owner-group --build "$appdir" "$output/${app}_${version}-1_${deb_arch}.deb"
        ;;
    rpm)
        mkdir -p "$stage/rpm/"{SOURCES,tmp,db}
        tar -C "$appdir" -czf "$stage/rpm/SOURCES/payload.tar.gz" usr
        rpmbuild -bb --define "_topdir $stage/rpm" --define "_rpmdir $output" \
            --define "_tmppath $stage/rpm/tmp" --define "_dbpath $stage/rpm/db" \
            --define "_rpmfilename %{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}.rpm" \
            --define "luti_version $version" packaging/recipes/luti-color-picker.spec
        ;;
    arch)
        mkdir -p "$stage/arch"
        tar -C "$appdir" -czf "$stage/arch/payload.tar.gz" usr
        checksum=$(sha256sum "$stage/arch/payload.tar.gz" | cut -d' ' -f1)
        sed -e "s/@VERSION@/$version/g" -e "s/@ARCH@/$arch/g" \
            -e "s/@GLIBC@/$glibc/g" -e "s/@SHA256@/$checksum/g" packaging/recipes/PKGBUILD.in > "$stage/arch/PKGBUILD"
        cd "$stage/arch"
        PKGDEST="$output" PKGEXT=.pkg.tar.zst makepkg --clean --force
        ;;
esac
