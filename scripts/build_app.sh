#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Extract version from Cargo.toml
VERSION=$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo "==> Building Koing v${VERSION}"

# Build release binary
echo "==> cargo build --release"
(cd "$PROJECT_DIR" && cargo build --release)

# Paths
APP_NAME="Koing.app"
APP_DIR="$PROJECT_DIR/$APP_NAME"
CONTENTS="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS/MacOS"
RESOURCES_DIR="$CONTENTS/Resources"
BINARY="$PROJECT_DIR/target/release/koing"
ZIP_NAME="Koing-${VERSION}.zip"

# Clean previous build
rm -rf "$APP_DIR"
rm -f "$PROJECT_DIR/$ZIP_NAME"

# Create .app bundle structure
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

# Copy binary
cp "$BINARY" "$MACOS_DIR/koing"
chmod +x "$MACOS_DIR/koing"

# Copy Info.plist with version substitution
sed "s/__VERSION__/${VERSION}/g" "$PROJECT_DIR/resources/Info.plist" > "$CONTENTS/Info.plist"

# Copy app icon
if [ -f "$PROJECT_DIR/resources/AppIcon.icns" ]; then
    cp "$PROJECT_DIR/resources/AppIcon.icns" "$RESOURCES_DIR/AppIcon.icns"
    echo "==> Copied AppIcon.icns to Resources/"
fi

# Copy data directory (ngram model)
if [ -d "$PROJECT_DIR/data" ]; then
    cp -R "$PROJECT_DIR/data" "$RESOURCES_DIR/data"
    echo "==> Copied data/ to Resources/"
fi

echo "==> Created $APP_NAME"

# Create zip for distribution
(cd "$PROJECT_DIR" && zip -r -y "$ZIP_NAME" "$APP_NAME")
echo "==> Created $ZIP_NAME"

# Print SHA256 for Homebrew Cask
SHA=$(shasum -a 256 "$PROJECT_DIR/$ZIP_NAME" | awk '{print $1}')
echo "==> SHA256: $SHA"
