#!/usr/bin/env bash
set -e

# Rune Installer Script
# Downloads and installs the latest release of Rune CLI from GitHub.

REPO="SickleFire/rune"
BINARY_NAME="rune"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Rune CLI Installer ===${NC}"

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="x86_64-unknown-linux-gnu"
            ARCHIVE="rune-$TARGET.tar.gz"
        else
            echo -e "${RED}Unsupported Linux architecture: $ARCH${NC}"
            exit 1
        fi
        ;;
    Darwin)
        if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
            TARGET="aarch64-apple-darwin"
            ARCHIVE="rune-$TARGET.tar.gz"
        else
            echo -e "${RED}Unsupported macOS architecture: $ARCH. Only Apple Silicon (aarch64) is currently built in releases.${NC}"
            exit 1
        fi
        ;;
    CYGWIN*|MINGW*|MSYS*)
        TARGET="x86_64-pc-windows-msvc"
        ARCHIVE="rune-$TARGET.zip"
        ;;
    *)
        echo -e "${RED}Unsupported operating system: $OS${NC}"
        exit 1
        ;;
esac

# Get latest release tag or download URL from GitHub API
echo -e "${BLUE}Fetching latest release info from GitHub...${NC}"
API_URL="https://api.github.com/repos/$REPO/releases/latest"

if command -v curl >/dev/null 2>&1; then
    RELEASE_JSON=$(curl -s "$API_URL")
elif command -v wget >/dev/null 2>&1; then
    RELEASE_JSON=$(wget -qO- "$API_URL")
else
    echo -e "${RED}Error: Neither curl nor wget is available.${NC}"
    exit 1
fi

# Extract download URL for the appropriate archive using grep/sed or python/node if available, or fetch direct tag download
# Simple robust fallback: construct download url using latest tag or tags/latest
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ARCHIVE"

echo -e "${BLUE}Downloading $ARCHIVE from $DOWNLOAD_URL...${NC}"
TMP_DIR="$(mktemp -d)"
cd "$TMP_DIR"

if command -v curl >/dev/null 2>&1; then
    curl -sL -O "$DOWNLOAD_URL"
else
    wget -q "$DOWNLOAD_URL"
fi

if [ ! -f "$ARCHIVE" ]; then
    echo -e "${RED}Failed to download $ARCHIVE. Please check if releases exist on GitHub for $REPO.${NC}"
    rm -rf "$TMP_DIR"
    exit 1
fi

echo -e "${BLUE}Extracting archive...${NC}"
if [[ "$ARCHIVE" == *.tar.gz ]]; then
    tar -xzf "$ARCHIVE"
elif [[ "$ARCHIVE" == *.zip ]]; then
    if command -v unzip >/dev/null 2>&1; then
        unzip -q "$ARCHIVE"
    else
        echo -e "${RED}Error: unzip command not found.${NC}"
        rm -rf "$TMP_DIR"
        exit 1
    fi
fi

# Create install directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

echo -e "${BLUE}Installing $BINARY_NAME to $INSTALL_DIR...${NC}"
if [ -f "$BINARY_NAME" ]; then
    cp "$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
elif [ -f "$BINARY_NAME.exe" ]; then
    cp "$BINARY_NAME.exe" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME.exe"
else
    echo -e "${RED}Binary not found in archive extracted contents.${NC}"
    rm -rf "$TMP_DIR"
    exit 1
fi

rm -rf "$TMP_DIR"

echo -e "${GREEN}=== Successfully installed Rune CLI! ===${NC}"
echo -e "Make sure ${BLUE}$INSTALL_DIR${NC} is in your PATH."
echo -e "You can verify the installation by running: ${GREEN}rune --version${NC}"
