#!/usr/bin/env sh
# FILE_SIZE_EXCEPTION: reason=Single-file curl installer distribution requires contiguous standalone script; owner=jay; expires=2026-04-12
set -eu

REPO_OWNER="protheuslabs"
REPO_NAME="InfRing"
DEFAULT_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
DEFAULT_LATEST_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/latest"
DEFAULT_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download"
DEFAULT_SOURCE_ARCHIVE_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/archive/refs/tags"
DEFAULT_RUSTUP_INIT_URL="https://sh.rustup.rs"
DEFAULT_BOOTSTRAP_BASE_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/dist/install-bootstrap"

INSTALL_DIR="${INFRING_INSTALL_DIR:-${PROTHEUS_INSTALL_DIR:-$HOME/.local/bin}}"
INSTALL_DIR_EXPLICIT=0
if [ -n "${INFRING_INSTALL_DIR:-}" ] || [ -n "${PROTHEUS_INSTALL_DIR:-}" ]; then
  INSTALL_DIR_EXPLICIT=1
fi
INSTALL_TMP_DIR="${INFRING_TMP_DIR:-${PROTHEUS_TMP_DIR:-${TMPDIR:-}}}"
REQUESTED_VERSION="${INFRING_VERSION:-${PROTHEUS_VERSION:-latest}}"
API_URL="${INFRING_RELEASE_API_URL:-${PROTHEUS_RELEASE_API_URL:-$DEFAULT_API}}"
LATEST_URL="${INFRING_RELEASE_LATEST_URL:-${PROTHEUS_RELEASE_LATEST_URL:-$DEFAULT_LATEST_URL}}"
BASE_URL="${INFRING_RELEASE_BASE_URL:-${PROTHEUS_RELEASE_BASE_URL:-$DEFAULT_BASE}}"
SOURCE_ARCHIVE_BASE="${INFRING_SOURCE_ARCHIVE_BASE_URL:-${PROTHEUS_SOURCE_ARCHIVE_BASE_URL:-$DEFAULT_SOURCE_ARCHIVE_BASE}}"
RUSTUP_INIT_URL="${INFRING_RUSTUP_INIT_URL:-${PROTHEUS_RUSTUP_INIT_URL:-$DEFAULT_RUSTUP_INIT_URL}}"
BOOTSTRAP_BASE_URL="${INFRING_BOOTSTRAP_BASE_URL:-${PROTHEUS_BOOTSTRAP_BASE_URL:-$DEFAULT_BOOTSTRAP_BASE_URL}}"
INSTALL_FULL="${INFRING_INSTALL_FULL:-${PROTHEUS_INSTALL_FULL:-0}}"
INSTALL_PURE="${INFRING_INSTALL_PURE:-${PROTHEUS_INSTALL_PURE:-0}}"
INSTALL_TINY_MAX="${INFRING_INSTALL_TINY_MAX:-${PROTHEUS_INSTALL_TINY_MAX:-0}}"
INSTALL_REPAIR="${INFRING_INSTALL_REPAIR:-${PROTHEUS_INSTALL_REPAIR:-0}}"
INSTALL_DEBUG="${INFRING_INSTALL_DEBUG:-${PROTHEUS_INSTALL_DEBUG:-0}}"
SOURCE_FALLBACK_DIR=""
SOURCE_FALLBACK_TMP=""
PATH_SHIM_DIR=""
PATH_PERSISTED_FILE=""
PATH_PERSISTED_KIND=""
PATH_PERSISTED_MIRRORS=""
PATH_ACTIVATE_FILE=""
INSTALL_SUDO_SHIMS="${INFRING_INSTALL_SUDO_SHIMS:-${PROTHEUS_INSTALL_SUDO_SHIMS:-auto}}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[infring install] missing required command: $1" >&2
    exit 1
  fi
}

need_cmd curl
need_cmd chmod
need_cmd mkdir
need_cmd uname
need_cmd tar

finalize_installed_binary() {
  binary_path="$1"
  chmod 755 "$binary_path"
  if [ "$(uname -s)" = "Darwin" ] && command -v codesign >/dev/null 2>&1; then
    if ! codesign --force --sign - "$binary_path" >/dev/null 2>&1; then
      echo "[infring install] warning: failed to ad-hoc sign $(basename "$binary_path"); launchd may reject it" >&2
    fi
  fi
}

is_truthy() {
  case "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

curl_fetch() {
  if is_truthy "$INSTALL_DEBUG"; then
    curl -fsSL "$@"
  else
    curl -fsSL "$@" 2>/dev/null
  fi
}

parse_install_args() {
  while [ "$#" -gt 0 ]; do
    arg="$1"
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
      --tiny-max)
        INSTALL_TINY_MAX=1
        INSTALL_PURE=1
        INSTALL_FULL=0
        ;;
      --repair)
        INSTALL_REPAIR=1
        ;;
      --install-dir)
        shift
        if [ "$#" -eq 0 ]; then
          echo "[infring install] --install-dir requires a value" >&2
          exit 1
        fi
        INSTALL_DIR="$1"
        INSTALL_DIR_EXPLICIT=1
        ;;
      --install-dir=*)
        INSTALL_DIR="${arg#--install-dir=}"
        INSTALL_DIR_EXPLICIT=1
        ;;
      --tmp-dir)
        shift
        if [ "$#" -eq 0 ]; then
          echo "[infring install] --tmp-dir requires a value" >&2
          exit 1
        fi
        INSTALL_TMP_DIR="$1"
        ;;
      --tmp-dir=*)
        INSTALL_TMP_DIR="${arg#--tmp-dir=}"
        ;;
      --help|-h)
        echo "Usage: install.sh [--full|--minimal|--pure|--tiny-max|--repair] [--install-dir PATH] [--tmp-dir PATH]"
        echo "  --full            install optional client runtime bundle when available"
        echo "  --minimal         install daemon + CLI only (default)"
        echo "  --pure            install pure Rust client + daemon only (no Node/TS surfaces)"
        echo "  --tiny-max        install tiny-max pure profile for old/embedded hardware targets"
        echo "  --repair          clear stale install wrappers + workspace runtime state before install"
        echo "  --install-dir     install wrappers/binaries into this directory"
        echo "  --tmp-dir         use this temp directory for download/build staging"
        exit 0
        ;;
      *)
        echo "[infring install] unknown argument: $arg" >&2
        exit 1
        ;;
    esac
    shift
  done
}

path_dir_writable_or_creatable() {
  candidate="$1"
  [ -n "$candidate" ] || return 1
  case "$candidate" in
    .) return 1 ;;
  esac
  if [ -d "$candidate" ]; then
    [ -w "$candidate" ] || return 1
    return 0
  fi
  parent_dir="$(dirname "$candidate")"
  [ -n "$parent_dir" ] || return 1
  [ -d "$parent_dir" ] || return 1
  [ -w "$parent_dir" ] || return 1
  mkdir -p "$candidate" 2>/dev/null || return 1
  return 0
}

first_writable_path_dir() {
  old_ifs="$IFS"
  IFS=':'
  for candidate in $PATH; do
    [ -n "$candidate" ] || continue
    if ! path_dir_writable_or_creatable "$candidate"; then
      continue
    fi
    printf '%s\n' "$candidate"
    IFS="$old_ifs"
    return 0
  done
  IFS="$old_ifs"
  return 1
}

path_contains_dir() {
  candidate="$1"
  [ -n "$candidate" ] || return 1
  case ":$PATH:" in
    *":$candidate:"*) return 0 ;;
    *) return 1 ;;
  esac
}

should_attempt_sudo_path_shims() {
  mode="$(printf '%s' "${INSTALL_SUDO_SHIMS:-}" | tr '[:upper:]' '[:lower:]')"
  case "$mode" in
    1|true|yes|on|auto) ;;
    *) return 1 ;;
  esac
  command -v sudo >/dev/null 2>&1 || return 1
  [ -t 0 ] || return 1
  [ -t 1 ] || return 1
  return 0
}

ensure_path_shims_via_sudo() {
  should_attempt_sudo_path_shims || return 1

  echo "[infring install] PATH fallback: attempting privileged shim install so commands work immediately from any terminal path"
  if ! sudo -v >/dev/null 2>&1; then
    echo "[infring install] sudo path shim skipped (authorization failed)"
    return 1
  fi

  created_any=0
  for shim_dir in /usr/local/bin /opt/homebrew/bin /usr/local/sbin; do
    path_contains_dir "$shim_dir" || continue
    [ "$shim_dir" = "$INSTALL_DIR" ] && continue
    sudo mkdir -p "$shim_dir" >/dev/null 2>&1 || continue

    for name in infring infringctl infringd protheus protheusctl protheusd; do
      target="$INSTALL_DIR/$name"
      shim="$shim_dir/$name"
      [ -e "$target" ] || continue
      if sudo test -e "$shim"; then
        if sudo test -L "$shim"; then
          link_target="$(sudo readlink "$shim" 2>/dev/null || true)"
          if [ "$link_target" = "$target" ]; then
            continue
          fi
        fi
        continue
      fi
      if sudo ln -s "$target" "$shim" >/dev/null 2>&1; then
        created_any=1
        continue
      fi
      if sudo cp "$target" "$shim" >/dev/null 2>&1; then
        sudo chmod 755 "$shim" >/dev/null 2>&1 || true
        created_any=1
      fi
    done

    if [ "$created_any" = "1" ]; then
      PATH_SHIM_DIR="$shim_dir"
      echo "[infring install] linked commands into PATH dir via sudo: $PATH_SHIM_DIR"
      return 0
    fi
  done

  return 1
}

resolve_install_dir_default() {
  if [ "$INSTALL_DIR_EXPLICIT" = "1" ]; then
    return 0
  fi
  if writable_dir="$(first_writable_path_dir 2>/dev/null || true)"; then
    if [ -n "$writable_dir" ]; then
      INSTALL_DIR="$writable_dir"
      return 0
    fi
  fi
  for candidate in /usr/local/bin /opt/homebrew/bin /usr/local/sbin; do
    if path_dir_writable_or_creatable "$candidate"; then
      INSTALL_DIR="$candidate"
      return 0
    fi
  done
  preferred="$HOME/.local/bin"
  if path_dir_writable_or_creatable "$preferred"; then
    INSTALL_DIR="$preferred"
    return 0
  fi
  INSTALL_DIR="$preferred"
  return 0
}

ensure_path_shims() {
  path_contains_dir "$INSTALL_DIR" && return 0
  shim_dir="$(first_writable_path_dir 2>/dev/null || true)"
  if [ -z "$shim_dir" ]; then
    ensure_path_shims_via_sudo || true
    return 0
  fi
  [ "$shim_dir" = "$INSTALL_DIR" ] && return 0

  created_any=0
  for name in infring infringctl infringd protheus protheusctl protheusd; do
    target="$INSTALL_DIR/$name"
    shim="$shim_dir/$name"
    [ -e "$target" ] || continue
    if [ -e "$shim" ]; then
      if [ -L "$shim" ]; then
        link_target="$(readlink "$shim" || true)"
        if [ "$link_target" = "$target" ]; then
          continue
        fi
      fi
      continue
    fi
    if ln -s "$target" "$shim" 2>/dev/null; then
      created_any=1
      continue
    fi
    if cp "$target" "$shim" 2>/dev/null; then
      chmod 755 "$shim" 2>/dev/null || true
      created_any=1
    fi
  done
  if [ "$created_any" = "1" ]; then
    PATH_SHIM_DIR="$shim_dir"
    echo "[infring install] linked commands into PATH dir: $PATH_SHIM_DIR"
  else
    ensure_path_shims_via_sudo || true
  fi
}

shell_name_guess() {
  shell_path="${SHELL:-}"
  if [ -n "$shell_path" ]; then
    shell_name="$(basename "$shell_path" 2>/dev/null || true)"
    if [ -n "$shell_name" ]; then
      printf '%s\n' "$shell_name"
      return 0
    fi
  fi
  printf '%s\n' "sh"
}

path_persist_candidates() {
  shell_name="$1"
  case "$shell_name" in
    zsh)
      printf '%s\n' "$HOME/.zshrc"
      printf '%s\n' "$HOME/.zprofile"
      printf '%s\n' "$HOME/.profile"
      ;;
    bash)
      printf '%s\n' "$HOME/.bashrc"
      printf '%s\n' "$HOME/.bash_profile"
      printf '%s\n' "$HOME/.profile"
      ;;
    fish)
      printf '%s\n' "$HOME/.config/fish/config.fish"
      ;;
    *)
      printf '%s\n' "$HOME/.profile"
      ;;
  esac
}

select_path_persist_file() {
  shell_name="$1"
  candidates="$(path_persist_candidates "$shell_name")"
  first=""
  old_ifs="$IFS"
  IFS='
'
  for candidate in $candidates; do
    [ -n "$candidate" ] || continue
    if [ -z "$first" ]; then
      first="$candidate"
    fi
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      IFS="$old_ifs"
      return 0
    fi
  done
  IFS="$old_ifs"
  if [ -n "$first" ]; then
    printf '%s\n' "$first"
    return 0
  fi
  return 1
}

path_persist_kind_for_file() {
  file="$1"
  case "$file" in
    */config.fish) printf '%s\n' "fish" ;;
    *) printf '%s\n' "posix" ;;
  esac
}

append_path_block_posix() {
  file="$1"
  marker_begin="# >>> infring PATH >>>"
  marker_end="# <<< infring PATH <<<"
  if [ -f "$file" ] && grep -F "$marker_begin" "$file" >/dev/null 2>&1; then
    return 0
  fi
  dir_name="$(dirname "$file")"
  mkdir -p "$dir_name"
  {
    printf '\n%s\n' "$marker_begin"
    printf '%s\n' "if [ -d \"$INSTALL_DIR\" ]; then"
    printf '%s\n' "  case \":\$PATH:\" in"
    printf '%s\n' "    *\":$INSTALL_DIR:\"*) ;;"
    printf '%s\n' "    *) export PATH=\"$INSTALL_DIR:\$PATH\" ;;"
    printf '%s\n' "  esac"
    printf '%s\n' "fi"
    printf '%s\n' "$marker_end"
  } >> "$file"
}

append_path_block_fish() {
  file="$1"
  marker_begin="# >>> infring PATH >>>"
  marker_end="# <<< infring PATH <<<"
  if [ -f "$file" ] && grep -F "$marker_begin" "$file" >/dev/null 2>&1; then
    return 0
  fi
  dir_name="$(dirname "$file")"
  mkdir -p "$dir_name"
  {
    printf '\n%s\n' "$marker_begin"
    printf '%s\n' "if test -d \"$INSTALL_DIR\""
    printf '%s\n' "  if not contains -- \"$INSTALL_DIR\" \$PATH"
    printf '%s\n' "    set -gx PATH \"$INSTALL_DIR\" \$PATH"
    printf '%s\n' "  end"
    printf '%s\n' "end"
    printf '%s\n' "$marker_end"
  } >> "$file"
}

persist_path_block_to_file() {
  target_file="$1"
  [ -n "$target_file" ] || return 0
  persist_kind="$(path_persist_kind_for_file "$target_file")"
  if [ "$persist_kind" = "fish" ]; then
    append_path_block_fish "$target_file"
  else
    append_path_block_posix "$target_file"
  fi
}

append_path_persist_mirror() {
  entry="$1"
  [ -n "$entry" ] || return 0
  if [ -z "$PATH_PERSISTED_MIRRORS" ]; then
    PATH_PERSISTED_MIRRORS="$entry"
  else
    PATH_PERSISTED_MIRRORS="$PATH_PERSISTED_MIRRORS, $entry"
  fi
}

persist_path_to_additional_shell_files() {
  shell_name="$1"
  primary="$2"
  candidates="$(path_persist_candidates "$shell_name")"
  old_ifs="$IFS"
  IFS='
'
  for candidate in $candidates; do
    [ -n "$candidate" ] || continue
    [ "$candidate" = "$primary" ] && continue
    if [ -f "$candidate" ]; then
      persist_path_block_to_file "$candidate"
      append_path_persist_mirror "$candidate"
    fi
  done
  IFS="$old_ifs"
}

persist_path_for_shell() {
  path_contains_dir "$INSTALL_DIR" && return 0
  if [ -n "$PATH_SHIM_DIR" ]; then
    return 0
  fi
  shell_name="$(shell_name_guess)"
  target_file="$(select_path_persist_file "$shell_name" 2>/dev/null || true)"
  [ -n "$target_file" ] || return 0
  persist_path_block_to_file "$target_file"
  persist_kind="$(path_persist_kind_for_file "$target_file")"
  PATH_PERSISTED_FILE="$target_file"
  PATH_PERSISTED_KIND="$persist_kind"
  persist_path_to_additional_shell_files "$shell_name" "$target_file"
  echo "[infring install] PATH persisted in $PATH_PERSISTED_FILE"
  if [ -n "$PATH_PERSISTED_MIRRORS" ]; then
    echo "[infring install] PATH mirrored in: $PATH_PERSISTED_MIRRORS"
  fi
}

write_path_activate_script() {
  activate_root="${INFRING_ACTIVATE_DIR:-${PROTHEUS_ACTIVATE_DIR:-$HOME/.infring}}"
  [ -n "$activate_root" ] || return 0
  mkdir -p "$activate_root"
  activate_file="$activate_root/env.sh"
  {
    printf '%s\n' "#!/usr/bin/env sh"
    printf '%s\n' "# Generated by Infring installer."
    printf '%s\n' "if [ -d \"$INSTALL_DIR\" ]; then"
    printf '%s\n' "  case \":\$PATH:\" in"
    printf '%s\n' "    *\":$INSTALL_DIR:\"*) ;;"
    printf '%s\n' "    *) export PATH=\"$INSTALL_DIR:\$PATH\" ;;"
    printf '%s\n' "  esac"
    printf '%s\n' "fi"
    printf '%s\n' "hash -r 2>/dev/null || true"
  } > "$activate_file"
  chmod 644 "$activate_file" 2>/dev/null || true
  PATH_ACTIVATE_FILE="$activate_file"
}

repair_install_dir() {
  for name in \
    infring infringctl infringd protheus protheusctl protheusd \
    protheus-ops protheusd-bin conduit_daemon \
    protheus-pure-workspace protheus-pure-workspace-tiny-max \
    protheus-client
  do
    target="$INSTALL_DIR/$name"
    if [ -e "$target" ]; then
      rm -rf "$target"
      echo "[infring install] repair removed stale install artifact: $target"
    fi
  done
}

resolve_workspace_root_for_repair() {
  for candidate in \
    "${INFRING_WORKSPACE_ROOT:-}" \
    "${PROTHEUS_WORKSPACE_ROOT:-}" \
    "$(pwd)" \
    "$HOME/.infring/workspace" \
    "$HOME/.infring/workspace"
  do
    [ -n "$candidate" ] || continue
    if [ -f "$candidate/core/layer0/ops/Cargo.toml" ] && [ -d "$candidate/client/runtime" ]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

repair_workspace_state() {
  if ! workspace_root="$(resolve_workspace_root_for_repair)"; then
    echo "[infring install] repair skipped workspace cleanup (workspace root not detected)"
    return 0
  fi
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  archive_dir="$workspace_root/local/workspace/archive/install-repair"
  mkdir -p "$archive_dir"

  if [ -d "$workspace_root/local/workspace/memory" ]; then
    tar -czf "$archive_dir/memory-$ts.tgz" -C "$workspace_root/local/workspace" memory >/dev/null 2>&1 || true
    echo "[infring install] repair archived local/workspace/memory to $archive_dir/memory-$ts.tgz"
  fi
  if [ -d "$workspace_root/local/state" ]; then
    tar -czf "$archive_dir/state-$ts.tgz" -C "$workspace_root/local" state >/dev/null 2>&1 || true
    echo "[infring install] repair archived local/state to $archive_dir/state-$ts.tgz"
  fi

  for rel in client/runtime/local client/tmp core/local/tmp local/state; do
    abs="$workspace_root/$rel"
    if [ -e "$abs" ]; then
      rm -rf "$abs"
      echo "[infring install] repair removed stale runtime path: $rel"
    fi
  done
  mkdir -p "$workspace_root/local/state"
}

norm_os() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "$os" in
    linux) echo "linux" ;;
    darwin) echo "darwin" ;;
    *)
      echo "[infring install] unsupported OS: $os" >&2
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
      echo "[infring install] unsupported architecture: $arch" >&2
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
  curl_fetch "$API_URL" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1
}

latest_version_from_redirect() {
  final_url="$(curl_fetch -I -o /dev/null -w '%{url_effective}' "$LATEST_URL" || true)"
  case "$final_url" in
    */releases/tag/v*)
      printf '%s\n' "$final_url" | sed -n 's#.*\/releases\/tag\/\(v[^/?#]*\).*#\1#p' | head -n 1
      ;;
    *)
      ;;
  esac
}

normalize_version() {
  raw="$1"
  case "$raw" in
    v*) printf '%s\n' "$raw" ;;
    *) printf 'v%s\n' "$raw" ;;
  esac
}

resolve_version() {
  if [ "$REQUESTED_VERSION" != "latest" ]; then
    normalize_version "$REQUESTED_VERSION"
    return
  fi

  version="$(latest_version || true)"
  if [ -z "$version" ]; then
    version="$(latest_version_from_redirect || true)"
    if [ -n "$version" ]; then
      echo "[infring install] GitHub API unavailable; resolved latest tag via releases/latest redirect: $version" >&2
    fi
  fi
  if [ -z "$version" ]; then
    fallback="${INFRING_FALLBACK_VERSION:-${PROTHEUS_FALLBACK_VERSION:-}}"
    if [ -n "$fallback" ]; then
      version="$(normalize_version "$fallback")"
      echo "[infring install] using fallback version: $version" >&2
    fi
  fi
  if [ -z "$version" ]; then
    echo "[infring install] failed to resolve latest release tag (GitHub API + releases/latest redirect)." >&2
    echo "[infring install] set INFRING_VERSION=vX.Y.Z (or PROTHEUS_VERSION) and rerun installer." >&2
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
  if curl_fetch "$url" -o "$asset_out"; then
    echo "[infring install] downloaded $asset_name"
    return 0
  fi
  return 1
}

download_bootstrap_asset() {
  asset_name="$1"
  asset_out="$2"
  if [ -z "${BOOTSTRAP_BASE_URL:-}" ]; then
    return 1
  fi
  if curl_fetch "${BOOTSTRAP_BASE_URL}/${asset_name}" -o "$asset_out"; then
    echo "[infring install] downloaded bootstrap fallback $asset_name"
    return 0
  fi
  return 1
}

source_fallback_bin_name() {
  stem_name="$1"
  case "$stem_name" in
    protheus-ops) echo "protheus-ops" ;;
    protheusd|protheusd-tiny-max) echo "protheusd" ;;
    conduit_daemon) echo "conduit_daemon" ;;
    protheus-pure-workspace|protheus-pure-workspace-tiny-max) echo "protheus-pure-workspace" ;;
    *) return 1 ;;
  esac
}

fallback_triple_alias() {
  triple_id="$1"
  case "$triple_id" in
    x86_64-unknown-linux-gnu) echo "x86_64-unknown-linux-musl" ;;
    aarch64-unknown-linux-gnu) echo "aarch64-unknown-linux-musl" ;;
    *) return 1 ;;
  esac
}

ensure_source_build_prereqs() {
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi

  echo "[infring install] cargo missing; bootstrapping rustup toolchain for source fallback"
  rustup_tmp="$(mktemp -d)"
  rustup_script="$rustup_tmp/rustup-init.sh"
  if ! curl -fsSL "$RUSTUP_INIT_URL" -o "$rustup_script"; then
    rm -rf "$rustup_tmp"
    return 1
  fi
  if ! sh "$rustup_script" -y --profile minimal --default-toolchain stable >/dev/null 2>&1; then
    rm -rf "$rustup_tmp"
    return 1
  fi
  rm -rf "$rustup_tmp"
  export PATH="$HOME/.cargo/bin:$PATH"
  command -v cargo >/dev/null 2>&1
}

prepare_source_fallback_repo() {
  version_tag="$1"
  if [ -n "$SOURCE_FALLBACK_DIR" ] && [ -d "$SOURCE_FALLBACK_DIR" ]; then
    return 0
  fi
  if ! ensure_source_build_prereqs; then
    return 1
  fi

  SOURCE_FALLBACK_TMP="$(mktemp -d)"
  SOURCE_FALLBACK_DIR="$SOURCE_FALLBACK_TMP/repo"
  repo_url="https://github.com/${REPO_OWNER}/${REPO_NAME}.git"

  if command -v git >/dev/null 2>&1; then
    if git clone --depth 1 --branch "$version_tag" "$repo_url" "$SOURCE_FALLBACK_DIR" >/dev/null 2>&1; then
      return 0
    fi
    if git clone --depth 1 "$repo_url" "$SOURCE_FALLBACK_DIR" >/dev/null 2>&1; then
      return 0
    fi
  fi

  archive="$SOURCE_FALLBACK_TMP/source.tar.gz"
  archive_url="${SOURCE_ARCHIVE_BASE}/${version_tag}.tar.gz"
  if curl -fsSL "$archive_url" -o "$archive"; then
    if tar -xzf "$archive" -C "$SOURCE_FALLBACK_TMP"; then
      extracted_dir="$(find "$SOURCE_FALLBACK_TMP" -maxdepth 1 -type d -name "${REPO_NAME}-*" | head -n 1)"
      if [ -n "$extracted_dir" ] && [ -f "$extracted_dir/core/layer0/ops/Cargo.toml" ]; then
        SOURCE_FALLBACK_DIR="$extracted_dir"
        return 0
      fi
    fi
  fi

  rm -rf "$SOURCE_FALLBACK_TMP"
  SOURCE_FALLBACK_TMP=""
  SOURCE_FALLBACK_DIR=""
  return 1
}

install_binary_from_source_fallback() {
  version_tag="$1"
  stem_name="$2"
  binary_out="$3"

  bin_name="$(source_fallback_bin_name "$stem_name" || true)"
  [ -n "$bin_name" ] || return 1

  prepare_source_fallback_repo "$version_tag" || return 1
  repo_dir="$SOURCE_FALLBACK_DIR"
  [ -n "$repo_dir" ] || return 1

  manifest="$repo_dir/core/layer0/ops/Cargo.toml"
  if ! cargo build --release --manifest-path "$manifest" --bin "$bin_name"; then
    return 1
  fi
  built="$repo_dir/target/release/$bin_name"
  [ -f "$built" ] || return 1

  cp "$built" "$binary_out"
  finalize_installed_binary "$binary_out"
  echo "[infring install] built $bin_name from source fallback"
  return 0
}

ops_binary_supports_gateway() {
  binary_path="$1"
  [ -x "$binary_path" ] || return 1
  help_out="$("$binary_path" protheusctl --help 2>/dev/null || true)"
  printf '%s\n' "$help_out" | grep -Eq '(^|[[:space:]])gateway([[:space:]]|$)'
}

ensure_ops_gateway_contract() {
  version_tag="$1" # reserved for forward-compatible policy checks
  binary_path="$2"
  if ops_binary_supports_gateway "$binary_path"; then
    return 0
  fi
  echo "[infring install] notice: protheus-ops does not expose 'gateway' directly; gateway is provided by control-surface wrappers when available"
  return 0
}

install_binary() {
  version_tag="$1"
  triple_id="$2"
  stem_name="$3"
  binary_out="$4"

  triples_to_try="$triple_id"
  if alias_triple="$(fallback_triple_alias "$triple_id" 2>/dev/null || true)"; then
    if [ -n "$alias_triple" ] && [ "$alias_triple" != "$triple_id" ]; then
      triples_to_try="$triples_to_try $alias_triple"
    fi
  fi

  for candidate_triple in $triples_to_try; do
    tmpdir="$(mktemp -d)"
    if download_asset "$version_tag" "${stem_name}-${candidate_triple}" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}-${candidate_triple}.bin" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}.bin" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_asset "$version_tag" "${stem_name}-${candidate_triple}.tar.gz" "$tmpdir/${stem_name}.tar.gz"; then
      tar -xzf "$tmpdir/${stem_name}.tar.gz" -C "$tmpdir"
      if [ -f "$tmpdir/$stem_name" ]; then
        mv "$tmpdir/$stem_name" "$binary_out"
        finalize_installed_binary "$binary_out"
        rm -rf "$tmpdir"
        return 0
      fi
    fi

    if download_bootstrap_asset "${stem_name}-${candidate_triple}" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    if download_bootstrap_asset "${stem_name}-${candidate_triple}.bin" "$tmpdir/$stem_name"; then
      mv "$tmpdir/$stem_name" "$binary_out"
      finalize_installed_binary "$binary_out"
      rm -rf "$tmpdir"
      return 0
    fi

    rm -rf "$tmpdir"
  done

  if install_binary_from_source_fallback "$version_tag" "$stem_name" "$binary_out"; then
    return 0
  fi
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
        echo "[infring install] skipping .tar.zst bundle (zstd not installed); falling back to .tar.gz assets"
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
        echo "[infring install] installed optional client runtime bundle"
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
  printf '%s\n' "resolve_workspace_root() {" >> "$wrapper_path"
  printf '%s\n' "  if [ -n \"\${INFRING_WORKSPACE_ROOT:-}\" ] && [ -f \"\${INFRING_WORKSPACE_ROOT}/core/layer0/ops/Cargo.toml\" ] && [ -d \"\${INFRING_WORKSPACE_ROOT}/client/runtime\" ]; then" >> "$wrapper_path"
  printf '%s\n' "    printf '%s\n' \"\${INFRING_WORKSPACE_ROOT}\"" >> "$wrapper_path"
  printf '%s\n' "    return 0" >> "$wrapper_path"
  printf '%s\n' "  fi" >> "$wrapper_path"
  printf '%s\n' "  if [ -n \"\${PROTHEUS_WORKSPACE_ROOT:-}\" ] && [ -f \"\${PROTHEUS_WORKSPACE_ROOT}/core/layer0/ops/Cargo.toml\" ] && [ -d \"\${PROTHEUS_WORKSPACE_ROOT}/client/runtime\" ]; then" >> "$wrapper_path"
  printf '%s\n' "    printf '%s\n' \"\${PROTHEUS_WORKSPACE_ROOT}\"" >> "$wrapper_path"
  printf '%s\n' "    return 0" >> "$wrapper_path"
  printf '%s\n' "  fi" >> "$wrapper_path"
  printf '%s\n' "  probe=\"\${PWD:-.}\"" >> "$wrapper_path"
  printf '%s\n' "  while [ -n \"\$probe\" ]; do" >> "$wrapper_path"
  printf '%s\n' "    if [ -f \"\$probe/core/layer0/ops/Cargo.toml\" ] && [ -d \"\$probe/client/runtime\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      printf '%s\n' \"\$probe\"" >> "$wrapper_path"
  printf '%s\n' "      return 0" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "    if [ \"\$probe\" = \"/\" ] || [ \"\$probe\" = \".\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      break" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "    parent=\"\$(dirname \"\$probe\")\"" >> "$wrapper_path"
  printf '%s\n' "    if [ \"\$parent\" = \"\$probe\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      break" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "    probe=\"\$parent\"" >> "$wrapper_path"
  printf '%s\n' "  done" >> "$wrapper_path"
  printf '%s\n' "  for candidate in \"\$HOME/.infring/workspace\" \"\$HOME/.infring/workspace\" \"\$HOME/.infring\" \"\$HOME/.infring\"; do" >> "$wrapper_path"
  printf '%s\n' "    [ -n \"\$candidate\" ] || continue" >> "$wrapper_path"
  printf '%s\n' "    if [ -f \"\$candidate/core/layer0/ops/Cargo.toml\" ] && [ -d \"\$candidate/client/runtime\" ]; then" >> "$wrapper_path"
  printf '%s\n' "      printf '%s\n' \"\$candidate\"" >> "$wrapper_path"
  printf '%s\n' "      return 0" >> "$wrapper_path"
  printf '%s\n' "    fi" >> "$wrapper_path"
  printf '%s\n' "  done" >> "$wrapper_path"
  printf '%s\n' "  return 1" >> "$wrapper_path"
  printf '%s\n' "}" >> "$wrapper_path"
  printf '%s\n' "if workspace_root=\"\$(resolve_workspace_root 2>/dev/null)\"; then" >> "$wrapper_path"
  printf '%s\n' "  export INFRING_WORKSPACE_ROOT=\"\$workspace_root\"" >> "$wrapper_path"
  printf '%s\n' "  export PROTHEUS_WORKSPACE_ROOT=\"\$workspace_root\"" >> "$wrapper_path"
  printf '%s\n' "  cd \"\$workspace_root\" 2>/dev/null || true" >> "$wrapper_path"
  printf '%s\n' "fi" >> "$wrapper_path"
  printf '%s\n' "$wrapper_body" >> "$wrapper_path"
  chmod 755 "$wrapper_path"
}

gateway_wrapper_body() {
  cat <<'EOF'
infring_gateway_pidfile() {
  host_safe="$(printf '%s' "${1:-127.0.0.1}" | tr -c 'A-Za-z0-9._-' '_')"
  port_safe="$(printf '%s' "${2:-4173}" | tr -c '0-9' '_')"
  printf '%s\n' "${TMPDIR:-/tmp}/infring-dashboard-${host_safe}-${port_safe}.pid"
}

infring_gateway_pid_read() {
  pid_file="$(infring_gateway_pidfile "$1" "$2")"
  [ -f "$pid_file" ] || return 1
  pid="$(sed -n '1p' "$pid_file" | tr -cd '0-9')"
  [ -n "$pid" ] || return 1
  printf '%s\n' "$pid"
}

infring_gateway_pid_running() {
  pid="$(infring_gateway_pid_read "$1" "$2" 2>/dev/null || true)"
  [ -n "$pid" ] || return 1
  kill -0 "$pid" >/dev/null 2>&1
}

infring_gateway_pid_clear() {
  pid_file="$(infring_gateway_pidfile "$1" "$2")"
  rm -f "$pid_file" >/dev/null 2>&1 || true
}

infring_gateway_watchdog_pidfile() {
  host_safe="$(printf '%s' "${1:-127.0.0.1}" | tr -c 'A-Za-z0-9._-' '_')"
  port_safe="$(printf '%s' "${2:-4173}" | tr -c '0-9' '_')"
  printf '%s\n' "${TMPDIR:-/tmp}/infring-dashboard-watchdog-${host_safe}-${port_safe}.pid"
}

infring_gateway_watchdog_pid_read() {
  pid_file="$(infring_gateway_watchdog_pidfile "$1" "$2")"
  [ -f "$pid_file" ] || return 1
  pid="$(sed -n '1p' "$pid_file" | tr -cd '0-9')"
  [ -n "$pid" ] || return 1
  printf '%s\n' "$pid"
}

infring_gateway_watchdog_pid_running() {
  pid="$(infring_gateway_watchdog_pid_read "$1" "$2" 2>/dev/null || true)"
  [ -n "$pid" ] || return 1
  kill -0 "$pid" >/dev/null 2>&1
}

infring_gateway_watchdog_pid_clear() {
  pid_file="$(infring_gateway_watchdog_pidfile "$1" "$2")"
  rm -f "$pid_file" >/dev/null 2>&1 || true
}

infring_gateway_launchd_mode_enabled() {
  case "$(uname -s 2>/dev/null || printf '')" in
    Darwin) ;;
    *) return 1 ;;
  esac
  [ "${INFRING_DASHBOARD_LAUNCHD:-1}" != "0" ] || return 1
  command -v launchctl >/dev/null 2>&1 || return 1
  return 0
}

infring_gateway_launchd_domain() {
  uid="$(id -u 2>/dev/null || true)"
  [ -n "$uid" ] || return 1
  for candidate in "gui/${uid}" "user/${uid}"; do
    if launchctl print "$candidate" >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  printf '%s\n' "user/${uid}"
}

infring_gateway_launchd_label() {
  printf '%s\n' "com.protheuslabs.infring.dashboard.shelltest2"
}

infring_gateway_launchd_plist() {
  label="$(infring_gateway_launchd_label "$1" "$2")"
  printf '%s\n' "$HOME/Library/LaunchAgents/${label}.plist"
}

infring_gateway_launchd_loaded() {
  domain="$(infring_gateway_launchd_domain 2>/dev/null || true)"
  label="$(infring_gateway_launchd_label "$1" "$2")"
  [ -n "$domain" ] || return 1
  launchctl print "${domain}/${label}" >/dev/null 2>&1
}

infring_gateway_xml_escape() {
  printf '%s' "${1:-}" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}

infring_gateway_launchd_write_plist() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  root="${3:-}"
  [ -n "$root" ] || return 1
  label="$(infring_gateway_launchd_label "$host" "$port")"
  plist="$(infring_gateway_launchd_plist "$host" "$port")"
  launch_dir="$(dirname "$plist")"
  mkdir -p "$launch_dir" >/dev/null 2>&1 || return 1
  dashboard_bin="${INFRING_DASHBOARD_BIN:-__INSTALL_DIR__/infringctl}"
  [ -x "$dashboard_bin" ] || dashboard_bin="__INSTALL_DIR__/protheusctl"
  [ -x "$dashboard_bin" ] || return 1
  label_xml="$(infring_gateway_xml_escape "$label")"
  launch_cmd="cd $root && exec $dashboard_bin dashboard-ui serve --host=$host --port=$port"
  launch_cmd_xml="$(infring_gateway_xml_escape "$launch_cmd")"
  cat > "$plist" <<__INFRING_PLIST__
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${label_xml}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/sh</string>
    <string>-lc</string>
    <string>${launch_cmd_xml}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/infring-dashboard-serve.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/infring-dashboard-serve.log</string>
</dict>
</plist>
__INFRING_PLIST__
  return 0
}

infring_gateway_launchd_start() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  root="${3:-}"
  infring_gateway_launchd_mode_enabled || return 1
  [ -n "$root" ] || return 1
  infring_gateway_launchd_write_plist "$host" "$port" "$root" || return 1
  domain="$(infring_gateway_launchd_domain 2>/dev/null || true)"
  label="$(infring_gateway_launchd_label "$host" "$port")"
  plist="$(infring_gateway_launchd_plist "$host" "$port")"
  [ -n "$domain" ] || return 1
  launchctl bootout "${domain}/${label}" >/dev/null 2>&1 || launchctl bootout "$domain" "$plist" >/dev/null 2>&1 || true
  launchctl bootstrap "$domain" "$plist" >/dev/null 2>&1 || return 1
  launchctl enable "${domain}/${label}" >/dev/null 2>&1 || true
  launchctl kickstart -k "${domain}/${label}" >/dev/null 2>&1 || true
  return 0
}

infring_gateway_launchd_stop() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  infring_gateway_launchd_mode_enabled || return 0
  domain="$(infring_gateway_launchd_domain 2>/dev/null || true)"
  label="$(infring_gateway_launchd_label "$host" "$port")"
  plist="$(infring_gateway_launchd_plist "$host" "$port")"
  if [ -n "$domain" ]; then
    launchctl disable "${domain}/${label}" >/dev/null 2>&1 || true
    launchctl bootout "${domain}/${label}" >/dev/null 2>&1 || launchctl bootout "$domain" "$plist" >/dev/null 2>&1 || true
  fi
  rm -f "$plist" >/dev/null 2>&1 || true
  return 0
}

infring_gateway_watchdog_stop() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  pid="$(infring_gateway_watchdog_pid_read "$host" "$port" 2>/dev/null || true)"
  if [ -n "$pid" ] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" >/dev/null 2>&1 || true
    i=0
    while [ "$i" -lt 6 ]; do
      if ! kill -0 "$pid" >/dev/null 2>&1; then
        break
      fi
      i=$((i + 1))
      sleep 1
    done
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill -9 "$pid" >/dev/null 2>&1 || true
    fi
  fi
  infring_gateway_watchdog_pid_clear "$host" "$port"
}

infring_gateway_dashboard_match() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  printf '%s\n' "dashboard-ui serve --host=${host} --port=${port}"
}

infring_gateway_dashboard_match_legacy() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  printf '%s\n' "client/runtime/systems/ui/infring_dashboard.ts serve --host=${host} --port=${port}"
}

infring_gateway_dashboard_process_running() {
  match="$(infring_gateway_dashboard_match "$1" "$2")"
  legacy_match="$(infring_gateway_dashboard_match_legacy "$1" "$2")"
  if command -v pgrep >/dev/null 2>&1; then
    pgrep -f "$match" >/dev/null 2>&1 && return 0
    pgrep -f "$legacy_match" >/dev/null 2>&1 && return 0
  else
    ps ax -o command= 2>/dev/null | awk -v m="$match" -v l="$legacy_match" 'index($0,m)>0 || index($0,l)>0 {found=1} END {exit(found?0:1)}'
    return $?
  fi
  return 1
}

infring_gateway_pid_matches_dashboard() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  pid="$(infring_gateway_pid_read "$host" "$port" 2>/dev/null || true)"
  [ -n "$pid" ] || return 1
  cmd="$(ps -p "$pid" -o command= 2>/dev/null || true)"
  [ -n "$cmd" ] || return 1
  match="$(infring_gateway_dashboard_match "$host" "$port")"
  legacy_match="$(infring_gateway_dashboard_match_legacy "$host" "$port")"
  case "$cmd" in
    *"$match"*) return 0 ;;
    *"$legacy_match"*) return 0 ;;
  esac
  return 1
}

infring_gateway_pid_sanitize() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  if ! infring_gateway_pid_running "$host" "$port"; then
    infring_gateway_pid_clear "$host" "$port"
    return 0
  fi
  if ! infring_gateway_pid_matches_dashboard "$host" "$port"; then
    infring_gateway_pid_clear "$host" "$port"
    return 0
  fi
  return 0
}

infring_gateway_stop_dashboard_managed() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  if infring_gateway_launchd_mode_enabled; then
    infring_gateway_launchd_stop "$host" "$port" >/dev/null 2>&1 || true
  fi
  pid="$(infring_gateway_pid_read "$host" "$port" 2>/dev/null || true)"
  if [ -n "$pid" ]; then
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
      i=0
      while [ "$i" -lt 8 ]; do
        if ! kill -0 "$pid" >/dev/null 2>&1; then
          break
        fi
        i=$((i + 1))
        sleep 1
      done
      if kill -0 "$pid" >/dev/null 2>&1; then
        kill -9 "$pid" >/dev/null 2>&1 || true
      fi
    fi
  fi
  match="$(infring_gateway_dashboard_match "$host" "$port")"
  if command -v pkill >/dev/null 2>&1; then
    pkill -f "$match" >/dev/null 2>&1 || true
    sleep 1
    pkill -9 -f "$match" >/dev/null 2>&1 || true
  else
    pids="$(ps ax -o pid= -o command= 2>/dev/null | awk -v m="$match" 'index($0,m)>0 {print $1}')"
    for row in $pids; do
      kill "$row" >/dev/null 2>&1 || true
    done
    sleep 1
    for row in $pids; do
      kill -9 "$row" >/dev/null 2>&1 || true
    done
  fi
  infring_gateway_pid_clear "$host" "$port"
  return 0
}

infring_gateway_health_ok() {
  host="$1"
  port="$2"
  if command -v curl >/dev/null 2>&1; then
    curl --connect-timeout 2 --max-time 35 -fsS "http://${host}:${port}/healthz" >/dev/null 2>&1 && return 0
  elif command -v wget >/dev/null 2>&1; then
    wget --timeout=35 -q -O - "http://${host}:${port}/healthz" >/dev/null 2>&1 && return 0
  fi
  return 1
}

infring_gateway_wait_dashboard() {
  host="$1"
  port="$2"
  attempts="${3:-1}"
  i=0
  while [ "$i" -lt "$attempts" ]; do
    if infring_gateway_health_ok "$host" "$port"; then
      return 0
    fi
    i=$((i + 1))
    sleep 1
  done
  return 1
}

infring_gateway_wait_dashboard_adaptive() {
  host="$1"
  port="$2"
  timeout_s="${3:-90}"
  enforce_fallback="${4:-1}"
  i=0
  while [ "$i" -lt "$timeout_s" ]; do
    if infring_gateway_health_ok "$host" "$port"; then
      return 0
    fi
    if [ "$enforce_fallback" = "1" ]; then
      infring_gateway_pid_sanitize "$host" "$port"
      if ! infring_gateway_dashboard_process_running "$host" "$port"; then
        infring_gateway_start_dashboard_fallback "$host" "$port" >/dev/null 2>&1 || true
      fi
    fi
    i=$((i + 1))
    sleep 1
  done
  return 1
}

infring_gateway_watchdog_loop() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  interval_s="${3:-5}"
  case "$interval_s" in
    ''|*[!0-9]*)
      interval_s=5
      ;;
  esac
  if [ "$interval_s" -lt 2 ]; then
    interval_s=2
  fi
  if [ "$interval_s" -gt 60 ]; then
    interval_s=60
  fi
  trap 'exit 0' INT TERM
  while true; do
    infring_gateway_pid_sanitize "$host" "$port"
    if ! infring_gateway_health_ok "$host" "$port"; then
      if ! infring_gateway_dashboard_process_running "$host" "$port"; then
        infring_gateway_start_dashboard_fallback "$host" "$port" >/dev/null 2>&1 || true
      fi
      infring_gateway_wait_dashboard "$host" "$port" 8 >/dev/null 2>&1 || true
    fi
    sleep "$interval_s"
  done
}

infring_gateway_spawn_detached_logged() {
  log_file="$1"
  shift || true
  [ -n "$log_file" ] || return 1
  [ "$#" -gt 0 ] || return 1
  if command -v setsid >/dev/null 2>&1; then
    setsid "$@" </dev/null >>"$log_file" 2>&1 &
  elif command -v nohup >/dev/null 2>&1; then
    nohup "$@" </dev/null >>"$log_file" 2>&1 &
  else
    "$@" </dev/null >>"$log_file" 2>&1 &
  fi
  printf '%s\n' "$!"
  return 0
}

infring_gateway_watchdog_start() {
  host="${1:-127.0.0.1}"
  port="${2:-4173}"
  interval_s="${3:-5}"
  case "$interval_s" in
    ''|*[!0-9]*)
      interval_s=5
      ;;
  esac
  if [ "$interval_s" -lt 2 ]; then
    interval_s=2
  fi
  if [ "$interval_s" -gt 60 ]; then
    interval_s=60
  fi
  if infring_gateway_launchd_mode_enabled && infring_gateway_launchd_loaded "$host" "$port"; then
    return 0
  fi
  if infring_gateway_watchdog_pid_running "$host" "$port"; then
    return 0
  fi
  infring_gateway_watchdog_pid_clear "$host" "$port"
  watchdog_pid="$(infring_gateway_spawn_detached_logged /tmp/infring-dashboard-watchdog.log "$0" "__dashboard-watchdog" "--host=${host}" "--port=${port}" "--interval=${interval_s}" 2>/dev/null || true)"
  if [ -n "$watchdog_pid" ]; then
    printf '%s\n' "$watchdog_pid" > "$(infring_gateway_watchdog_pidfile "$host" "$port")"
  fi
  return 0
}

infring_gateway_start_dashboard_fallback() {
  host="$1"
  port="$2"
  infring_gateway_pid_sanitize "$host" "$port"
  if infring_gateway_health_ok "$host" "$port"; then
    return 0
  fi
  if infring_gateway_dashboard_process_running "$host" "$port"; then
    return 0
  fi
  root=""
  for candidate in "${INFRING_WORKSPACE_ROOT:-}" "${PROTHEUS_WORKSPACE_ROOT:-}" "${PWD:-.}"; do
    [ -n "$candidate" ] || continue
    if [ -f "$candidate/core/layer0/ops/Cargo.toml" ] && [ -d "$candidate/client/runtime" ]; then
      root="$candidate"
      break
    fi
  done
  [ -n "$root" ] || return 1
  dashboard_bin="${INFRING_DASHBOARD_BIN:-__INSTALL_DIR__/infringctl}"
  [ -x "$dashboard_bin" ] || dashboard_bin="__INSTALL_DIR__/protheusctl"
  [ -x "$dashboard_bin" ] || return 1
  if infring_gateway_launchd_mode_enabled; then
    if infring_gateway_launchd_start "$host" "$port" "$root" >/dev/null 2>&1; then
      match="$(infring_gateway_dashboard_match "$host" "$port")"
      if command -v pgrep >/dev/null 2>&1; then
        child_pid="$(pgrep -f "$match" | head -n 1 || true)"
        if [ -n "$child_pid" ]; then
          printf '%s\n' "$child_pid" > "$(infring_gateway_pidfile "$host" "$port")"
        fi
      fi
      return 0
    fi
  fi
  (
    cd "$root" 2>/dev/null || exit 1
    child_pid="$(infring_gateway_spawn_detached_logged /tmp/infring-dashboard-serve.log "$dashboard_bin" \
      dashboard-ui serve "--host=${host}" "--port=${port}" \
      2>/dev/null || true)"
    if [ -n "$child_pid" ]; then
      printf '%s\n' "$child_pid" > "$(infring_gateway_pidfile "$host" "$port")"
    fi
  ) >/dev/null 2>&1 || return 1
  return 0
}

if [ "${1:-}" = "__dashboard-watchdog" ]; then
  shift || true
  watchdog_host="127.0.0.1"
  watchdog_port="4173"
  watchdog_interval="5"
  for token in "$@"; do
    case "$token" in
      --host=*)
        watchdog_host="${token#*=}"
        ;;
      --port=*)
        watchdog_port="${token#*=}"
        ;;
      --interval=*)
        watchdog_interval="${token#*=}"
        ;;
    esac
  done
  infring_gateway_watchdog_loop "$watchdog_host" "$watchdog_port" "$watchdog_interval"
  exit 0
fi

if [ "${1:-}" = "gateway" ]; then
  shift || true
  gateway_action=""
  case "${1:-}" in
    "" )
      gateway_action="start"
      ;;
    start|boot|stop|restart|status|attach|subscribe|tick|diagnostics|efficiency-status|embedded-core-status)
      gateway_action="${1:-start}"
      shift || true
      ;;
    --help|-h|help)
      echo "Usage: infring gateway [start|stop|restart|status|attach|subscribe|tick|diagnostics] [flags]"
      echo "  default action is 'start'"
      echo "  add --dashboard-open=0 to skip browser auto-open on start"
      exit 0
      ;;
    *)
      gateway_action="start"
      ;;
  esac

  if [ "$gateway_action" = "boot" ]; then
    gateway_action="start"
  fi

  gateway_output="$("__INSTALL_DIR__/infringd" "$gateway_action" "$@" 2>&1)"
  gateway_status=$?
  if [ "$gateway_status" -ne 0 ]; then
    if [ -n "$gateway_output" ]; then
      printf '%s\n' "$gateway_output" >&2
    fi
    echo "[infring gateway] ${gateway_action} failed" >&2
    exit "$gateway_status"
  fi
  if [ "${INFRING_GATEWAY_RAW:-0}" = "1" ] || [ "${PROTHEUS_GATEWAY_RAW:-0}" = "1" ]; then
    if [ -n "$gateway_output" ]; then
      printf '%s\n' "$gateway_output"
    fi
  fi

  receipt_hash="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"receipt_hash":"\([^"]*\)".*/\1/p' | head -n 1)"
  root_path="$(printf '%s\n' "$gateway_output" | sed -n 's/.*"root":"\([^"]*\)".*/\1/p' | head -n 1)"
  if [ -z "$root_path" ] && [ -n "${INFRING_WORKSPACE_ROOT:-}" ]; then
    root_path="$INFRING_WORKSPACE_ROOT"
  fi

  dashboard_open="1"
  dashboard_host="127.0.0.1"
  dashboard_port="4173"
  dashboard_watchdog_enabled="${INFRING_DASHBOARD_WATCHDOG:-0}"
  dashboard_watchdog_interval="${INFRING_DASHBOARD_WATCHDOG_INTERVAL:-5}"
  legacy_supervisor_mode="${INFRING_GATEWAY_LEGACY_SUPERVISOR:-0}"
  case "$dashboard_watchdog_interval" in
    ''|*[!0-9]*)
      dashboard_watchdog_interval=5
      ;;
  esac
  if [ "$dashboard_watchdog_interval" -lt 2 ]; then
    dashboard_watchdog_interval=2
  fi
  if [ "$dashboard_watchdog_interval" -gt 60 ]; then
    dashboard_watchdog_interval=60
  fi
  if [ "${INFRING_NO_BROWSER:-0}" = "1" ] || [ "${PROTHEUS_NO_BROWSER:-0}" = "1" ]; then
    dashboard_open="0"
  fi
  for token in "$@"; do
    case "$token" in
      --dashboard-open=0|--no-browser)
        dashboard_open="0"
        ;;
      --dashboard-open=1)
        dashboard_open="1"
        ;;
      --dashboard-host=*)
        dashboard_host="${token#*=}"
        ;;
      --dashboard-port=*)
        dashboard_port="${token#*=}"
        ;;
    esac
  done
  dashboard_url="${INFRING_DASHBOARD_URL:-http://${dashboard_host}:${dashboard_port}/dashboard#chat}"
  core_opened_browser="0"
  if printf '%s\n' "$gateway_output" | grep -Eq '"opened_browser"[[:space:]]*:[[:space:]]*true'; then
    core_opened_browser="1"
  fi

  if [ "$gateway_action" = "start" ] || [ "$gateway_action" = "restart" ]; then
    if [ "$legacy_supervisor_mode" != "1" ]; then
      # Default mode: Rust core is sole authority for dashboard lifecycle/watchdog.
      infring_gateway_watchdog_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_watchdog_pid_clear "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_launchd_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_pid_sanitize "$dashboard_host" "$dashboard_port"
    elif [ "$gateway_action" = "restart" ]; then
      infring_gateway_watchdog_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      infring_gateway_stop_dashboard_managed "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
    fi
    dashboard_ready="0"
    if infring_gateway_wait_dashboard "$dashboard_host" "$dashboard_port" 20; then
      dashboard_ready="1"
    else
      if [ "$legacy_supervisor_mode" = "1" ] && [ "${INFRING_DASHBOARD_FALLBACK:-1}" != "0" ] && [ "${PROTHEUS_DASHBOARD_FALLBACK:-1}" != "0" ]; then
        infring_gateway_start_dashboard_fallback "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
      fi
      if [ "$dashboard_ready" != "1" ]; then
        wait_max="${INFRING_DASHBOARD_WAIT_MAX:-90}"
        case "$wait_max" in
          ''|*[!0-9]*)
            wait_max=90
            ;;
        esac
        adaptive_fallback=0
        if [ "$legacy_supervisor_mode" = "1" ]; then
          adaptive_fallback=1
        fi
        if infring_gateway_wait_dashboard_adaptive "$dashboard_host" "$dashboard_port" "$wait_max" "$adaptive_fallback"; then
          dashboard_ready="1"
        fi
      fi
    fi
    # Core daemon-control is authoritative for browser launch.
    # Only fallback-open here when core explicitly did not open a tab.
    if [ "$dashboard_open" = "1" ] && [ "$core_opened_browser" != "1" ]; then
      if command -v open >/dev/null 2>&1; then
        open "$dashboard_url" >/dev/null 2>&1 || true
      elif command -v xdg-open >/dev/null 2>&1; then
        xdg-open "$dashboard_url" >/dev/null 2>&1 || true
      fi
    fi
    if [ "$dashboard_ready" != "1" ]; then
      echo "[infring gateway] warning: dashboard healthz not ready at http://${dashboard_host}:${dashboard_port}/healthz" >&2
    fi
    if [ "$legacy_supervisor_mode" = "1" ] && [ "$dashboard_watchdog_enabled" != "0" ]; then
      infring_gateway_watchdog_start "$dashboard_host" "$dashboard_port" "$dashboard_watchdog_interval" >/dev/null 2>&1 || true
    fi
    if [ "$gateway_action" = "restart" ]; then
      echo "[infring gateway] runtime restarted"
    else
      echo "[infring gateway] runtime started"
    fi
    echo "[infring gateway] dashboard: $dashboard_url"
    [ -n "$root_path" ] && echo "[infring gateway] workspace: $root_path"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
  elif [ "$gateway_action" = "stop" ]; then
    infring_gateway_watchdog_stop "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
    infring_gateway_stop_dashboard_managed "$dashboard_host" "$dashboard_port" >/dev/null 2>&1 || true
    echo "[infring gateway] runtime stopped"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
  elif [ "$gateway_action" = "status" ]; then
    echo "[infring gateway] runtime status received"
    dashboard_status_healthy="0"
    if infring_gateway_wait_dashboard "$dashboard_host" "$dashboard_port" 3; then
      dashboard_status_healthy="1"
    fi
    if [ "$dashboard_status_healthy" = "1" ]; then
      echo "[infring gateway] dashboard healthy: http://${dashboard_host}:${dashboard_port}/healthz"
    else
      echo "[infring gateway] dashboard down: http://${dashboard_host}:${dashboard_port}/healthz"
    fi
    if infring_gateway_pid_running "$dashboard_host" "$dashboard_port"; then
      echo "[infring gateway] dashboard pid: $(infring_gateway_pid_read "$dashboard_host" "$dashboard_port" 2>/dev/null || true)"
    fi
    core_watchdog_pid=""
    if [ -n "$root_path" ] && [ -f "$root_path/local/state/ops/daemon_control/dashboard_watchdog.pid" ]; then
      core_watchdog_pid="$(sed -n '1p' "$root_path/local/state/ops/daemon_control/dashboard_watchdog.pid" | tr -cd '0-9')"
    fi
    if [ -n "$core_watchdog_pid" ] && kill -0 "$core_watchdog_pid" >/dev/null 2>&1; then
      echo "[infring gateway] dashboard watchdog pid: $core_watchdog_pid (core)"
    elif infring_gateway_watchdog_pid_running "$dashboard_host" "$dashboard_port"; then
      echo "[infring gateway] dashboard watchdog pid: $(infring_gateway_watchdog_pid_read "$dashboard_host" "$dashboard_port" 2>/dev/null || true) (legacy)"
    fi
    if [ "$legacy_supervisor_mode" = "1" ] && infring_gateway_launchd_mode_enabled; then
      if infring_gateway_launchd_loaded "$dashboard_host" "$dashboard_port"; then
        echo "[infring gateway] dashboard supervisor: launchd ($(infring_gateway_launchd_label "$dashboard_host" "$dashboard_port"))"
      else
        echo "[infring gateway] dashboard supervisor: launchd (not loaded)"
      fi
    fi
    echo "[infring gateway] dashboard: $dashboard_url"
    [ -n "$root_path" ] && echo "[infring gateway] workspace: $root_path"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
  else
    echo "[infring gateway] action complete: $gateway_action"
    [ -n "$receipt_hash" ] && echo "[infring gateway] receipt: $receipt_hash"
  fi
  exit 0
fi
EOF
}

main() {
  parse_install_args "$@"
  resolve_install_dir_default

  if [ -n "${INSTALL_TMP_DIR:-}" ]; then
    mkdir -p "$INSTALL_TMP_DIR"
    export TMPDIR="$INSTALL_TMP_DIR"
  fi
  mkdir -p "$INSTALL_DIR"
  if is_truthy "$INSTALL_REPAIR"; then
    echo "[infring install] repair mode enabled"
    repair_install_dir
    repair_workspace_state
  fi
  triple="$(platform_triple)"
  version="$(resolve_version)"

  echo "[infring install] version: $version"
  echo "[infring install] platform: $triple"
  echo "[infring install] install dir: $INSTALL_DIR"

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
    if is_truthy "$INSTALL_TINY_MAX"; then
      if ! install_binary "$version" "$triple" "protheus-pure-workspace-tiny-max" "$pure_bin"; then
        if ! install_binary "$version" "$triple" "protheus-pure-workspace" "$pure_bin"; then
          echo "[infring install] failed to fetch protheus-pure-workspace for $triple ($version)" >&2
          exit 1
        fi
      fi
    elif ! install_binary "$version" "$triple" "protheus-pure-workspace" "$pure_bin"; then
      echo "[infring install] failed to fetch protheus-pure-workspace for $triple ($version)" >&2
      exit 1
    fi
    if is_truthy "$INSTALL_TINY_MAX"; then
      echo "[infring install] tiny-max pure mode selected: Rust-only tiny profile installed"
    else
      echo "[infring install] pure mode selected: Rust-only client installed"
    fi
  else
    if ! install_binary "$version" "$triple" "protheus-ops" "$ops_bin"; then
      echo "[infring install] failed to fetch protheus-ops for $triple ($version)" >&2
      exit 1
    fi
    if ! ensure_ops_gateway_contract "$version" "$ops_bin"; then
      exit 1
    fi
  fi

  if [ "$prefer_musl_protheusd" = "1" ]; then
    if is_truthy "$INSTALL_TINY_MAX"; then
      if install_binary "$version" "x86_64-unknown-linux-musl" "protheusd-tiny-max" "$protheusd_bin"; then
        daemon_wrapper_body="exec \"$protheusd_bin\" \"\$@\""
        echo "[infring install] using static musl tiny-max protheusd"
      fi
    fi
    if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "x86_64-unknown-linux-musl" "protheusd" "$protheusd_bin"; then
      daemon_wrapper_body="exec \"$protheusd_bin\" \"\$@\""
      echo "[infring install] using static musl protheusd (embedded-minimal-core)"
    fi
  fi

  if [ -z "$daemon_wrapper_body" ] && is_truthy "$INSTALL_TINY_MAX"; then
    if install_binary "$version" "$triple" "protheusd-tiny-max" "$protheusd_bin"; then
      daemon_wrapper_body="exec \"$protheusd_bin\" \"\$@\""
      echo "[infring install] using native tiny-max protheusd"
    fi
  fi

  if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "$triple" "protheusd" "$protheusd_bin"; then
    daemon_wrapper_body="exec \"$protheusd_bin\" \"\$@\""
    echo "[infring install] using native protheusd"
  fi

  if [ -z "$daemon_wrapper_body" ] && install_binary "$version" "$triple" "conduit_daemon" "$daemon_bin"; then
    daemon_wrapper_body="exec \"$daemon_bin\" \"\$@\""
    echo "[infring install] using conduit_daemon compatibility fallback"
  else
    if [ -z "$daemon_wrapper_body" ]; then
      echo "[infring install] no dedicated daemon binary found; falling back to protheus-ops spine mode"
    fi
  fi

  gateway_shim="$(gateway_wrapper_body | sed "s|__INSTALL_DIR__|${INSTALL_DIR}|g")"
  if is_truthy "$INSTALL_PURE"; then
    if is_truthy "$INSTALL_TINY_MAX"; then
      write_wrapper "infring" "${gateway_shim}
exec \"$pure_bin\" --tiny-max=1 \"\$@\""
    else
      write_wrapper "infring" "${gateway_shim}
exec \"$pure_bin\" \"\$@\""
    fi
    write_wrapper "infringctl" "${gateway_shim}
exec \"$pure_bin\" conduit \"\$@\""
  else
    write_wrapper "infring" "${gateway_shim}
exec \"$ops_bin\" protheusctl \"\$@\""
    write_wrapper "infringctl" "${gateway_shim}
exec \"$ops_bin\" protheusctl \"\$@\""
  fi

  if [ -n "$daemon_wrapper_body" ]; then
    write_wrapper "infringd" "$daemon_wrapper_body"
  else
    if is_truthy "$INSTALL_PURE"; then
      echo "[infring install] no daemon binary available for pure mode" >&2
      exit 1
    fi
    write_wrapper "infringd" "exec \"$ops_bin\" spine \"\$@\""
  fi

  write_wrapper "protheus" "echo \"[deprecation] 'protheus' is deprecated; use 'infring'.\" >&2; exec \"$INSTALL_DIR/infring\" \"\$@\""
  write_wrapper "protheusctl" "exec \"$INSTALL_DIR/infringctl\" \"\$@\""
  write_wrapper "protheusd" "echo \"[deprecation] 'protheusd' is deprecated; use 'infringd'.\" >&2; exec \"$INSTALL_DIR/infringd\" \"\$@\""

  if is_truthy "$INSTALL_PURE"; then
    echo "[infring install] pure mode: skipping Infring client bundle"
  elif is_truthy "$INSTALL_FULL"; then
    client_dir="$INSTALL_DIR/protheus-client"
    if install_client_bundle "$version" "$triple" "$client_dir"; then
      echo "[infring install] full mode enabled: client runtime installed at $client_dir"
    else
      echo "[infring install] full mode requested but no client runtime bundle was published for this release"
    fi
  else
    echo "[infring install] lazy mode: skipping TS systems/eyes client bundle (use --full to include)"
  fi

  echo "[infring install] installed: infring, infringctl, infringd"
  echo "[infring install] aliases: protheus, protheusctl, protheusd"

  ensure_path_shims
  persist_path_for_shell
  write_path_activate_script
  quickstart_prefix=""
  if command -v infring >/dev/null 2>&1; then
    echo "[infring install] PATH check: infring command available in current shell"
  else
    case ":$PATH:" in
      *":$INSTALL_DIR:"*)
        echo "[infring install] PATH check: install dir is on PATH but shell may require command hash refresh"
        echo "[infring install] activate now: hash -r 2>/dev/null || true"
        quickstart_prefix="hash -r 2>/dev/null || true && "
        ;;
      *)
        if [ -n "$PATH_SHIM_DIR" ]; then
          echo "[infring install] PATH notice: wrappers installed in $INSTALL_DIR and linked from $PATH_SHIM_DIR"
          if [ -n "$PATH_ACTIVATE_FILE" ]; then
            echo "[infring install] activate now: . \"$PATH_ACTIVATE_FILE\""
            quickstart_prefix=". \"$PATH_ACTIVATE_FILE\" && "
          fi
        elif [ -n "$PATH_PERSISTED_FILE" ]; then
          echo "[infring install] activate now: . \"$PATH_PERSISTED_FILE\""
          echo "[infring install] fallback: export PATH=\"$INSTALL_DIR:\$PATH\""
          quickstart_prefix=". \"$PATH_PERSISTED_FILE\" && "
          if [ -n "$PATH_ACTIVATE_FILE" ]; then
            echo "[infring install] portable activate script: . \"$PATH_ACTIVATE_FILE\""
          fi
        else
          echo "[infring install] add to PATH: export PATH=\"$INSTALL_DIR:\$PATH\""
          if [ -n "$PATH_ACTIVATE_FILE" ]; then
            echo "[infring install] portable activate script: . \"$PATH_ACTIVATE_FILE\""
            quickstart_prefix=". \"$PATH_ACTIVATE_FILE\" && "
          else
            quickstart_prefix="export PATH=\"$INSTALL_DIR:\$PATH\" && "
          fi
        fi
        ;;
    esac
  fi
  if [ -n "$PATH_ACTIVATE_FILE" ]; then
    echo "[infring install] activation script: $PATH_ACTIVATE_FILE"
  fi
  echo "[infring install] run now (direct path): \"$INSTALL_DIR/infring\" --help"
  echo "[infring install] quickstart now (direct path): \"$INSTALL_DIR/infring\" gateway"
  echo "[infring install] run: ${quickstart_prefix}infring --help"
  echo "[infring install] quickstart: ${quickstart_prefix}infring gateway"
  echo "[infring install] stop: ${quickstart_prefix}infring gateway stop"

  if [ -n "$SOURCE_FALLBACK_TMP" ] && [ -d "$SOURCE_FALLBACK_TMP" ]; then
    rm -rf "$SOURCE_FALLBACK_TMP"
  fi
}

main "$@"
