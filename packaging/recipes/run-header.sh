#!/bin/sh
set -eu

stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
offset=$(awk '/^__LUTI_ARCHIVE__$/ { print NR + 1; exit }' "$0")
tail -n +"$offset" "$0" | tar -xz -C "$stage"
"$stage/AppRun" "$@"
exit
__LUTI_ARCHIVE__
