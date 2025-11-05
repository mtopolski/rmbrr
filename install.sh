#!/bin/sh
# Install script for rmbrr
# Usage: curl -fsSL https://raw.githubusercontent.com/mtopolski/rmbrr/main/install.sh | sh

set -e

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="linux-x86_64"
        else
            echo "Unsupported architecture: $ARCH"
            exit 1
        fi
        ;;
    Darwin*)
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="macos-x86_64"
        elif [ "$ARCH" = "arm64" ]; then
            TARGET="macos-aarch64"
        else
            echo "Unsupported architecture: $ARCH"
            exit 1
        fi
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Get latest release version
echo "Fetching latest release..."
VERSION=$(curl -fsSL https://api.github.com/repos/mtopolski/rmbrr/releases/latest | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$VERSION" ]; then
    echo "Failed to get latest version"
    exit 1
fi

echo "Installing rmbrr $VERSION for $TARGET..."

# Download binary
DOWNLOAD_URL="https://github.com/mtopolski/rmbrr/releases/download/$VERSION/rmbrr-$TARGET"
TMPFILE=$(mktemp)

if ! curl -fsSL "$DOWNLOAD_URL" -o "$TMPFILE"; then
    echo "Failed to download rmbrr"
    exit 1
fi

# Make executable
chmod +x "$TMPFILE"

# Install to /usr/local/bin (or ~/bin if no sudo)
if [ -w /usr/local/bin ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -w "$HOME/bin" ]; then
    INSTALL_DIR="$HOME/bin"
    mkdir -p "$INSTALL_DIR"
else
    echo "Cannot find writable installation directory"
    echo "Please run with sudo or create ~/bin"
    exit 1
fi

mv "$TMPFILE" "$INSTALL_DIR/rmbrr"

echo "Successfully installed rmbrr to $INSTALL_DIR/rmbrr"
echo ""
echo "Try it out:"
echo "  rmbrr --help"
