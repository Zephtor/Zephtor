#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "$0")" && pwd)"
if [ -x "$HOME/.cargo/bin/cargo" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
fi
app_id="org.zephtor.MCLauncher"
manifest="$root/org.zephtor.MCLauncher.yml"
stage="$root/.flatpak-src"
build_dir="$root/.flatpak-build"
state_dir="$root/.flatpak-build-state"
repo="$root/.flatpak-repo"
dist="$root/dist"
tmp="$(mktemp -d)"
target_dir="${CARGO_TARGET_DIR:-$tmp/target}"

for path in "$stage" "$build_dir" "$state_dir" "$repo"; do
    if [ -e "$path" ]; then
        printf 'Build-Pfad existiert bereits; bitte erst prüfen: %s\n' "$path" >&2
        exit 1
    fi
done

cleanup() {
    rm -rf -- "$stage" "$build_dir" "$state_dir" "$repo" "$tmp"
}
trap cleanup EXIT

bundle="$dist/ZephTor-MC-Launcher.flatpak"
if [ -e "$bundle" ]; then
    printf 'Bundle existiert bereits; bitte erst prüfen: %s\n' "$bundle" >&2
    exit 1
fi

mkdir -p "$stage" "$dist"
CARGO_TARGET_DIR="$target_dir" cargo build \
    --manifest-path "$root/Cargo.toml" --locked --release
cp "$target_dir/release/zephtor-mc-launcher" "$stage/zephtor-mc-launcher"
cp "$root/launcher-config.example" "$root/org.zephtor.MCLauncher.desktop" \
    "$root/org.zephtor.MCLauncher.metainfo.xml" "$stage/"
mkdir -p "$stage/assets"
cp "$root/assets/launcher-icon.svg" "$stage/assets/"

flatpak-builder --force-clean --state-dir="$state_dir" --repo="$repo" "$build_dir" "$manifest"
flatpak build-bundle "$repo" "$bundle" "$app_id" stable
printf 'Flatpak erstellt: %s\n' "$bundle"
