#!/usr/bin/env sh
# agent-undo installer.
#
# Usage:
#   curl -fsSL https://agent-undo.dev/install.sh | sh
#
# Detects OS + arch, downloads the matching release binary from GitHub, and
# drops it into ~/.local/bin (or $AGENT_UNDO_INSTALL_DIR if set). Creates
# the directory if needed and prints a PATH hint if it isn't already on PATH.

set -eu

REPO="peaktwilight/agent-undo"
BIN_NAME="au"
INSTALL_DIR="${AGENT_UNDO_INSTALL_DIR:-$HOME/.local/bin}"

err()  { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info() { printf '\033[36m::\033[0m %s\n' "$*"; }
ok()   { printf '\033[32m✓\033[0m %s\n' "$*"; }

# --- detect platform ----------------------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)   OS_TAG="unknown-linux-gnu" ;;
  Darwin)  OS_TAG="apple-darwin" ;;
  *)       err "unsupported OS: $OS (need Linux or macOS)" ;;
esac

case "$ARCH" in
  x86_64|amd64)        ARCH_TAG="x86_64" ;;
  arm64|aarch64)       ARCH_TAG="aarch64" ;;
  *)                   err "unsupported architecture: $ARCH" ;;
esac

TARGET="${ARCH_TAG}-${OS_TAG}"
info "detected platform: $TARGET"

# --- resolve latest release tag ----------------------------------------------
TAG="${AGENT_UNDO_VERSION:-}"
if [ -z "$TAG" ]; then
  info "fetching latest release tag from $REPO..."
  TAG="$(
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
      | grep -o '"tag_name":[[:space:]]*"[^"]*"' \
      | head -n 1 \
      | sed 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/'
  )" || err "could not determine latest release. Set AGENT_UNDO_VERSION=v0.x.y to override."
  if [ -z "$TAG" ]; then
    err "no releases yet at https://github.com/$REPO/releases. \
Use \`cargo install agent-undo\` for now."
  fi
fi
info "installing $TAG"

# --- download tarball --------------------------------------------------------
TARBALL="${BIN_NAME}-${TAG}-${TARGET}.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$TARBALL"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

info "downloading $URL"
if ! curl -fsSL "$URL" -o "$TMP/$TARBALL"; then
  err "download failed. Check that the release exists for $TARGET."
fi

info "extracting"
tar -xzf "$TMP/$TARBALL" -C "$TMP"

if [ ! -f "$TMP/$BIN_NAME" ]; then
  # Some release pipelines nest the binary in a subdirectory.
  FOUND="$(find "$TMP" -name "$BIN_NAME" -type f | head -n 1)"
  [ -n "$FOUND" ] || err "binary $BIN_NAME not found in tarball"
  mv "$FOUND" "$TMP/$BIN_NAME"
fi

chmod +x "$TMP/$BIN_NAME"

# --- install -----------------------------------------------------------------
mkdir -p "$INSTALL_DIR"
mv "$TMP/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
ok "installed $BIN_NAME to $INSTALL_DIR/$BIN_NAME"

# --- PATH hint ---------------------------------------------------------------
case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    ok "$INSTALL_DIR is already on your PATH"
    ;;
  *)
    printf '\n'
    printf '\033[33m!\033[0m Add %s to your PATH:\n' "$INSTALL_DIR"
    printf '    echo '\''export PATH="%s:$PATH"'\'' >> ~/.zshrc   # or ~/.bashrc\n' "$INSTALL_DIR"
    printf '    source ~/.zshrc\n'
    ;;
esac

printf '\n'
"$INSTALL_DIR/$BIN_NAME" --version 2>/dev/null || true
printf '\n'
ok 'next: cd into a project and run `au init --install-hooks`'
