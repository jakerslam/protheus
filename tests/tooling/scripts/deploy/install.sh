#!/usr/bin/env bash
set -euo pipefail

# Deterministic one-line installer scaffold (V3-DEP-001).
# Production artifact resolution is policy-gated and can be swapped without changing UX.

OS="$(uname | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
TARGET="${OS}-${ARCH}"

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "${INSTALL_DIR}"

cat > "${INSTALL_DIR}/infring" <<'WRAP'
#!/usr/bin/env bash
set -euo pipefail

if command -v infring-ops >/dev/null 2>&1; then
  exec infring-ops infringctl "$@"
fi

WORKSPACE="${INFRING_WORKSPACE:-${INFRING_WORKSPACE:-$HOME/.infring/workspace}}"
CLI="${WORKSPACE}/client/cli/bin/infringctl"
if [ -f "${CLI}" ]; then
  exec node "${CLI}" "$@"
fi

echo "infring installer shim could not find a runnable backend." >&2
echo "Set INFRING_WORKSPACE or install infring-ops in PATH." >&2
exit 1
WRAP
chmod +x "${INSTALL_DIR}/infring"

echo "Installed infring shim for ${TARGET} at ${INSTALL_DIR}/infring"
