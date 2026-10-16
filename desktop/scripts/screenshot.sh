#!/bin/sh
# Renders the desktop app's own window to a PNG (docs/desktop-app.md §5.7, §9).
#
#   desktop/scripts/screenshot.sh <out.png> [--wait <s>] -- <app flags>
#
# Builds the debug binary with `--features evidence`, runs it with
# `--evidence --screenshot <out.png> --screenshot-after <s>` plus the flags after `--`, and waits
# for it to exit. The app renders its window with GPUI's `render_to_image()`, which works over SSH
# (`screencapture` doesn't). Pass `--data-dir` under /tmp/librespydus-evidence/<task>/ and `--demo`
# for anything signed in (docs/desktop-app.md §2).
#
# Example:
#   desktop/scripts/screenshot.sh /tmp/librespydus-evidence/D1/home.png -- \
#     --data-dir /tmp/librespydus-evidence/D1/data --demo --library https://canning.spydus.com --route home
set -eu

usage() {
    echo "usage: $0 <out.png> [--wait <seconds>] -- <app flags>" >&2
    exit 2
}

[ $# -ge 1 ] || usage
out=$1
shift
wait=6
while [ $# -gt 0 ]; do
    case $1 in
        --wait)
            [ $# -ge 2 ] || usage
            wait=$2
            shift 2
            ;;
        --)
            shift
            break
            ;;
        *) usage ;;
    esac
done

here=$(cd "$(dirname "$0")/.." && pwd)
cargo build --manifest-path "$here/Cargo.toml" --features evidence --quiet
mkdir -p "$(dirname "$out")"
rm -f "$out"
"$here/target/debug/librespydus" --evidence --screenshot "$out" --screenshot-after "$wait" "$@"
if [ -f "$out" ]; then
    echo "screenshot: $out"
else
    echo "screenshot: no image was written" >&2
    exit 1
fi
