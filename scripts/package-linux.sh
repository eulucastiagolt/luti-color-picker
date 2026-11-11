#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
output=${1:-"$root/target/dist"}
target_dir=${CARGO_TARGET_DIR:-"$root/target"}
mkdir -p "$output"
output=$(CDPATH='' cd -- "$output" && pwd)
cd "$root"

cargo build --release --locked --target-dir "$target_dir"
binary="$target_dir/release/luti-color-picker"
version=$("$binary" --version)
version=${version##* }
name="luti-color-picker-$version-linux-$(uname -m)"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir "$stage/$name"
install -m755 "$binary" "$stage/$name/luti-color-picker"
strip --strip-unneeded "$stage/$name/luti-color-picker"
cp "$root"/packaging/* "$root/LICENSE" "$root/README.md" "$stage/$name/"
tar -C "$stage" -czf "$output/$name.tar.gz" "$name"
printf '%s\n' "$output/$name.tar.gz"
