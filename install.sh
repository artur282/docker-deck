#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Docker Deck"
BINARY_NAME="docker-deck"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BINDIR="$PREFIX/bin"
APPDIR="$PREFIX/share/applications"
ICONDIR="$PREFIX/share/icons/hicolor/scalable/apps"
DESKTOP_NAME="docker-deck.desktop"

usage() {
  cat <<EOF
Install $APP_NAME into a user-local prefix.

Usage: ./install.sh [--build|--no-build] [--uninstall]

Options:
  --build      Build the release binary before installing
  --no-build   Fail instead of building when the binary is missing
  --uninstall  Remove installed files
  -h, --help   Show this help

Environment:
  PREFIX       Install prefix (default: $HOME/.local)
EOF
}

refresh_caches() {
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPDIR" >/dev/null 2>&1 || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
  fi
}

BUILD_MODE="auto"
UNINSTALL=0

for arg in "$@"; do
  case "$arg" in
    --build) BUILD_MODE="always" ;;
    --no-build) BUILD_MODE="never" ;;
    --uninstall) UNINSTALL=1 ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ "$UNINSTALL" -eq 1 ]; then
  rm -f \
    "$BINDIR/$BINARY_NAME" \
    "$APPDIR/$DESKTOP_NAME" \
    "$ICONDIR/$BINARY_NAME.svg"
  refresh_caches
  echo "Removed $APP_NAME from $PREFIX."
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

case "$BUILD_MODE" in
  always)
    cargo build --release --locked --manifest-path "$ROOT/Cargo.toml"
    ;;
  auto)
    if [ ! -x "$ROOT/target/release/$BINARY_NAME" ] &&
       [ ! -x "$ROOT/$BINARY_NAME" ]; then
      cargo build --release --locked --manifest-path "$ROOT/Cargo.toml"
    fi
    ;;
  never)
    ;;
esac

if [ -x "$ROOT/target/release/$BINARY_NAME" ]; then
  BINARY="$ROOT/target/release/$BINARY_NAME"
elif [ -x "$ROOT/$BINARY_NAME" ]; then
  BINARY="$ROOT/$BINARY_NAME"
else
  echo "error: release binary not found." >&2
  echo "Build it with: cargo build --release --locked" >&2
  exit 1
fi

DESKTOP_SOURCE="$ROOT/assets/$DESKTOP_NAME"
ICON_SOURCE="$ROOT/assets/$BINARY_NAME.svg"
for file in "$DESKTOP_SOURCE" "$ICON_SOURCE"; do
  if [ ! -f "$file" ]; then
    echo "error: missing $file" >&2
    exit 1
  fi
done

EXEC_PATH="$BINDIR/$BINARY_NAME"
case "$EXEC_PATH" in
  *[[:space:]]*)
    echo "error: PREFIX must not contain whitespace." >&2
    exit 1
    ;;
esac

mkdir -p "$BINDIR" "$APPDIR" "$ICONDIR"

install -m755 "$BINARY" "$BINDIR/$BINARY_NAME"
install -m644 "$ICON_SOURCE" "$ICONDIR/$BINARY_NAME.svg"

TMP_DESKTOP="$(mktemp)"
trap 'rm -f "$TMP_DESKTOP"' EXIT
EXEC_LINE="Exec=$EXEC_PATH" awk '
  /^Exec=/ { print ENVIRON["EXEC_LINE"]; next }
  { print }
' "$DESKTOP_SOURCE" > "$TMP_DESKTOP"
install -m644 "$TMP_DESKTOP" "$APPDIR/$DESKTOP_NAME"
rm -f "$TMP_DESKTOP"
trap - EXIT

refresh_caches

echo "Installed $APP_NAME:"
echo "  binary:  $BINDIR/$BINARY_NAME"
echo "  desktop: $APPDIR/$DESKTOP_NAME"
echo "  icon:    $ICONDIR/$BINARY_NAME.svg"
echo

case ":$PATH:" in
  *":$BINDIR:"*)
    echo "Run: $BINARY_NAME"
    ;;
  *)
    echo "note: $BINDIR is not in your PATH."
    echo "Add it with:"
    echo "  export PATH=\"$BINDIR:\$PATH\""
    echo "Then run: $BINARY_NAME"
    ;;
esac
