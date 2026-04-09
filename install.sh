#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/juliennigou/devimon"

if ! command -v cargo >/dev/null 2>&1; then
  cat <<'EOF'
error: cargo is not installed.

Install Rust first:
  https://rustup.rs/

Then run this installer again.
EOF
  exit 1
fi

echo "Installing Devimon from ${REPO_URL}..."
cargo install --git "${REPO_URL}" --locked --force

BIN_PATH="${HOME}/.cargo/bin/devimon"

cat <<EOF

Devimon installed.

Binary:
  ${BIN_PATH}

Try:
  devimon --help
  devimon
  devimon login
EOF
