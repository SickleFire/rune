#!/usr/bin/env bash
set -e

REPO="SickleFire/rune"
BINARY_NAME="rune"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Rune CLI Installer ===${NC}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="x86_64-unknown-linux-gnu"
            ARCHIVE="rune-$TARGET.tar.gz"
        else
            echo -e "${RED}Unsupported Linux architecture: $ARCH${NC}"; exit 1
        fi
        ;;
    Darwin)
        if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
            TARGET="aarch64-apple-darwin"
            ARCHIVE="rune-$TARGET.tar.gz"
        else
            echo -e "${RED}Unsupported macOS architecture: $ARCH. Only Apple Silicon is currently built.${NC}"; exit 1
        fi
        ;;
    CYGWIN*|MINGW*|MSYS*)
        TARGET="x86_64-pc-windows-msvc"
        ARCHIVE="rune-$TARGET.zip"
        ;;
    *)
        echo -e "${RED}Unsupported operating system: $OS${NC}"; exit 1
        ;;
esac

# Construct download URL directly — no API call needed for latest release
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ARCHIVE"

echo -e "${BLUE}Downloading $ARCHIVE...${NC}"
TMP_DIR="$(mktemp -d)"

# Trap ensures cleanup on any exit — expected or not
trap 'rm -rf "$TMP_DIR"' EXIT

cd "$TMP_DIR"

if command -v curl >/dev/null 2>&1; then
    curl -fSL -O "$DOWNLOAD_URL"  # -f: fail on HTTP errors, -S: show errors even with -s
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOWNLOAD_URL"
else
    echo -e "${RED}Error: Neither curl nor wget is available.${NC}"; exit 1
fi

if [ ! -f "$ARCHIVE" ]; then
    echo -e "${RED}Failed to download $ARCHIVE. Do any releases exist for $REPO?${NC}"
    exit 1
fi

echo -e "${BLUE}Extracting archive...${NC}"
if [[ "$ARCHIVE" == *.tar.gz ]]; then
    tar -xzf "$ARCHIVE"
elif [[ "$ARCHIVE" == *.zip ]]; then
    if command -v unzip >/dev/null 2>&1; then
        unzip -q "$ARCHIVE"
    else
        echo -e "${RED}Error: unzip not found.${NC}"; exit 1
    fi
fi

mkdir -p "$INSTALL_DIR"

echo -e "${BLUE}Installing $BINARY_NAME to $INSTALL_DIR...${NC}"
if [ -f "$BINARY_NAME" ]; then
    cp "$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
elif [ -f "$BINARY_NAME.exe" ]; then
    cp "$BINARY_NAME.exe" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME.exe"
else
    echo -e "${RED}Binary not found in extracted archive.${NC}"; exit 1
fi

# TMP_DIR cleaned up automatically by trap

echo -e "${GREEN}=== Successfully installed Rune CLI! ===${NC}"

# Only warn about PATH if the install dir isn't already in it
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "Add ${BLUE}$INSTALL_DIR${NC} to your PATH to use rune from anywhere."
fi

echo -e "Verify with: ${GREEN}rune --version${NC}"