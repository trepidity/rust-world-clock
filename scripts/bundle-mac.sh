#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

APP_NAME="WorldClock"
BINARY="rust-world-clock"
IDENTIFIER="com.jaredljennings.worldclock"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
APP_DIR="dist/${APP_NAME}.app"
ICON_SRC="assets/app-icon.svg"
ICON_ICNS="assets/app-icon.icns"

# --- Build ---
echo "==> Building release binary..."
cargo build --release

# --- Generate .icns icon ---
if [ ! -f "${ICON_ICNS}" ] || [ "${ICON_SRC}" -nt "${ICON_ICNS}" ]; then
  echo "==> Generating icon..."
  WORK=$(mktemp -d)
  trap 'rm -rf "$WORK"' EXIT

  qlmanage -t -s 512 -o "$WORK" "$ICON_SRC" 2>/dev/null || true
  SRC_PNG="$WORK/$(basename "$ICON_SRC").png"
  if [ ! -f "$SRC_PNG" ]; then
    echo "ERROR: qlmanage failed to render ${ICON_SRC}" >&2
    exit 1
  fi

  ICONSET="$WORK/icon.iconset"
  mkdir "$ICONSET"
  sips -z 16   16   "$SRC_PNG" --out "$ICONSET/icon_16x16.png"      >/dev/null
  sips -z 32   32   "$SRC_PNG" --out "$ICONSET/icon_16x16@2x.png"   >/dev/null
  sips -z 32   32   "$SRC_PNG" --out "$ICONSET/icon_32x32.png"      >/dev/null
  sips -z 64   64   "$SRC_PNG" --out "$ICONSET/icon_32x32@2x.png"   >/dev/null
  sips -z 128  128  "$SRC_PNG" --out "$ICONSET/icon_128x128.png"    >/dev/null
  sips -z 256  256  "$SRC_PNG" --out "$ICONSET/icon_128x128@2x.png" >/dev/null
  sips -z 256  256  "$SRC_PNG" --out "$ICONSET/icon_256x256.png"    >/dev/null
  sips -z 512  512  "$SRC_PNG" --out "$ICONSET/icon_256x256@2x.png" >/dev/null
  cp "$SRC_PNG" "$ICONSET/icon_512x512.png"
  cp "$SRC_PNG" "$ICONSET/icon_512x512@2x.png"

  iconutil -c icns "$ICONSET" -o "$ICON_ICNS"
  trap - EXIT
  rm -rf "$WORK"
  echo "==> Icon: ${ICON_ICNS}"
fi

# --- Assemble .app bundle ---
echo "==> Assembling ${APP_NAME}.app..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

cp "target/release/${BINARY}" "$APP_DIR/Contents/MacOS/${BINARY}"
cp "$ICON_ICNS" "$APP_DIR/Contents/Resources/app-icon.icns"

cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key><string>World Clock</string>
  <key>CFBundleIdentifier</key><string>${IDENTIFIER}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleExecutable</key><string>${BINARY}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>app-icon</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
</dict>
</plist>
PLIST

echo "==> Done: ${APP_DIR}"
echo ""
echo "Run now:         open ${APP_DIR}"
echo "Install:         cp -r ${APP_DIR} /Applications/"
