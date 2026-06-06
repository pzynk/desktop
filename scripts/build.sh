#!/bin/bash
set -e

# Extract current version
CURRENT_VERSION=$(node -e "console.log(require('./src-tauri/tauri.conf.json').version)")

# Prompt for new version
read -p "Enter new version (current is $CURRENT_VERSION): " VERSION

if [ -z "$VERSION" ]; then
  echo "No version entered. Keeping current version ($CURRENT_VERSION)."
  VERSION=$CURRENT_VERSION
else
  # Update version in tauri.conf.json
  echo "Updating tauri.conf.json to version $VERSION..."
  node -e "
  const fs = require('fs');
  const file = './src-tauri/tauri.conf.json';
  const data = JSON.parse(fs.readFileSync(file, 'utf8'));
  data.version = '$VERSION';
  fs.writeFileSync(file, JSON.stringify(data, null, 2), 'utf8');
  "
fi

# 1. Run the build scripts
echo "Building Debian package..."
sh ./scripts/build-deb.sh

echo "Building Windows setup..."
sh ./scripts/build-win.sh

echo "Building version: $VERSION"

# 3. Define source and destination paths
DEB_SRC="src-tauri/target/release/bundle/deb/Pzync_${VERSION}_amd64.deb"
DEB_SIG_SRC="${DEB_SRC}.sig"
WIN_SRC="src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis/Pzync_${VERSION}_x64-setup.exe"
WIN_SIG_SRC="${WIN_SRC}.sig"

DEB_DEST="release/deb/app.deb"
DEB_SIG_DEST="release/deb/app.deb.sig"
WIN_DEST="release/windows/app.exe"
WIN_SIG_DEST="release/windows/app.exe.sig"

# 4. Ensure release directories exist
mkdir -p release/deb release/windows

# 5. Copy the artifacts
echo "Copying built artifacts to release/..."
cp "$DEB_SRC" "$DEB_DEST"
cp "$DEB_SIG_SRC" "$DEB_SIG_DEST"
cp "$WIN_SRC" "$WIN_DEST"
cp "$WIN_SIG_SRC" "$WIN_SIG_DEST"

# Copy to showcase public folder for downloading
SHOWCASE_DOWNLOADS="../showcase/public/downloads"
if [ -d "$SHOWCASE_DOWNLOADS" ] || mkdir -p "$SHOWCASE_DOWNLOADS"; then
  echo "Copying to showcase downloads..."
  cp "$DEB_DEST" "$SHOWCASE_DOWNLOADS/app.deb"
  cp "$WIN_DEST" "$SHOWCASE_DOWNLOADS/app.exe"
  
  # Copy Android APK if it exists
  APK_SRC="../app/app/build/intermediates/apk/debug/app-debug.apk"
  if [ -f "$APK_SRC" ]; then
    cp "$APK_SRC" "$SHOWCASE_DOWNLOADS/app.apk"
  fi
fi

# Copy the public key for reference
if [ -f "/home/adhil/.tauri/myapp.key.pub" ]; then
  cp "/home/adhil/.tauri/myapp.key.pub" "release/public.key"
fi

# 6. Generate latest.json
DEB_SIG=$(cat "$DEB_SIG_DEST")
WIN_SIG=$(cat "$WIN_SIG_DEST")
PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat <<EOF > release/latest.json
{
  "version": "$VERSION",
  "notes": "Release v$VERSION",
  "pub_date": "$PUB_DATE",
  "platforms": {
    "linux-x86_64": {
      "signature": "$DEB_SIG",
      "url": "https://raw.githubusercontent.com/pzynk/desktop/refs/heads/main/release/deb/app.deb"
    },
    "windows-x86_64": {
      "signature": "$WIN_SIG",
      "url": "https://raw.githubusercontent.com/pzynk/desktop/refs/heads/main/release/windows/app.exe"
    },
    "darwin-x86_64": {
      "signature": "",
      "url": ""
    }
  }
}
EOF

echo "Done! Release artifacts and release/latest.json updated."
