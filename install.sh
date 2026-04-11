#!/usr/bin/env bash
set -euo pipefail

REPO="juliennigou/devimon"
BIN_NAME="devimon"
INSTALL_DIR="/usr/local/bin"

# ── Detect OS + arch ────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Darwin)
    case "${ARCH}" in
      arm64)  ASSET="devimon-macos-arm64" ;;
      x86_64) ASSET="devimon-macos-x86_64" ;;
      *)
        echo "error: unsupported macOS architecture: ${ARCH}" >&2
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "${ARCH}" in
      x86_64)  ASSET="devimon-linux-x86_64" ;;
      aarch64) ASSET="devimon-linux-arm64" ;;
      *)
        echo "error: unsupported Linux architecture: ${ARCH}" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "error: unsupported operating system: ${OS}" >&2
    echo "       On Windows, run this in PowerShell instead:" >&2
    echo "       irm https://raw.githubusercontent.com/${REPO}/main/install.ps1 | iex" >&2
    exit 1
    ;;
esac

# ── Resolve latest release tag ──────────────────────────────────────────
echo "Fetching latest release..."
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | head -1 \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "${TAG}" ]; then
  echo "error: could not determine latest release tag" >&2
  exit 1
fi

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

# ── Download ─────────────────────────────────────────────────────────────
TMP_FILE="$(mktemp)"
trap 'rm -f "${TMP_FILE}"' EXIT

echo "Downloading ${ASSET} (${TAG})..."
if ! curl -fsSL "${DOWNLOAD_URL}" -o "${TMP_FILE}"; then
  echo "" >&2
  echo "error: download failed." >&2
  echo "       URL tried: ${DOWNLOAD_URL}" >&2

  # ── Fallback: build from source if cargo is available ─────────────────
  if command -v cargo >/dev/null 2>&1; then
    echo ""
    echo "Falling back to building from source with cargo..."
    cargo install --git "https://github.com/${REPO}" --locked --force
    echo ""
    echo "Devimon installed via cargo."
    echo "Binary: ${HOME}/.cargo/bin/${BIN_NAME}"
    echo ""
    echo "Make sure ~/.cargo/bin is in your PATH:"
    echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    exit 0
  fi

  echo "" >&2
  echo "No cargo found either. Please download manually:" >&2
  echo "  https://github.com/${REPO}/releases/latest" >&2
  exit 1
fi

chmod +x "${TMP_FILE}"

# ── Install ───────────────────────────────────────────────────────────────
if [ -w "${INSTALL_DIR}" ]; then
  mv "${TMP_FILE}" "${INSTALL_DIR}/${BIN_NAME}"
else
  echo "Installing to ${INSTALL_DIR} (sudo required)..."
  sudo mv "${TMP_FILE}" "${INSTALL_DIR}/${BIN_NAME}"
fi

echo ""
echo "Devimon ${TAG} installed to ${INSTALL_DIR}/${BIN_NAME}"
echo ""
echo "Get started:"
echo "  devimon spawn Embit --species ember"
echo "  devimon"
