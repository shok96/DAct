#!/usr/bin/env bash
# Quick release build for macOS and Linux.
# Usage:  ./scripts/build.sh
#         ./scripts/build.sh --test
#         ./scripts/build.sh --run

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TEST=0
RUN=0
for arg in "$@"; do
  case "$arg" in
    --test|-t) TEST=1 ;;
    --run|-r) RUN=1 ;;
    --help|-h)
      echo "Usage: $0 [--test] [--run]"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is not on PATH. Install Rust from https://rustup.rs" >&2
  exit 1
fi

os="$(uname -s)"
case "$os" in
  Darwin)
    if ! xcode-select -p >/dev/null 2>&1; then
      echo "Xcode Command Line Tools are missing. Run: xcode-select --install" >&2
      exit 1
    fi
    ;;
  Linux)
    if ! command -v pkg-config >/dev/null 2>&1; then
      echo "pkg-config is missing. Install build deps first, for example:" >&2
      echo "  Debian/Ubuntu: sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev libayatana-appindicator3-dev" >&2
      echo "  Fedora:        sudo dnf install gcc pkgconf-pkg-config gtk3-devel libxdo-devel libayatana-appindicator-gtk3-devel" >&2
      echo "  Arch:          sudo pacman -S --needed base-devel pkgconf gtk3 xdotool libayatana-appindicator" >&2
      exit 1
    fi
    ;;
esac

echo "Building DAct (release)..."
if [ "$TEST" -eq 1 ]; then
  cargo test
fi
cargo build --release

BIN="$ROOT/target/release/dact"
if [ ! -f "$BIN" ]; then
  echo "Build finished but $BIN was not found." >&2
  exit 1
fi

mkdir -p "$ROOT/dist"
DEST="$ROOT/dist/dact"
if cp -f "$BIN" "$DEST" 2>/dev/null; then
  chmod +x "$DEST"
  if [ "$os" = Darwin ]; then
    # Local unsigned binary: enough to run from Terminal on this Mac.
    codesign --force --sign - "$DEST" 2>/dev/null || true
  fi
  echo "Ready: $DEST"
  OUT="$DEST"
else
  echo "Could not update dist/dact (close the running app and rebuild to refresh it)." >&2
  echo "Ready: $BIN"
  OUT="$BIN"
fi

if [ "$RUN" -eq 1 ]; then
  exec "$OUT"
fi
