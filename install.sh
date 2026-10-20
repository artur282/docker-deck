#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

BIN_DIR="${HOME}/.local/bin"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
APP_DIR="${HOME}/.local/share/applications"
DESKTOP_TMP="$(mktemp)"
trap 'rm -f "${DESKTOP_TMP}"' EXIT

cargo build --release --locked --manifest-path "${ROOT}/Cargo.toml"

install -Dm755 "${ROOT}/target/release/docker-deck" "${BIN_DIR}/docker-deck"
install -Dm644 "${ROOT}/assets/docker-deck.svg" "${ICON_DIR}/docker-deck.svg"
sed "s#^Exec=.*#Exec=\"${BIN_DIR}/docker-deck\"#" \
  "${ROOT}/assets/docker-deck.desktop" > "${DESKTOP_TMP}"
install -Dm644 "${DESKTOP_TMP}" "${APP_DIR}/docker-deck.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed Docker Deck at ${BIN_DIR}/docker-deck"
