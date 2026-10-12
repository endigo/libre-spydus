#!/usr/bin/env bash
# Build the LibreSpydus Rust core (core/) for Android with cargo-ndk and place the shared
# libraries where Gradle packages them: android/app/src/main/jniLibs/<abi>/liblibrespydus_core.so
#
# The Kotlin bindings are NOT generated here: the app compiles core/generated/kotlin directly
# (see android/app/build.gradle.kts), so the .so and the committed bindings must come from the
# same core revision.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: android/scripts/build-core.sh [options]

Builds core/ (crate librespydus-core) with cargo-ndk into
android/app/src/main/jniLibs/<abi>/liblibrespydus_core.so, 16 KB page-aligned.

Options:
  --release            Optimised build (default).
  --debug              Debug build (faster to compile, much larger .so).
  --abi <list>         Comma-separated Android ABIs (default: arm64-v8a,x86_64).
                       Known: arm64-v8a, armeabi-v7a, x86_64.
  --all                All ABIs: arm64-v8a, armeabi-v7a, x86_64.
  --from <dir>         Copy prebuilt jniLibs from <dir> (a directory holding <abi>/ folders,
                       e.g. $MAIN/android/app/src/main/jniLibs) instead of building.
  --no-verify          Skip the 16 KB ELF alignment check.
  -h, --help           Show this help.

Environment:
  ANDROID_HOME         Android SDK (default: ~/Library/Android/sdk).
  ANDROID_NDK_HOME     NDK (default: $ANDROID_HOME/ndk/28.2.13676358).
  CARGO_TARGET_DIR     Honoured; default is core/target.

Prerequisites (once):
  rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
  cargo install cargo-ndk
EOF
}

NDK_VERSION="28.2.13676358"
MIN_SDK=26
PAGE_SIZE=16384

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANDROID_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_DIR="$(cd "$ANDROID_DIR/.." && pwd)"
CORE_DIR="$REPO_DIR/core"
JNI_DIR="$ANDROID_DIR/app/src/main/jniLibs"
LIB_NAME="liblibrespydus_core.so"

profile="release"
abis="arm64-v8a,x86_64"
from=""
verify=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) profile="release" ;;
    --debug) profile="debug" ;;
    --abi)
      [[ $# -ge 2 ]] || { echo "error: --abi needs a value" >&2; exit 2; }
      abis="$2"; shift ;;
    --abi=*) abis="${1#--abi=}" ;;
    --all) abis="arm64-v8a,armeabi-v7a,x86_64" ;;
    --from)
      [[ $# -ge 2 ]] || { echo "error: --from needs a directory" >&2; exit 2; }
      from="$2"; shift ;;
    --from=*) from="${1#--from=}" ;;
    --no-verify) verify=0 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

IFS=',' read -r -a abi_list <<<"$abis"
for abi in "${abi_list[@]}"; do
  case "$abi" in
    arm64-v8a|armeabi-v7a|x86_64) ;;
    *) echo "error: unsupported ABI '$abi' (use arm64-v8a, armeabi-v7a, x86_64)" >&2; exit 2 ;;
  esac
done

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/$NDK_VERSION}"

readelf_bin() {
  local r
  r="$(ls "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-readelf 2>/dev/null | head -1 || true)"
  if [[ -n "$r" ]]; then echo "$r"; elif command -v llvm-readelf >/dev/null; then command -v llvm-readelf; fi
}

# Every PT_LOAD segment must be aligned to at least 16 KB to load on 16 KB-page devices
# (the ps16k emulator images, and Android 15+ devices that use 16 KB pages).
verify_alignment() {
  local so="$1" readelf aligns bad=0
  readelf="$(readelf_bin)"
  if [[ -z "$readelf" ]]; then
    echo "warning: llvm-readelf not found; skipping alignment check for $so" >&2
    return 0
  fi
  aligns="$("$readelf" -lW "$so" | awk '$1 == "LOAD" { print $NF }')"
  [[ -n "$aligns" ]] || { echo "error: no LOAD segments in $so" >&2; return 1; }
  for a in $aligns; do
    if (( a < PAGE_SIZE )); then bad=1; fi
  done
  if (( bad )); then
    echo "error: $so is not 16 KB aligned (LOAD align: $(echo $aligns | tr '\n' ' '))" >&2
    return 1
  fi
  echo "  16 KB aligned: LOAD align $(echo $aligns | tr '\n' ' ')"
}

if [[ -n "$from" ]]; then
  [[ -d "$from" ]] || { echo "error: --from: no such directory: $from" >&2; exit 1; }
  for abi in "${abi_list[@]}"; do
    src="$from/$abi/$LIB_NAME"
    [[ -f "$src" ]] || { echo "error: --from: missing $src" >&2; exit 1; }
    mkdir -p "$JNI_DIR/$abi"
    cp "$src" "$JNI_DIR/$abi/$LIB_NAME"
    echo "copied $src -> $JNI_DIR/$abi/$LIB_NAME"
    if (( verify )); then verify_alignment "$JNI_DIR/$abi/$LIB_NAME"; fi
  done
  exit 0
fi

command -v cargo >/dev/null || { echo "error: cargo not found (install Rust via rustup)" >&2; exit 1; }
cargo ndk --version >/dev/null 2>&1 || { echo "error: cargo-ndk not found: cargo install cargo-ndk" >&2; exit 1; }
[[ -d "$ANDROID_NDK_HOME" ]] || { echo "error: NDK not found at $ANDROID_NDK_HOME (install NDK $NDK_VERSION)" >&2; exit 1; }

ndk_args=()
for abi in "${abi_list[@]}"; do
  ndk_args+=(-t "$abi")
  case "$abi" in
    arm64-v8a) triple="aarch64-linux-android" ;;
    armeabi-v7a) triple="armv7-linux-androideabi" ;;
    x86_64) triple="x86_64-linux-android" ;;
  esac
  rustup target list --installed 2>/dev/null | grep -qx "$triple" \
    || { echo "error: Rust target $triple missing: rustup target add $triple" >&2; exit 1; }
  # NDK r28 links 16 KB-aligned by default; pass it explicitly so an older NDK can't regress it.
  # Target-scoped so it never replaces rustflags from the user's own cargo config for the host.
  var="CARGO_TARGET_$(echo "$triple" | tr '[:lower:]-' '[:upper:]_')_RUSTFLAGS"
  export "$var=${!var:-} -C link-arg=-Wl,-z,max-page-size=$PAGE_SIZE"
done

cargo_args=(build --lib)
[[ "$profile" == "release" ]] && cargo_args+=(--release)

echo "building librespydus-core ($profile) for ${abi_list[*]} (NDK $(basename "$ANDROID_NDK_HOME"), API $MIN_SDK)"
mkdir -p "$JNI_DIR"
(
  cd "$CORE_DIR"
  cargo ndk --platform "$MIN_SDK" "${ndk_args[@]}" -o "$JNI_DIR" "${cargo_args[@]}"
)

for abi in "${abi_list[@]}"; do
  so="$JNI_DIR/$abi/$LIB_NAME"
  [[ -f "$so" ]] || { echo "error: expected $so after the build" >&2; exit 1; }
  echo "$so ($(du -h "$so" | cut -f1 | tr -d ' '))"
  if (( verify )); then verify_alignment "$so"; fi
done
echo "done"
