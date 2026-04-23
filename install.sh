#!/bin/bash

set -e

APP_NAME="lasada"
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/$APP_NAME"

echo "Installing $APP_NAME..."

# 1. Check for Cargo
if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo is not installed. Please install Rust: https://rustup.rs/"
    exit 1
fi

# 2. Build the project
echo "Building $APP_NAME in release mode..."
cargo build --release

# 3. Create directories
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"

# 4. Install binary
echo "Installing binary to $INSTALL_DIR/$APP_NAME"
cp target/release/lasada "$INSTALL_DIR/$APP_NAME"
chmod +x "$INSTALL_DIR/$APP_NAME"

# 5. Install config
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "Installing default config to $CONFIG_DIR/config.toml"
    cp config.toml "$CONFIG_DIR/config.toml"
else
    echo "Config file already exists at $CONFIG_DIR/config.toml, skipping."
fi

# 6. Check PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "Warning: $INSTALL_DIR is not in your PATH."
    echo "Please add 'export PATH=\$PATH:$INSTALL_DIR' to your .bashrc or .zshrc"
fi

echo "Successfully installed $APP_NAME!"
echo "You can now run it by typing '$APP_NAME' (if $INSTALL_DIR is in your PATH)."
