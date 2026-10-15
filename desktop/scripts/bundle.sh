#!/bin/sh
# Builds LibreSpydus.app from a release build (docs/desktop-app.md §11.5).
#
#   desktop/scripts/bundle.sh [--dmg]
#
# - `cargo build --release`;
# - assembles desktop/target/LibreSpydus.app: Contents/MacOS/librespydus, Contents/Info.plist
#   (packaging/Info.plist with the version from Cargo.toml), Contents/PkgInfo and
#   Contents/Resources/AppIcon.icns, made here from the iOS 1024 px icon with sips + iconutil;
# - signs it ad hoc (`codesign --force --deep -s -`) and checks the signature;
# - with --dmg, also makes desktop/target/LibreSpydus-<version>.dmg holding the app and an
#   Applications link.
#
# macOS only; needs the Command Line Tools (no Xcode).
set -eu

usage() {
    echo "usage: $0 [--dmg]" >&2
    exit 2
}

dmg=0
while [ $# -gt 0 ]; do
    case $1 in
        --dmg) dmg=1 ;;
        *) usage ;;
    esac
    shift
done

[ "$(uname -s)" = Darwin ] || { echo "bundle.sh: macOS only" >&2; exit 1; }

here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/.." && pwd)
target=$here/target
icon_src=$repo/ios/LibreSpydus/Resources/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png
name=LibreSpydus
app=$target/$name.app

# The package version: the first `version = "…"` line, which is in [package].
version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "$here/Cargo.toml" | head -n 1)
[ -n "$version" ] || { echo "bundle.sh: no version in Cargo.toml" >&2; exit 1; }
[ -f "$icon_src" ] || { echo "bundle.sh: missing $icon_src" >&2; exit 1; }

echo "==> cargo build --release ($name $version)"
cargo build --manifest-path "$here/Cargo.toml" --release --bin librespydus

echo "==> $app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$target/release/librespydus" "$app/Contents/MacOS/librespydus"
sed "s/@VERSION@/$version/g" "$here/packaging/Info.plist" >"$app/Contents/Info.plist"
plutil -lint -s "$app/Contents/Info.plist"
printf 'APPL????' >"$app/Contents/PkgInfo"

echo "==> AppIcon.icns"
iconset=$target/packaging/AppIcon.iconset
rm -rf "$iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    double=$((size * 2))
    sips -z "$size" "$size" "$icon_src" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z "$double" "$double" "$icon_src" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"

echo "==> codesign (ad hoc)"
codesign --force --deep -s - "$app"
codesign --verify --deep --strict --verbose=2 "$app"

if [ "$dmg" = 1 ]; then
    image=$target/$name-$version.dmg
    echo "==> $image"
    stage=$target/packaging/dmg
    rm -rf "$stage" "$image"
    mkdir -p "$stage"
    cp -R "$app" "$stage/"
    ln -s /Applications "$stage/Applications"
    hdiutil create -volname "$name $version" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$image" >/dev/null
    rm -rf "$stage"
    hdiutil verify -quiet "$image"
    echo "dmg: $image"
fi

echo "app: $app"
