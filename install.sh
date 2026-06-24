#!/bin/sh
set -e

# ── Avenys (mire) Install Script ──────────────────────────────────────
# Installs the mire compiler from the latest GitHub release artifact
# Usage: curl -fsSL <url> | sh
# Options:
#   MIRE_TAG   - specific release tag (default: fetches latest from API)
#   MIRE_PREFIX - install prefix (default: /usr/local)

REPO="mire-lang/Avenys-rust"
PREFIX="${MIRE_PREFIX:-/usr/local}"
BIN_DIR="${PREFIX}/bin"
LIB_DIR="${PREFIX}/lib/mire"

echo "┌─ Avenys Mire Compiler Install ──────────────────────────────────┐"
echo "│ repo:   ${REPO}"
echo "│ prefix: ${PREFIX}"
echo "└─────────────────────────────────────────────────────────────────┘"

TARBALL="mire-linux-x86_64.tar.gz"

get_latest_tag() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    fi
}

if [ -n "${MIRE_TAG:-}" ]; then
    TAG="$MIRE_TAG"
else
    TAG="$(get_latest_tag)"
    if [ -z "$TAG" ]; then
        echo "error: could not determine latest tag. Set MIRE_TAG manually."
        exit 1
    fi
fi

echo "│ tag:    ${TAG}"
URL="https://github.com/${REPO}/releases/download/${TAG}/${TARBALL}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo ""
echo "  downloading ${URL}..."
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$TMPDIR/$TARBALL"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$URL" -O "$TMPDIR/$TARBALL"
else
    echo "error: need curl or wget"
    exit 1
fi

echo "  extracting..."
tar xzf "$TMPDIR/$TARBALL" -C "$TMPDIR"

echo "  installing mire runtime..."
sudo mkdir -p "$BIN_DIR" "$LIB_DIR"
sudo cp "$TMPDIR/mire/mire" "$BIN_DIR/mire"
sudo chmod +x "$BIN_DIR/mire"

if [ -d "$TMPDIR/mire/runtime" ]; then
    sudo cp -r "$TMPDIR/mire/runtime" "$LIB_DIR/"
fi
if [ -d "$TMPDIR/mire/pal" ]; then
    sudo cp -r "$TMPDIR/mire/pal" "$LIB_DIR/"
fi
if [ -d "$TMPDIR/mire/kioto" ]; then
    echo "  installing kioto stdlib..."
    mkdir -p "$HOME/.owl/modules"
    cp -r "$TMPDIR/mire/kioto" "$HOME/.owl/modules/kioto"
fi

echo ""
echo "  verifying..."
"$BIN_DIR/mire" --version || echo "  (version check failed, continuing)"

echo ""
echo "  install complete: $(which mire || echo "$BIN_DIR/mire")"
echo "  try: mire --help"
