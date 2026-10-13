#!/usr/bin/env bash
# Builds the LibreSpydus Rust core (core/) for iOS and packages it as
# ios/LibreSpydusCore/LibreSpydusCore.xcframework (static library, device + simulator).
#
# The generated Swift bindings are NOT copied: ios/LibreSpydusCore/Sources/LibreSpydusCore/
# librespydus_core.swift is a committed symlink to core/generated/swift/librespydus_core.swift,
# so core/generated/swift/ stays the single source of truth.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ios/scripts/build-core.sh [--release | --debug] [--from <LibreSpydusCore.xcframework>]

Builds librespydus-core (core/) as a static library for the iOS targets
  aarch64-apple-ios           device
  aarch64-apple-ios-sim       simulator (Apple silicon)
  x86_64-apple-ios            simulator (Intel)
lipos the two simulator slices and creates
  ios/LibreSpydusCore/LibreSpydusCore.xcframework
with core/generated/swift/librespydus_coreFFI.h and a module map exposing the Clang module
`librespydus_coreFFI` (what the generated Swift imports). That xcframework is the only build
output the Xcode project needs; it's gitignored.

Options
  --release        optimised build (default)
  --debug          debug build (faster to compile, much slower at runtime)
  --from <path>    don't build: copy a prebuilt LibreSpydusCore.xcframework, e.g. the
                   coordinator's $MAIN/ios/LibreSpydusCore/LibreSpydusCore.xcframework
  -h, --help       show this help

Environment
  CARGO_TARGET_DIR      honoured (default: core/target)
  IPHONEOS_DEPLOYMENT_TARGET  default 17.0 (the app's minimum)

Prerequisites (build mode): Xcode, rustup with
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
EOF
}

PROFILE=release
FROM=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) PROFILE=release ;;
    --debug) PROFILE=debug ;;
    --from)
      [[ $# -ge 2 ]] || { echo "build-core.sh: --from needs a path" >&2; exit 2; }
      FROM="$2"
      shift
      ;;
    -h | --help) usage; exit 0 ;;
    *) echo "build-core.sh: unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
  shift
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IOS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_DIR="$(cd "$IOS_DIR/.." && pwd)"
CORE_DIR="$REPO_DIR/core"
GENERATED="$CORE_DIR/generated/swift"
PACKAGE_DIR="$IOS_DIR/LibreSpydusCore"
OUT="$PACKAGE_DIR/LibreSpydusCore.xcframework"
BINDINGS_LINK="$PACKAGE_DIR/Sources/LibreSpydusCore/librespydus_core.swift"

log() { printf '==> %s\n' "$*"; }
die() { printf 'build-core.sh: %s\n' "$*" >&2; exit 1; }

# The package compiles the generated Swift through a symlink; make sure it still resolves.
[[ -f "$BINDINGS_LINK" ]] || die "missing $BINDINGS_LINK (expected a symlink to core/generated/swift/librespydus_core.swift)"
[[ -f "$GENERATED/librespydus_coreFFI.h" ]] || die "missing $GENERATED/librespydus_coreFFI.h"

if [[ -n "$FROM" ]]; then
  FROM="${FROM%/}"
  [[ -f "$FROM/Info.plist" ]] || die "--from: $FROM is not an .xcframework (no Info.plist)"
  log "Copying prebuilt $FROM"
  rm -rf "$OUT"
  cp -R "$FROM" "$OUT"
  # A prebuilt framework from another checkout must match these bindings, or the app traps
  # at launch with "UniFFI API checksum mismatch".
  stale=0
  while IFS= read -r header; do
    cmp -s "$header" "$GENERATED/librespydus_coreFFI.h" || stale=1
  done < <(find "$OUT" -name librespydus_coreFFI.h)
  if [[ $stale -ne 0 ]]; then
    echo "build-core.sh: warning: the copied FFI header differs from core/generated/swift/librespydus_coreFFI.h;" >&2
    echo "build-core.sh: warning: rebuild with ios/scripts/build-core.sh (no --from) if the app traps at launch." >&2
  fi
  log "Done: $OUT"
  exit 0
fi

command -v cargo >/dev/null || die "cargo not found (install rustup)"
command -v xcodebuild >/dev/null || die "xcodebuild not found (install Xcode)"

TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios)
installed="$(rustup target list --installed 2>/dev/null || true)"
for t in "${TARGETS[@]}"; do
  grep -qx "$t" <<<"$installed" || die "Rust target $t missing: rustup target add ${TARGETS[*]}"
done

TARGET_DIR="${CARGO_TARGET_DIR:-$CORE_DIR/target}"
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}"
CARGO_FLAGS=()
[[ "$PROFILE" == release ]] && CARGO_FLAGS+=(--release)

for t in "${TARGETS[@]}"; do
  log "cargo build ($PROFILE) --target $t"
  # `cargo rustc --crate-type staticlib` skips the cdylib/rlib the iOS app doesn't use.
  cargo rustc --manifest-path "$CORE_DIR/Cargo.toml" --lib --crate-type staticlib \
    --target "$t" ${CARGO_FLAGS[@]+"${CARGO_FLAGS[@]}"}
done

LIB=liblibrespydus_core.a
DEVICE_LIB="$TARGET_DIR/aarch64-apple-ios/$PROFILE/$LIB"
SIM_ARM_LIB="$TARGET_DIR/aarch64-apple-ios-sim/$PROFILE/$LIB"
SIM_X86_LIB="$TARGET_DIR/x86_64-apple-ios/$PROFILE/$LIB"
for f in "$DEVICE_LIB" "$SIM_ARM_LIB" "$SIM_X86_LIB"; do
  [[ -f "$f" ]] || die "expected build output $f"
done

STAGE="$TARGET_DIR/ios-xcframework-stage"
rm -rf "$STAGE"
mkdir -p "$STAGE/simulator" "$STAGE/Headers/librespydus_coreFFI"

log "lipo simulator slices"
lipo -create "$SIM_ARM_LIB" "$SIM_X86_LIB" -output "$STAGE/simulator/$LIB"

# Headers live in a module-named subdirectory so the module map can't collide with another
# xcframework's module.modulemap in the build's shared include directory.
cp "$GENERATED/librespydus_coreFFI.h" "$STAGE/Headers/librespydus_coreFFI/"
cp "$GENERATED/librespydus_coreFFI.modulemap" "$STAGE/Headers/librespydus_coreFFI/module.modulemap"

log "xcodebuild -create-xcframework"
rm -rf "$OUT"
xcodebuild -create-xcframework \
  -library "$DEVICE_LIB" -headers "$STAGE/Headers" \
  -library "$STAGE/simulator/$LIB" -headers "$STAGE/Headers" \
  -output "$OUT" >/dev/null

log "Done: $OUT"
