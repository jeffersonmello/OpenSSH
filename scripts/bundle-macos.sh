#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script requires macOS." >&2
  exit 1
fi
if [[ "$#" -gt 1 ]]; then
  echo "Usage: $0 [aarch64-apple-darwin|x86_64-apple-darwin]" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
BUILD_ARGS=(--release --locked)
BINARY="target/release/openssh-manager"
if [[ "$#" -eq 1 ]]; then
  case "$1" in
    aarch64-apple-darwin|x86_64-apple-darwin) ;;
    *) echo "Unsupported macOS target: $1" >&2; exit 1 ;;
  esac
  BUILD_ARGS+=(--target "$1")
  BINARY="target/$1/release/openssh-manager"
fi

cargo build "${BUILD_ARGS[@]}"
VERSION="$(sed -nE 's/^version = "([^"]+)"/\1/p' Cargo.toml | head -n 1)"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][A-Za-z0-9.-]+)?$ ]]; then
  echo "Could not determine a valid package version." >&2
  exit 1
fi

APP="dist/OpenSSH Manager.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
# Replace the inode instead of truncating an executable mapped by an open app.
cp "$BINARY" "$APP/Contents/MacOS/openssh-manager.next"
mv -f "$APP/Contents/MacOS/openssh-manager.next" "$APP/Contents/MacOS/openssh-manager"
cp LICENSE "$APP/Contents/Resources/LICENSE"

ICONSET="dist/OpenSSH.iconset"
if [[ -f assets/brand/logo.png ]]; then
  mkdir -p "$ICONSET"
  for SIZE in 16 32 128 256 512; do
    sips -z "$SIZE" "$SIZE" assets/brand/logo.png --out "$ICONSET/icon_${SIZE}x${SIZE}.png" >/dev/null
    DOUBLE=$((SIZE * 2))
    sips -z "$DOUBLE" "$DOUBLE" assets/brand/logo.png --out "$ICONSET/icon_${SIZE}x${SIZE}@2x.png" >/dev/null
  done
  iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/OpenSSH.icns"
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>OpenSSH Manager</string>
  <key>CFBundleDisplayName</key><string>OpenSSH Manager</string>
  <key>CFBundleIdentifier</key><string>io.github.jeffersonmello.openssh-manager</string>
  <key>CFBundleExecutable</key><string>openssh-manager</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleIconFile</key><string>OpenSSH.icns</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>Copyright 2026 Jefferson Mello Olynyki. MIT License.</string>
</dict></plist>
PLIST

plutil -lint "$APP/Contents/Info.plist"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"
echo "Created $APP (ad hoc signature; not notarized)."
