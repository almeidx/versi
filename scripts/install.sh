#!/bin/bash

set -e

OS="$(uname -s)"

case "$OS" in
    Darwin)
        APP_NAME="Versi.app"
        INSTALL_DIR="/Applications"

        # Find the app bundle - check current directory and script directory
        SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
        APP_PATH=""

        if [ -d "./$APP_NAME" ]; then
            APP_PATH="./$APP_NAME"
        elif [ -d "$SCRIPT_DIR/$APP_NAME" ]; then
            APP_PATH="$SCRIPT_DIR/$APP_NAME"
        else
            echo "Error: Cannot find $APP_NAME"
            echo "Please run this script from the directory containing $APP_NAME"
            exit 1
        fi

        echo "Installing Versi..."

        # Remove quarantine attribute
        echo "Removing quarantine attribute..."
        xattr -cr "$APP_PATH"

        # Check if already installed
        if [ -d "$INSTALL_DIR/$APP_NAME" ]; then
            echo "Existing installation found. Replacing..."
            rm -rf "$INSTALL_DIR/$APP_NAME"
        fi

        # Move to Applications
        echo "Moving to $INSTALL_DIR..."
        mv "$APP_PATH" "$INSTALL_DIR/"

        echo ""
        echo "Installation complete!"
        echo "You can now launch Versi from your Applications folder."
        ;;
    *)
        echo "Error: This installer only supports macOS."
        echo "Detected OS: $OS"
        exit 1
        ;;
esac
