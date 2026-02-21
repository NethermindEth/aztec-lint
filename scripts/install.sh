#!/usr/bin/env bash
set -euo pipefail

TOOL="aztec-lint"
REPO="${AZTEC_LINT_REPO:-NethermindEth/aztec-lint}"
INSTALL_DIR="${AZTEC_LINT_INSTALL_DIR:-$HOME/.local/bin}"
VERSION_INPUT="${1:-${AZTEC_LINT_VERSION:-latest}}"

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

normalize_os() {
  case "$(uname -s)" in
    Linux) printf 'linux' ;;
    Darwin) printf 'macos' ;;
    *) fail "unsupported operating system: $(uname -s)" ;;
  esac
}

normalize_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf 'x86_64' ;;
    arm64|aarch64) printf 'aarch64' ;;
    *) fail "unsupported architecture: $(uname -m)" ;;
  esac
}

resolve_tag() {
  if [[ "$VERSION_INPUT" == "latest" ]]; then
    printf 'latest'
    return
  fi

  if [[ "$VERSION_INPUT" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf '%s' "$VERSION_INPUT"
    return
  fi

  if [[ "$VERSION_INPUT" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf 'v%s' "$VERSION_INPUT"
    return
  fi

  fail "version must be 'latest', 'vX.Y.Z', or 'X.Y.Z'"
}

verify_checksum() {
  local workdir="$1"
  local checksums_file="$2"
  local asset_name="$3"
  local line

  line="$(grep "  ${asset_name}$" "${checksums_file}" || true)"
  [[ -n "$line" ]] || fail "missing checksum entry for ${asset_name}"

  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$workdir" && printf '%s\n' "$line" | sha256sum -c -)
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    (cd "$workdir" && printf '%s\n' "$line" | shasum -a 256 -c -)
    return
  fi

  fail "neither sha256sum nor shasum is available for checksum verification"
}

main() {
  require_cmd curl
  require_cmd tar
  require_cmd install

  local os
  local arch
  local tag
  local base_url
  local asset
  local checksum_asset
  local tmpdir
  local unpack_dir

  os="$(normalize_os)"
  arch="$(normalize_arch)"
  tag="$(resolve_tag)"

  case "${os}-${arch}" in
    linux-x86_64|macos-x86_64|macos-aarch64) ;;
    *) fail "no release artifact is published for ${os}-${arch}" ;;
  esac

  asset="${TOOL}-${os}-${arch}.tar.gz"
  checksum_asset="checksums.txt"

  if [[ "$tag" == "latest" ]]; then
    base_url="https://github.com/${REPO}/releases/latest/download"
  else
    base_url="https://github.com/${REPO}/releases/download/${tag}"
  fi

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  log "Downloading ${asset} from ${REPO} (${tag})..."
  curl -fsSL "${base_url}/${asset}" -o "${tmpdir}/${asset}" || fail "failed to download ${asset}"

  log "Downloading checksums..."
  curl -fsSL "${base_url}/${checksum_asset}" -o "${tmpdir}/${checksum_asset}" \
    || fail "failed to download ${checksum_asset}"

  log "Verifying checksum..."
  verify_checksum "$tmpdir" "${tmpdir}/${checksum_asset}" "$asset"

  log "Installing to ${INSTALL_DIR}..."
  tar -xzf "${tmpdir}/${asset}" -C "$tmpdir"
  unpack_dir="${tmpdir}/${TOOL}-${os}-${arch}"
  [[ -x "${unpack_dir}/${TOOL}" ]] || fail "binary not found in release archive"

  mkdir -p "$INSTALL_DIR"
  install -m 0755 "${unpack_dir}/${TOOL}" "${INSTALL_DIR}/${TOOL}"

  log "Installed ${TOOL} to ${INSTALL_DIR}/${TOOL}"
  if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    log "${INSTALL_DIR} is not on PATH. Run with full path or add it to PATH."
  fi
  log "Run '${TOOL} --help' to verify installation."
}

main "$@"
