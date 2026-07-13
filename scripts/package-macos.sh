#!/usr/bin/env bash
# Build Boinc.app and a .dmg (macOS only; run by the release workflow).
# Signing: set APPLE_SIGNING_IDENTITY to codesign the bundle. Notarization
# additionally needs an App Store Connect API key wired into CI — tracked as
# plan.md risk #3 and not yet automated.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
APP=dist/Boinc.app

cargo build --release -p boinc-app -p boinc-cli

rm -rf dist
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/boinc-app "$APP/Contents/MacOS/"
cp target/release/boinc "$APP/Contents/MacOS/"
cp assets/boinc.png "$APP/Contents/Resources/"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key><string>com.hideterms.boinc</string>
    <key>CFBundleName</key><string>Boinc</string>
    <key>CFBundleDisplayName</key><string>Boinc</string>
    <key>CFBundleExecutable</key><string>boinc-app</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
    codesign --force --deep --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$APP"
    echo "signed with $APPLE_SIGNING_IDENTITY"
else
    echo "APPLE_SIGNING_IDENTITY not set; bundle is unsigned (Gatekeeper will warn)"
fi

hdiutil create "dist/Boinc-${VERSION}.dmg" -volname "Boinc" -srcfolder "$APP" -ov -format UDZO
echo "built dist/Boinc-${VERSION}.dmg"
