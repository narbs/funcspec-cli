#!/usr/bin/env bash
# Install script for funcspec
# Usage: curl -fsSL https://funcspec.net/install.sh | bash
#    or: bash install.sh [--uninstall]
set -euo pipefail

REPO="narbs/funcspec-cli"
BINARY="funcspec"
UNINSTALL=false

# Parse args
for arg in "$@"; do
  case "$arg" in
    --uninstall) UNINSTALL=true ;;
    --help|-h)
      echo "Usage: $0 [--uninstall]"
      echo ""
      echo "Options:"
      echo "  --uninstall   Remove funcspec from the system"
      exit 0
      ;;
  esac
done

# ── Helpers ──────────────────────────────────────────────────────────────────

info()    { printf '\033[0;32m[info]\033[0m %s\n' "$*"; }
warn()    { printf '\033[0;33m[warn]\033[0m %s\n' "$*" >&2; }
error()   { printf '\033[0;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  if ! command -v "$1" &>/dev/null; then
    error "Required command not found: $1"
  fi
}

# ── Determine install directory ───────────────────────────────────────────────

choose_install_dir() {
  if [ -w "/usr/local/bin" ]; then
    echo "/usr/local/bin"
  elif [ -n "${HOME:-}" ]; then
    echo "${HOME}/.local/bin"
  else
    error "Cannot determine a writable install directory."
  fi
}

# ── Uninstall ─────────────────────────────────────────────────────────────────

if [ "$UNINSTALL" = true ]; then
  INSTALL_DIR="$(choose_install_dir)"
  TARGET="${INSTALL_DIR}/${BINARY}"
  if [ -f "$TARGET" ]; then
    rm -f "$TARGET"
    info "Removed $TARGET"
  else
    # Also try common locations
    for dir in /usr/local/bin "${HOME}/.local/bin"; do
      if [ -f "${dir}/${BINARY}" ]; then
        rm -f "${dir}/${BINARY}"
        info "Removed ${dir}/${BINARY}"
        exit 0
      fi
    done
    warn "funcspec not found; nothing to uninstall."
  fi
  exit 0
fi

# ── Detect OS / architecture ─────────────────────────────────────────────────

need_cmd uname
need_cmd curl
need_cmd tar

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)          TARGET="x86_64-unknown-linux-musl" ;;
      aarch64|arm64)   TARGET="aarch64-unknown-linux-gnu" ;;
      *)               error "Unsupported Linux architecture: $ARCH" ;;
    esac
    EXT="tar.gz"
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)          TARGET="x86_64-apple-darwin" ;;
      arm64)           TARGET="aarch64-apple-darwin" ;;
      *)               error "Unsupported macOS architecture: $ARCH" ;;
    esac
    EXT="tar.gz"
    ;;
  *)
    error "Unsupported OS: $OS. For Windows, download the binary from https://github.com/${REPO}/releases"
    ;;
esac

# ── Fetch latest version ──────────────────────────────────────────────────────

info "Fetching latest release version..."
VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"

if [ -z "$VERSION" ]; then
  error "Could not determine latest version. Check https://github.com/${REPO}/releases"
fi

info "Latest version: $VERSION"

# ── Download & verify ─────────────────────────────────────────────────────────

ARCHIVE="${BINARY}-${VERSION}-${TARGET}.${EXT}"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
DOWNLOAD_URL="${BASE_URL}/${ARCHIVE}"
CHECKSUM_URL="${BASE_URL}/checksums.sha256"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

info "Downloading $ARCHIVE..."
curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "${TMP_DIR}/${ARCHIVE}"
curl -fsSL "$CHECKSUM_URL" -o "${TMP_DIR}/checksums.sha256"

info "Verifying checksum..."
cd "$TMP_DIR"

# sha256sum vs shasum (macOS)
if command -v sha256sum &>/dev/null; then
  grep "${ARCHIVE}" checksums.sha256 | sha256sum --check --status
elif command -v shasum &>/dev/null; then
  EXPECTED="$(grep "${ARCHIVE}" checksums.sha256 | awk '{print $1}')"
  ACTUAL="$(shasum -a 256 "${ARCHIVE}" | awk '{print $1}')"
  if [ "$EXPECTED" != "$ACTUAL" ]; then
    error "Checksum mismatch! Expected: $EXPECTED  Got: $ACTUAL"
  fi
else
  warn "No sha256sum or shasum found — skipping checksum verification."
fi

info "Checksum OK."

# ── Extract & install ─────────────────────────────────────────────────────────

tar xzf "${ARCHIVE}"

INSTALL_DIR="$(choose_install_dir)"
mkdir -p "$INSTALL_DIR"

mv "${BINARY}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

info "Installed funcspec ${VERSION} to ${INSTALL_DIR}/${BINARY}"

# PATH hint
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    warn "${INSTALL_DIR} is not in your PATH."
    warn "Add this to your shell profile:"
    warn "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac

info "Run 'funcspec --help' to get started, or 'funcspec login' to authenticate."
