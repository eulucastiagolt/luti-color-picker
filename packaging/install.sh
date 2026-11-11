#!/bin/sh
set -eu

package_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
prefix=${PREFIX:-"$HOME/.local"}
app_id=io.github.eulucastiagolt.LutiColorPicker

install -Dm755 "$package_dir/luti-color-picker" "$prefix/bin/luti-color-picker"
install -Dm644 "$package_dir/$app_id.desktop" "$prefix/share/applications/$app_id.desktop"
install -Dm644 "$package_dir/$app_id.png" "$prefix/share/icons/hicolor/256x256/apps/$app_id.png"
install -Dm644 "$package_dir/LICENSE" "$prefix/share/licenses/luti-color-picker/LICENSE"

printf 'Luti Color Picker installed in %s\n' "$prefix"
