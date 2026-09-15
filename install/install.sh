#!/bin/sh
set -e

# ── Mire Toolchain Install ────────────────────────────────────────────
#
# Usage:
#   curl -fsSL <url> | sh                        # install Owl + Kioto (default)
#   curl -fsSL <url> | sh -s -- --compiler       # also install mire compiler
#   curl -fsSL <url> | sh -s -- --compiler-only   # compiler only
#   curl -fsSL <url> | sh -s -- --kioto-only      # kioto stdlib only

REPO_OWL="${OWL_REPO:-Evelynx-Dev/owl}"
REPO_COMPILER="${COMPILER_REPO:-Evelynx-Dev/Avenys-rust}"
REPO_KIOTO="${KIOTO_REPO:-Evelynx-Dev/Kioto-1}"
OWL_TARBALL="owl-linux-x86_64.tar.gz"
COMPILER_TARBALL="mire-compiler-linux-x86_64.tar.gz"
PREFIX=""
TAG_COMPILER=""
TAG_KIOTO=""
YES=0
NO_PROFILE=0
CHECK_ONLY=0
INSTALL_COMPILER=0
INSTALL_OWL=1
INSTALL_KIOTO=1
COMPILER_ONLY=0
KIOTO_ONLY=0

usage() {
    cat <<'USAGE'
Mire Toolchain Install

Usage:
  install.sh [options]

Options:
  --yes, -y             Non-interactive (skip confirmations)
  --prefix <path>        Install prefix (default: /usr/local)
  --compiler             Also install the Mire compiler
  --compiler-only        Install compiler only
  --kioto-only           Install kioto stdlib only (sets up ~/.owl/)
  --owl-only             Install owl only (no compiler, no kioto)
  --no-profile           Skip shell profile PATH modification
  --check                Check prerequisites only; never install packages
  --tag-compiler <tag>   Specific release tag (default: latest)
  --tag-kioto <tag>      Specific kioto release tag (default: latest)
  --help, -h             Show this help

Examples:
  curl https://.../install.sh | sh                     # install owl + kioto (default)
  curl https://.../install.sh | sh -s -- --compiler     # install owl + kioto + compiler
  curl https://.../install.sh | sh -s -- --compiler-only # compiler only
  curl https://.../install.sh | sh -s -- --kioto-only    # kioto only
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --yes|-y)             YES=1; shift ;;
        --prefix)              PREFIX="$2"; shift 2 ;;
        --compiler)            INSTALL_COMPILER=1; shift ;;
        --compiler-only)        COMPILER_ONLY=1; INSTALL_OWL=0; INSTALL_KIOTO=0; shift ;;
        --kioto-only)           KIOTO_ONLY=1; INSTALL_COMPILER=0; INSTALL_OWL=0; shift ;;
        --owl-only)             INSTALL_OWL=1; INSTALL_KIOTO=0; INSTALL_COMPILER=0; shift ;;
        --no-profile)          NO_PROFILE=1; shift ;;
        --check)               CHECK_ONLY=1; shift ;;
        --tag-owl)              TAG_COMPILER="$2"; shift 2 ;;
        --tag-compiler)         TAG_COMPILER="$2"; shift 2 ;;
        --tag-kioto)            TAG_KIOTO="$2"; shift 2 ;;
        --help|-h)              usage; exit 0 ;;
        --)                     shift; break ;;
        -*)                     echo "error: unknown option: $1" >&2; usage; exit 1 ;;
        *)                      echo "error: unexpected argument: $1" >&2; usage; exit 1 ;;
    esac
done

if [ -z "$PREFIX" ]; then
    PREFIX="${MIRE_PREFIX:-/usr/local}"
fi

case "$PREFIX" in
    ~/*) PREFIX="${HOME}${PREFIX#~}" ;;
    ~)   PREFIX="${HOME}" ;;
esac

BIN_DIR="${PREFIX}/bin"
LIB_DIR="${PREFIX}/lib/mire"
OWL_HOME="$HOME/.owl"

banner() {
    echo ""
    echo "┌─ Mire Toolchain Install ─────────────────────────────────────────┐"
    echo "│ prefix: ${PREFIX}"
    if [ "$KIOTO_ONLY" = "1" ]; then
        echo "│ mode:   kioto only (stdlib)"
    elif [ "$COMPILER_ONLY" = "1" ]; then
        echo "│ mode:   compiler only"
    elif [ "$INSTALL_COMPILER" = "1" ]; then
        echo "│ mode:   owl + kioto + compiler (full)"
    elif [ "$INSTALL_OWL" = "1" ] && [ "$INSTALL_KIOTO" = "1" ]; then
        echo "│ mode:   owl + kioto (default)"
    elif [ "$INSTALL_OWL" = "1" ]; then
        echo "│ mode:   owl only"
    else
        echo "│ mode:   compiler only"
    fi
    echo "└──────────────────────────────────────────────────────────────────┘"
}

banner

if [ ! -t 0 ] && [ "$YES" != "1" ] && [ "$CHECK_ONLY" != "1" ] \
    && tty -s </dev/tty 2>/dev/null; then
    exec </dev/tty
fi

needs_sudo() {
    case "$PREFIX" in
        /usr|/usr/local|/opt*|/etc*) return 0 ;;
        *) return 1 ;;
    esac
}

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
    echo "  installing: curl tar git clang llvm lld cargo libssl libsodium zlib zstd libarchive"
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
            sudo apt-get install -y -qq curl tar git clang-22 llvm-22 llvm-22-dev llvm-22-tools lld-22 cargo pkg-config libssl-dev libsodium-dev zlib1g-dev libzstd-dev libarchive-dev
            ;;
        pacman)
            sudo pacman -Sy --needed --noconfirm curl tar git clang llvm lld rust openssl libsodium zlib zstd libarchive
            ;;
        dnf|yum)
            sudo "$pm" install -y curl tar git clang llvm-devel lld rust cargo pkgconf-pkg-config openssl-devel libsodium-devel zlib-devel zstd-devel libarchive-devel
            ;;
        apk)
            sudo apk add curl tar git gcompat libgcc clang llvm-dev lld cargo openssl-dev libsodium-dev zlib-dev zstd-dev libarchive-dev
            ;;
        zypper)
            sudo zypper install -y curl tar git clang llvm-devel lld rust cargo pkg-config libopenssl-devel libsodium-devel zlib-devel libzstd-devel libarchive-devel
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
    if [ "$INSTALL_COMPILER" = "1" ] || [ "$COMPILER_ONLY" = "1" ]; then
        if ! command -v cargo >/dev/null 2>&1; then
            missing="$missing cargo"
        else
            cargo_version="$(cargo --version 2>/dev/null | sed -n 's/^cargo \([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\1 \2/p')"
            cargo_major="${cargo_version%% *}"
            cargo_minor="${cargo_version##* }"
            if [ -z "$cargo_version" ] || [ "$cargo_major" -lt 1 ] || { [ "$cargo_major" = "1" ] && [ "$cargo_minor" -lt 85 ]; }; then
                missing="$missing cargo>=1.85"
            fi
        fi
        if ! command -v clang >/dev/null 2>&1; then
            missing="$missing clang"
        fi
        llvm_config=""
        if command -v llvm-config-22 >/dev/null 2>&1; then llvm_config="llvm-config-22"; elif command -v llvm-config >/dev/null 2>&1; then llvm_config="llvm-config"; fi
        if [ -z "$llvm_config" ] || [ "$("$llvm_config" --version 2>/dev/null | cut -d. -f1)" != "22" ]; then
            missing="$missing llvm-22"
        fi
        if ! command -v llc-22 >/dev/null 2>&1 && ! command -v llc >/dev/null 2>&1; then
            missing="$missing llc-22"
        fi
        if ! command -v llvm-ar-22 >/dev/null 2>&1 && ! command -v llvm-ar >/dev/null 2>&1; then
            missing="$missing llvm-ar-22"
        fi
        if ! command -v ld.lld-22 >/dev/null 2>&1 && ! command -v ld.lld >/dev/null 2>&1; then
            missing="$missing lld-22"
        fi
        if ! command -v pkg-config >/dev/null 2>&1; then
            missing="$missing pkg-config"
        elif ! pkg-config --exists openssl libsodium zlib libzstd libarchive 2>/dev/null; then
            missing="$missing native-dev-libraries"
        fi
    fi
    if ! command -v openssl >/dev/null 2>&1; then
        missing="$missing openssl"
    fi
    if command -v ldconfig >/dev/null 2>&1 && ! ldconfig -p 2>/dev/null | grep -q 'libsodium.so'; then
        missing="$missing libsodium"
    fi
    if [ "$KIOTO_ONLY" = "1" ] && ! command -v git >/dev/null 2>&1; then
        missing="$missing git"
    fi

    if [ -z "$missing" ]; then
        return 0
    fi

    if [ "$CHECK_ONLY" = "1" ]; then
        echo "  missing:${missing}"
        return 1
    fi

    local pm
    pm="$(detect_pkg_manager)"

    if [ "$pm" = "none" ]; then
        echo ""
        echo "  warning: missing:${missing}"
        echo "  install these and re-run."
        return 1
    fi

    install_deps "$pm"
}

get_latest_tag() {
    local repo="$1"
    local tag=""
    if command -v curl >/dev/null 2>&1; then
        tag="$(curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
        # If API is rate limited, fall back to scraping releases page
        if [ -z "$tag" ]; then
            tag="$(curl -fsSL "https://github.com/${repo}/releases/latest" 2>/dev/null \
                | grep -oE '/releases/tag/v[^"]+' | head -1 | sed 's|/releases/tag/||')"
        fi
    elif command -v wget >/dev/null 2>&1; then
        tag="$(wget -qO- "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
        if [ -z "$tag" ]; then
            tag="$(wget -qO- "https://github.com/${repo}/releases/latest" 2>/dev/null \
                | grep -oE '/releases/tag/v[^"]+' | head -1 | sed 's|/releases/tag/||')"
        fi
    fi
    echo "$tag"
}

if [ -z "$TAG_COMPILER" ]; then
    TAG_COMPILER="$(get_latest_tag "$REPO_COMPILER")"
    if [ -z "$TAG_COMPILER" ]; then
        echo "  warning: could not determine latest release tag"
        echo "  set TAG_COMPILER env or use --tag-compiler"
        exit 1
    fi
fi

check_prerequisites

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

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

OWL_BIN=""
COMPILER_BIN=""

# ── Kioto only ───────────────────────────────────────────────────────
if [ "$KIOTO_ONLY" = "1" ]; then
    echo ""
    echo "  ── Installing kioto stdlib ───────────────────────────────────"
    echo ""

    if [ "$YES" != "1" ]; then
        echo "  Will install:"
        echo "    • kioto stdlib   → ${OWL_HOME}/libs/kioto/"
        echo "    • owl home       → ${OWL_HOME}/"
        echo ""
        read -r -p "  continue? [Y/n] " ans
        case "$ans" in
            [nN]*) echo "  aborted."; exit 0 ;;
        esac
    fi

    mkdir -p "${OWL_HOME}/libs" "${OWL_HOME}/tmp" "${OWL_HOME}/cfg"

    if [ ! -f "$OWL_HOME/config.toml" ]; then
        cat > "$OWL_HOME/config.toml" <<CONFIG
[owl]
version = "1.0.0"

[libs]
path = "$OWL_HOME/libs"

[download]
timeout = 30
retry = 3
CONFIG
        echo "  created ${OWL_HOME}/config.toml"
    fi

    TAG_KIOTO="${TAG_KIOTO:-$(get_latest_tag "$REPO_KIOTO")}"
    echo "  downloading kioto ${TAG_KIOTO:-latest}..."

    set -e
    cd "$TMPDIR"
    rm -rf kioto-clone
    if command -v git >/dev/null 2>&1; then
        git clone --depth 1 --branch "${TAG_KIOTO:-main}" "https://github.com/${REPO_KIOTO}" kioto-clone 2>/dev/null || \
            git clone --depth 1 "https://github.com/${REPO_KIOTO}" kioto-clone
    else
        echo "  error: git is required for kioto-only install"
        exit 1
    fi

    rm -rf "${OWL_HOME}/libs/kioto"
    cp -r kioto-clone "${OWL_HOME}/libs/kioto"

    echo "  installed kioto to ${OWL_HOME}/libs/kioto/"
    echo ""
    echo "  ─────────────────────────────────────────────────────────────────"
    echo "  kioto install complete"
    echo ""
    echo "  Owl will resolve kioto from ${OWL_HOME}/libs/kioto/"
    echo ""
    exit 0
fi

# ── Install owl + kioto ───────────────────────────────────────────────
if [ "$INSTALL_OWL" = "1" ]; then
    echo ""
    echo "  ── Installing owl ──────────────────────────────────────────────"
    echo ""

    if [ "$YES" != "1" ]; then
        echo "  Will install:"
        echo "    • owl (pm)       → ${BIN_DIR}/owl"
        if [ "$INSTALL_KIOTO" = "1" ]; then
            echo "    • kioto stdlib   → ${OWL_HOME}/libs/kioto/"
        fi
        echo "    • owl home       → ${OWL_HOME}/"
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

    OWL_URL="https://github.com/${REPO_COMPILER}/releases/download/${TAG_COMPILER}/${OWL_TARBALL}"

    echo "  downloading owl ${TAG_COMPILER}..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$OWL_URL" -o "$TMPDIR/$OWL_TARBALL"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$OWL_URL" -O "$TMPDIR/$OWL_TARBALL"
    fi

    if [ -f "$TMPDIR/$OWL_TARBALL" ] && [ -s "$TMPDIR/$OWL_TARBALL" ]; then
        echo "  extracting..."
        tar xzf "$TMPDIR/$OWL_TARBALL" -C "$TMPDIR"

        if [ -f "$TMPDIR/owl/owl" ]; then
            install_file "$TMPDIR/owl/owl" "${BIN_DIR}/owl"
            OWL_BIN="${BIN_DIR}/owl"
        fi

        if [ -d "$TMPDIR/owl/kioto" ] && [ "$INSTALL_KIOTO" = "1" ]; then
            mkdir -p "${OWL_HOME}/libs" "${OWL_HOME}/tmp" "${OWL_HOME}/cfg"
            rm -rf "${OWL_HOME}/libs/kioto"
            cp -r "$TMPDIR/owl/kioto" "${OWL_HOME}/libs/kioto"
        fi

        if [ ! -f "$OWL_HOME/config.toml" ]; then
            mkdir -p "$OWL_HOME"
            cat > "$OWL_HOME/config.toml" <<CONFIG
[owl]
version = "1.0.0"

[libs]
path = "$OWL_HOME/libs"

[download]
timeout = 30
retry = 3
CONFIG
        fi
    else
        echo "  error: could not download owl release"
        echo "  tag: ${TAG_COMPILER}"
        echo "  url: ${OWL_URL}"
        if [ "$INSTALL_COMPILER" != "1" ] && [ "$COMPILER_ONLY" != "1" ]; then
            exit 1
        fi
    fi
fi

# ── Install compiler ──────────────────────────────────────────────────
if [ "$INSTALL_COMPILER" = "1" ] || [ "$COMPILER_ONLY" = "1" ]; then
    echo ""
    echo "  ── Installing Mire compiler ──────────────────────────────────"
    echo ""

    if [ "$YES" != "1" ]; then
        echo "  Will install:"
        echo "    • Mire compiler  → ${BIN_DIR}/mire"
        echo "    • Mire runtime   → ${LIB_DIR}/"
        if needs_sudo; then
            echo ""
            echo "  sudo needed for system install."
        fi
        echo ""
        read -r -p "  continue? [Y/n] " ans
        case "$ans" in
            [nN]*) echo "  skipped compiler install." ;;
        esac
    fi

    COMPILER_URL="https://github.com/${REPO_COMPILER}/releases/download/${TAG_COMPILER}/${COMPILER_TARBALL}"

    echo "  downloading compiler ${TAG_COMPILER}..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$COMPILER_URL" -o "$TMPDIR/$COMPILER_TARBALL" 2>/dev/null || \
            curl -fsSL "https://github.com/${REPO_COMPILER}/releases/download/${TAG_COMPILER}/${COMPILER_TARBALL}" -o "$TMPDIR/$COMPILER_TARBALL"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$COMPILER_URL" -O "$TMPDIR/$COMPILER_TARBALL" 2>/dev/null || \
            wget -q "https://github.com/${REPO_COMPILER}/releases/download/${TAG_COMPILER}/${COMPILER_TARBALL}" -O "$TMPDIR/$COMPILER_TARBALL"
    fi

    if [ -f "$TMPDIR/$COMPILER_TARBALL" ] && [ -s "$TMPDIR/$COMPILER_TARBALL" ]; then
        echo "  extracting..."
        tar xzf "$TMPDIR/$COMPILER_TARBALL" -C "$TMPDIR"

        if [ -f "$TMPDIR/compiler/mire" ]; then
            install_file "$TMPDIR/compiler/mire" "${BIN_DIR}/mire"
            COMPILER_BIN="${BIN_DIR}/mire"
        fi

        if [ -d "$TMPDIR/compiler/runtime" ]; then
            install_dir "$TMPDIR/compiler/runtime"
        fi
        if [ -d "$TMPDIR/compiler/pal" ]; then
            install_dir "$TMPDIR/compiler/pal"
        fi
    else
        echo "  error: could not download compiler release"
        echo "  tag: ${TAG_COMPILER}"
        echo "  url: ${COMPILER_URL}"
        echo ""
        echo "  Build from source:"
        echo "    git clone https://github.com/${REPO_COMPILER}"
        echo "    cd ${REPO_COMPILER}"
        echo "    cargo build --release"
        echo "    sudo cp target/release/mire /usr/local/bin/mire"
        if [ "$COMPILER_ONLY" = "1" ]; then
            exit 1
        fi
    fi
fi

# ── PATH setup ────────────────────────────────────────────────────────
if [ "$NO_PROFILE" != "1" ] && [ "$COMPILER_ONLY" != "1" ]; then
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
                    [nN]*) echo "  skipped." ;;
                esac
            fi

            BACKUP="${PROFILE_FILE}.owl-backup-$(date +%Y%m%d-%H%M%S)"
            if [ -f "$PROFILE_FILE" ]; then
                cp "$PROFILE_FILE" "$BACKUP"
            else
                touch "$PROFILE_FILE"
            fi
            echo "  backup: ${BACKUP}"

            cat >> "$PROFILE_FILE" << PATHLINE

# added by Mire install script
export PATH="${BIN_DIR}:\$PATH"
PATHLINE
            echo "  updated ${PROFILE_FILE}"
            echo "  run: source ${PROFILE_FILE}"
            ;;
    esac
fi

# ── Done ──────────────────────────────────────────────────────────────
echo ""
echo "  ─────────────────────────────────────────────────────────────────"
echo "  install complete"
echo ""
if [ -n "$OWL_BIN" ] && [ -f "$OWL_BIN" ]; then
    echo "  owl:  ${OWL_BIN}"
    "$OWL_BIN" -V 2>/dev/null || true
fi
if [ -n "$COMPILER_BIN" ] && [ -f "$COMPILER_BIN" ]; then
    echo "  mire: ${COMPILER_BIN}"
    "$COMPILER_BIN" --version 2>/dev/null || true
fi
echo ""
if [ "$COMPILER_ONLY" != "1" ]; then
    echo "  try:"
    if [ -n "$OWL_BIN" ] && [ -f "$OWL_BIN" ]; then
        echo "    owl --help"
        echo "    owl new my-project"
        echo "    owl run"
    fi
    echo ""
    echo "  Need the compiler?"
    echo "    curl https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --compiler"
fi
echo ""
