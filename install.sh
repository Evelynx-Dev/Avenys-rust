#!/bin/sh
set -e

# ── Avenys Toolchain Install ──────────────────────────────────────────
# Installs mire + owl + kioto from the latest GitHub release artifact.
#
# Usage:
#   curl -fsSL <url> | sh                           # interactive (default)
#   curl -fsSL <url> | sh -s -- --yes                # non-interactive (CI)
#   curl -fsSL <url> | sh -s -- --prefix ~/.local    # user install
#   curl -fsSL <url> | sh -s -- --no-owl             # mire only
#   curl -fsSL <url> | sh -s -- --no-profile         # skip PATH setup
#   curl -fsSL <url> | sh -s -- --tag B1.1           # specific release

REPO="mire-lang/Avenys-rust"
TARBALL="mire-linux-x86_64.tar.gz"
PREFIX=""
TAG=""
YES=0
NO_OWL=0
NO_PROFILE=0

usage() {
    cat <<'USAGE'
Avenys Toolchain Install

Usage:
  install.sh [options]

Options:
  --yes, -y        Non-interactive (skip confirmations)
  --prefix <path>  Install prefix (default: /usr/local)
  --no-owl         Install only mire, skip owl
  --no-profile     Skip shell profile PATH modification
  --tag <tag>      Specific release tag (default: latest)
  --help, -h       Show this help

Examples:
  curl https://.../install.sh | sh                    # interactive
  curl https://.../install.sh | sh -s -- --yes        # non-interactive
  install.sh --prefix ~/.local                        # user-local
  install.sh --no-owl                                 # compiler only
USAGE
}

# ── Flag parsing ──────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
    case "$1" in
        --yes|-y)      YES=1; shift ;;
        --prefix)      PREFIX="$2"; shift 2 ;;
        --no-owl)      NO_OWL=1; shift ;;
        --no-profile)  NO_PROFILE=1; shift ;;
        --tag)         TAG="$2"; shift 2 ;;
        --help|-h)     usage; exit 0 ;;
        --)            shift; break ;;
        -*)            echo "error: unknown option: $1" >&2; usage; exit 1 ;;
        *)             echo "error: unexpected argument: $1" >&2; usage; exit 1 ;;
    esac
done

if [ -z "$PREFIX" ]; then
    PREFIX="${MIRE_PREFIX:-/usr/local}"
fi

# Expand ~ in prefix
case "$PREFIX" in
    ~/*) PREFIX="${HOME}${PREFIX#~}" ;;
    ~)   PREFIX="${HOME}" ;;
esac

BIN_DIR="${PREFIX}/bin"
LIB_DIR="${PREFIX}/lib/mire"

banner() {
    echo ""
    echo "┌─ Avenys Toolchain Install ───────────────────────────────────┐"
    echo "│ repo:   ${REPO}"
    echo "│ prefix: ${PREFIX}"
    [ "$NO_OWL" = "1" ] && echo "│ mire only" || echo "│ mire + owl + kioto"
    echo "└──────────────────────────────────────────────────────────────┘"
}

banner

# ── Flags for sudo ────────────────────────────────────────────────────
needs_sudo() {
    # System prefixes (need sudo) vs user prefixes (don't)
    case "$PREFIX" in
        /usr|/usr/local|/opt*|/etc*) return 0 ;;
        *) return 1 ;;
    esac
}

# ── Prerequisite detection & install ──────────────────────────────────
detect_pkg_manager() {
    if command -v apt-get >/dev/null 2>&1; then
        echo "apt"
    elif command -v pacman >/dev/null 2>&1; then
        echo "pacman"
    elif command -v dnf >/dev/null 2>&1; then
        echo "dnf"
    elif command -v yum >/dev/null 2>&1; then
        echo "yum"
    elif command -v apk >/dev/null 2>&1; then
        echo "apk"
    elif command -v zypper >/dev/null 2>&1; then
        echo "zypper"
    else
        echo "none"
    fi
}

install_deps() {
    local pm="$1"
    echo ""
    echo "  installing: curl tar clang llvm"
    echo "  manager: ${pm}"

    if [ "$YES" != "1" ]; then
        read -r -p "  proceed? [Y/n] " ans
        case "$ans" in
            [nN]*) echo "  skipping dependency install"; return ;;
        esac
    fi

    case "$pm" in
        apt)
            sudo apt-get update -qq
            sudo apt-get install -y -qq curl tar clang llvm-dev 2>/dev/null || \
            sudo apt-get install -y -qq curl tar clang llvm-18-dev 2>/dev/null || \
            sudo apt-get install -y -qq curl tar clang
            ;;
        pacman)
            sudo pacman -Sy --noconfirm curl tar clang llvm
            ;;
        dnf|yum)
            sudo "$pm" install -y curl tar clang llvm-devel
            ;;
        apk)
            sudo apk add curl tar clang llvm-dev
            ;;
        zypper)
            sudo zypper install -y curl tar clang llvm-devel
            ;;
    esac
}

check_prerequisites() {
    local missing=""
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        missing="$missing curl"
    fi
    if ! command -v tar >/dev/null 2>&1; then
        missing="$missing tar"
    fi
    if ! command -v clang >/dev/null 2>&1; then
        missing="$missing clang"
    fi

    if [ -z "$missing" ]; then
        return 0
    fi

    local pm
    pm="$(detect_pkg_manager)"

    if [ "$pm" = "none" ]; then
        echo ""
        echo "  warning: missing:${missing}"
        echo "  install these and re-run. continuing..."
        return 0
    fi

    install_deps "$pm"
}

check_prerequisites

# ── Shell detection for PATH ──────────────────────────────────────────
detect_shell_profile() {
    local sh
    sh="$(basename "${SHELL:-/bin/sh}")"
    case "$sh" in
        zsh)  echo "${ZDOTDIR:-$HOME}/.zshrc" ;;
        fish) echo "$HOME/.config/fish/config.fish" ;;
        bash)
            if [ -f "$HOME/.bash_profile" ]; then
                echo "$HOME/.bash_profile"
            else
                echo "$HOME/.bashrc"
            fi
            ;;
        *)    echo "$HOME/.profile" ;;
    esac
}

# ── Get release tag ───────────────────────────────────────────────────
get_latest_tag() {
    local tag
    if command -v curl >/dev/null 2>&1; then
        tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
    elif command -v wget >/dev/null 2>&1; then
        tag="$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
    fi
    echo "$tag"
}

if [ -z "$TAG" ]; then
    TAG="$(get_latest_tag)"
    if [ -z "$TAG" ]; then
        echo "error: could not determine latest tag. Use --tag <tag>."
        exit 1
    fi
fi

# ── Confirmation ──────────────────────────────────────────────────────
echo ""
echo "  tag:     ${TAG}"
echo "  bin dir: ${BIN_DIR}"
echo "  lib dir: ${LIB_DIR}"

if [ "$YES" != "1" ]; then
    echo ""
    echo "  Will install:"
    echo "    • mire compiler  → ${BIN_DIR}/mire"
    echo "    • mire runtime   → ${LIB_DIR}/"
    if [ "$NO_OWL" != "1" ]; then
        echo "    • owl (pm)       → ${BIN_DIR}/owl"
    fi
    echo "    • kioto stdlib   → ~/.owl/modules/kioto/"
    if [ "$NO_PROFILE" != "1" ]; then
        local profile
        profile="$(detect_shell_profile)"
        echo "    • PATH update    → ${profile}"
    fi
    if needs_sudo; then
        echo ""
        echo "  sudo needed for system install."
    fi
    echo ""
    read -r -p "  continue? [Y/n] " ans
    case "$ans" in
        [nN]*) echo "  aborted."; exit 0 ;;
    esac
fi

# ── Download & extract ────────────────────────────────────────────────
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

if [ ! -f "$TMPDIR/mire/mire" ]; then
    echo "error: release tarball missing mire binary"
    echo "  tarball contents:"
    find "$TMPDIR/mire" -type f | sort
    exit 1
fi

# ── Install mire ──────────────────────────────────────────────────────
install_file() {
    local src="$1" dst="$2"
    if needs_sudo; then
        sudo mkdir -p "$(dirname "$dst")"
        sudo cp "$src" "$dst"
        sudo chmod +x "$dst"
    else
        mkdir -p "$(dirname "$dst")"
        cp "$src" "$dst"
        chmod +x "$dst"
    fi
}

install_dir() {
    local src="$1"
    local name
    name="$(basename "$src")"
    if needs_sudo; then
        sudo mkdir -p "${LIB_DIR}"
        sudo rm -rf "${LIB_DIR}/${name}"
        sudo cp -r "$src" "${LIB_DIR}/"
    else
        mkdir -p "${LIB_DIR}"
        rm -rf "${LIB_DIR}/${name}"
        cp -r "$src" "${LIB_DIR}/"
    fi
}

echo ""
echo "  installing mire..."
install_file "$TMPDIR/mire/mire" "$BIN_DIR/mire"

echo "  installing runtime + pal..."
if [ -d "$TMPDIR/mire/runtime" ]; then
    install_dir "$TMPDIR/mire/runtime"
fi
if [ -d "$TMPDIR/mire/pal" ]; then
    install_dir "$TMPDIR/mire/pal"
fi

echo ""
echo "  verifying mire..."
"$BIN_DIR/mire" --version 2>&1 || echo "  (version check failed)"

# ── Install owl ───────────────────────────────────────────────────────
if [ "$NO_OWL" != "1" ] && [ -f "$TMPDIR/mire/owl" ]; then
    echo ""
    echo "  installing owl..."
    install_file "$TMPDIR/mire/owl" "$BIN_DIR/owl"
    "$BIN_DIR/owl" -V 2>&1 || echo "  (owl version check failed)"
elif [ "$NO_OWL" != "1" ]; then
    echo ""
    echo "  (owl not in release — skipping)"
fi

# ── Setup kioto + owl home ────────────────────────────────────────────
echo ""
echo "  setting up kioto..."
OWL_HOME="$HOME/.owl"
mkdir -p "$OWL_HOME/modules" "$OWL_HOME/tmp"

if [ ! -f "$OWL_HOME/config.toml" ]; then
    cat > "$OWL_HOME/config.toml" << 'CONFIG'
[owl]
version = "1.0.0"

[modules]
path = "~/.owl/modules"

[download]
timeout = 30
retry = 3
CONFIG
    echo "  created ~/.owl/config.toml"
fi

if [ -d "$TMPDIR/mire/kioto" ]; then
    rm -rf "$OWL_HOME/modules/kioto"
    cp -r "$TMPDIR/mire/kioto" "$OWL_HOME/modules/kioto"
    echo "  kioto → ~/.owl/modules/kioto/"
else
    echo "  (kioto not in release — skipping)"
fi

# ── PATH setup ────────────────────────────────────────────────────────
if [ "$NO_PROFILE" != "1" ]; then
    PROFILE_FILE="$(detect_shell_profile)"

    case ":$PATH:" in
        *":$BIN_DIR:"*)
            echo ""
            echo "  ${BIN_DIR} already in PATH"
            ;;
        *)
            echo ""
            echo "  adding ${BIN_DIR} to PATH"

            if [ "$YES" != "1" ]; then
                read -r -p "  modify ${PROFILE_FILE}? [Y/n] " ans
                case "$ans" in
                    [nN]*) echo "  skipped." ; return 2 2>/dev/null || true ;;
                esac
            fi

            BACKUP="${PROFILE_FILE}.opencode-backup-$(date +%Y%m%d-%H%M%S)"
            if [ -f "$PROFILE_FILE" ]; then
                cp "$PROFILE_FILE" "$BACKUP"
            else
                touch "$PROFILE_FILE"
            fi
            echo "  backup: ${BACKUP}"

            cat >> "$PROFILE_FILE" << PATHLINE

# added by mire install script
export PATH="${BIN_DIR}:\$PATH"
PATHLINE
            echo "  updated ${PROFILE_FILE}"
            echo "  run: source ${PROFILE_FILE}"
            ;;
    esac
fi

# ── Done ──────────────────────────────────────────────────────────────
M_BIN="$BIN_DIR/mire"
O_BIN="$BIN_DIR/owl"

echo ""
echo "  ──────────────────────────────────────────────────────────────"
echo "  install complete"
echo ""
echo "  mire: ${M_BIN}"
"$M_BIN" --version 2>/dev/null || true
if [ "$NO_OWL" != "1" ] && [ -f "$O_BIN" ]; then
    echo "  owl:  ${O_BIN}"
    "$O_BIN" -V 2>/dev/null || true
fi
echo ""
echo "  try:"
echo "    mire --help"
if [ "$NO_OWL" != "1" ] && [ -f "$O_BIN" ]; then
    echo "    owl -h"
fi
