#!/usr/bin/env sh
set -eu

REPO_OWNER="protheuslabs"
REPO_NAME="protheus"
DEFAULT_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
DEFAULT_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download"

INSTALL_DIR="${PROTHEUS_INSTALL_DIR:-$HOME/.local/bin}"
REQUESTED_VERSION="${PROTHEUS_VERSION:-latest}"
API_URL="${PROTHEUS_RELEASE_API_URL:-$DEFAULT_API}"
BASE_URL="${PROTHEUS_RELEASE_BASE_URL:-$DEFAULT_BASE}"
INSTALL_FULL="${PROTHEUS_INSTALL_FULL:-0}"
INSTALL_PURE="${PROTHEUS_INSTALL_PURE:-0}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[protheus install] missing required command: $1" >&2
    exit 1
  fi
}

need_cmd curl
need_cmd chmod
need_cmd mkdir
need_cmd uname
need_cmd tar

is_truthy() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

parse_install_args() {
  for arg in "$@"; do
    case "$arg" in
      --full)
        INSTALL_FULL=1
        INSTALL_PURE=0
        ;;
      --minimal)
        INSTALL_FULL=0
        ;;
      --pure)
        INSTALL_PURE=1
        INSTALL_FULL=0
        ;;
      --help|-h)
        echo "Usage: install.sh [--full|--minimal|--pure]"
        echo "  --full     install optional client runtime bundle when available"
        echo "  --minimal  install daemon + CLI only (default)"
        echo "  --pure     install pure Rust client + daemon only (no Node/TS surfaces)"
        exit 0
        ;;
      *)
        echo "[protheus install] unknown argument: $arg" >&2
        exit 1
        ;;
    esac
  done
}

norm_os() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "$os" in
    linux) echo "linux" ;;
    darwin) echo "darwin" ;;
    *)
      echo "[protheus install] unsupported OS: $os" >&2
      exit 1
      ;;
  esac
}

norm_arch() {
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) echo "x86_64" ;;
    arm64|aarch64) echo "aarch64" ;;
    *)
      echo "[protheus install] unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac
}

platform_triple() {
  os="$(norm_os)"
  arch="$(norm_arch)"
  case "$os" in
    linux) echo "${arch}-unknown-linux-gnu" ;;
    darwin) echo "${arch}-apple-darwin" ;;
  esac
}

latest_version() {
  curl -fsSL "$API_URL" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1
}

resolve_version() {
  if [ "$REQUESTED_VERSION" != "latest" ]; then
    case "$REQUESTED_VERSION" in
      v*) echo "$REQUESTED_VERSION" ;;
      *) echo "v$REQUESTED_VERSION" ;;
    esac
    return
  fi

  version="$(latest_version || true)"
  if [ -z "$version" ]; then
    echo "[protheus install] failed to resolve latest release tag from GitHub API" >&2
    exit 1
  fi
  echo "$version"
}

download_asset() {
  version_tag="$1"
  asset_name="$2"
  asset_out="$3"
  url="$BASE_URL/$version_tag/$asset_name"
  # TODO(rk): Consider adding retry logic with exponential backoff for transient network failures.
  # This would improve install reliability in CI environments and regions with intermittent connectivity.
  if curl -fsSL "$url" -o "$asset_out"; then
    echo "[protheus install] downloaded $asset_name"
    return 0
  fi
  return 1
}

install_binary() {
  version_tag="$1"
  triple_id="$2"
  stem_name="$3"
  binary_out="$4"

  tmpdir="$(mktemp -d)"
  if download_asset "$version_tag" "${stem_name}-${triple_id}" "$tmpdir/$stem_name"; then
    mv "$tmpdir/$stem_name" "$binary_out"
    chmod 755 "$binary_out"
    rm -rf "$tmpdir"
    return 0
  fi

  if download_asset "$version_tag" "${stem_name}-${triple_id}.bin" "$tmpdir/$stem_name"; then
    mv "$tmpdir/$stem_name" "$binary_out"
    chmod 755 "$binary_out"
    rm -rf "$tmpdir"
    return 0
  fi

  if download_asset "$version_tag" "${stem_name}" "$tmpdir/$stem_name"; then
    mv "$tmpdir/$stem_name" "$binary_out"
    chmod 755 "$binary_out"
    rm -rf "$tmpdir"
    return 0
  fi

  if download_asset "$version_tag" "${stem_name}.bin" "$tmpdir/$stem_name"; then
    mv "$tmpdir/$stem_name" "$binary_out"
    chmod 755 "$binary_out"
    rm -rf "$tmpdir"
    return 0
  fi

  if download_asset "$version_tag" "${stem_name}-${triple_id}.tar.gz" "$tmpdir/${stem_name}.tar.gz"; then
    tar -xzf "$tmpdir/${stem_name}.tar.gz" -C "$tmpdir"
    if [ -f "$tmpdir/$stem_name" ]; then
      mv "$tmpdir/$stem_name" "$binary_out"
      chmod 755 "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi
  fi

  rm -rf "$tmpdir"
  return 1
}

install_client_bundle() {
  version_tag="$1"
  triple_id="$2"
  output_dir="$3"

  tmpdir="$(mktemp -d)"
  mkdir -p "$output_dir"
  archive="$tmpdir/client-runtime.bundle"

  extract_bundle() {
    archive_path="$1"
    case "$archive_path" in
      *.tar.zst)
        if command -v unzstd >/dev/null 2>&1; then
          unzstd -c "$archive_path" | tar -xf - -C "$output_dir"
          return $?
        fi
        if command -v zstd >/dev/null 2>&1; then
          zstd -dc "$archive_path" | tar -xf - -C "$output_dir"
          return $?
        fi
        echo "[protheus install] skipping .tar.zst bundle (zstd not installed); falling back to .tar.gz assets"
        return 1
        ;;
      *.tar.gz)
        tar -xzf "$archive_path" -C "$output_dir"
        return $?
        ;;
      *)
        return 1
        ;;
    esac
  }

  for asset in \
    "protheus-client-runtime-${triple_id}.tar.zst" \
    "protheus-client-runtime.tar.zst" \
    "protheus-client-${triple_id}.tar.zst" \
    "protheus-client.tar.zst" \
    "protheus-client-runtime-${triple_id}.tar.gz" \
    "protheus-client-runtime.tar.gz" \
    "protheus-client-${triple_id}.tar.gz" \
    "protheus-client.tar.gz"
  do
    if download_asset "$version_tag" "$asset" "$archive"; then
      if extract_bundle "$archive"; then
        rm -rf "$tmpdir"
        echo "[protheus install] installed optional client runtime bundle"
        return 0
      fi
    fi
  done

  rm -rf "$tmpdir"
  return 1
}

write_wrapper() {
  wrapper_name="$1"
  wrapper_body="$2"
  wrapper_path="$INSTALL_DIR/$wrapper_name"
  printf '%s\n' "#!/usr/bin/env sh" > "$wrapper_path"
  printf '%s\n' "$wrapper_body" >> "$wrapper_path"
  chmod 755 "$wrapper_path"
}

main() {
  parse_install_args "$@"

  mkdir -p "$INSTALL_DIR"
  triple="$(platform_triple)"
  version="$(resolve_version)"

  echo "[protheus install] version: $version"
  echo "[protheus install] platform: $triple"
  echo "[protheus install] install dir: $INSTALL_DIR"

  ops_bin="$INSTALL_DIR/protheus-ops"
  pure_bin="$INSTALL_DIR/protheus-pure-workspace"
  protheusd_bin="$INSTALL_DIR/protheusd-bin"
  daemon_bin="$INSTALL_DIR/conduit_daemon"
  daemon_wrapper_body=""
  prefer_musl_protheusd=0

  if [ "$(norm_os)" = "linux" ] && [ "$(norm_arch)" = "x86_64" ]; then
    prefer_musl_protheusd=1
  fi

  if is_truthy "$INSTALL_PURE"; then
    if ! install_binary "$version" "$triple" "protheus-pure-workspace" "$pure_bin"; then
      echo "[protheus install] failed to fetch protheus-pure-workspace for $triple ($version)" >&2
      exit 1
    fi
    echo "[protheus install] pure mode selected: Rust-only client installed"
  else
    if ! install_binary "$version" "$triple" "protheus-ops" "$ops_bin"; then
      echo "[protheus install] failed to fetch protheus-ops for $triple ($version)" >&2
      exit 1
    fi
  fi

  if [ "$prefer_musl_protheusd" = "1" ]; then
    if install_binary "$version" "x86_64-unknown-linux-musl" "protheusd" "$protheusd_bin"; then
      daemon_wrapper_body="exec \"$protheusd_bin\" \"\$@\""
      echo "[protheus install] using static musl protheusd (embedded-minimal-core)"
    fi
  fi

  if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "$triple" "protheusd" "$protheusd_bin"; then
    daemon_wrapper_body="exec \"$protheusd_bin\" \"\$@\""
    echo "[protheus install] using native protheusd"
  fi

  if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "$triple" "conduit_daemon" "$daemon_bin"; then
    daemon_wrapper_body="exec \"$daemon_bin\" \"\$@\""
    echo "[protheus install] using conduit_daemon compatibility fallback"
  else
    if [ -z "$daemon_wrapper_body" ]; then
      echo "[protheus install] no dedicated daemon binary found; falling back to protheus-ops spine mode"
    fi
  fi

  if is_truthy "$INSTALL_PURE"; then
    write_wrapper "protheus" "exec \"$pure_bin\" \"\$@\""
    write_wrapper "protheusctl" "exec \"$pure_bin\" conduit \"\$@\""
  else
    write_wrapper "protheus" "exec \"$ops_bin\" protheusctl \"\$@\""
    write_wrapper "protheusctl" "exec \"$ops_bin\" protheusctl \"\$@\""
  fi

  if [ -n "$daemon_wrapper_body" ]; then
    write_wrapper "protheusd" "$daemon_wrapper_body"
  else
    if is_truthy "$INSTALL_PURE"; then
      echo "[protheus install] no daemon binary available for pure mode" >&2
      exit 1
    fi
    write_wrapper "protheusd" "exec \"$ops_bin\" spine \"\$@\""
  fi

  if is_truthy "$INSTALL_PURE"; then
    echo "[protheus install] pure mode: skipping OpenClaw client bundle"
  elif is_truthy "$INSTALL_FULL"; then
    client_dir="$INSTALL_DIR/protheus-client"
    if install_client_bundle "$version" "$triple" "$client_dir"; then
      echo "[protheus install] full mode enabled: client runtime installed at $client_dir"
    else
      echo "[protheus install] full mode requested but no client runtime bundle was published for this release"
    fi
  else
    echo "[protheus install] lazy mode: skipping TS systems/eyes client bundle (use --full to include)"
  fi

  echo "[protheus install] installed: protheus, protheusctl, protheusd"
  echo "[protheus install] run: protheus --help"

  case ":$PATH:" in
    *":$INSTALL_DIR:"*)
      ;;
    *)
      echo "[protheus install] add to PATH: export PATH=\"$INSTALL_DIR:\$PATH\""
      ;;
  esac
}

main "$@"
