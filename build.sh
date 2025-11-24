## How to Build ##

#!/usr/bin/env bash
set -e

# Build release binary
cargo build --release

# Prepare package directories
PKGDIR="nocturne-notes-deb"
mkdir -p $PKGDIR/DEBIAN \
         $PKGDIR/usr/bin \
         $PKGDIR/usr/share/applications \
         $PKGDIR/usr/share/icons/hicolor/scalable/apps

# Copy binary (rename to match desktop Exec line)
cp target/release/nocturne_notes $PKGDIR/usr/bin/nocturne-notes

# Copy desktop entry and icon
cp nocturne-notes.desktop $PKGDIR/usr/share/applications/
cp nocturne-notes.svg $PKGDIR/usr/share/icons/hicolor/scalable/apps/nocturne-notes.svg

# Write control file
cat > $PKGDIR/DEBIAN/control <<EOF
Package: nocturne-notes
Version: 1.5.0
Section: utils
Priority: optional
Architecture: amd64
Depends: libc6, libgtk-4-1
Maintainer: JEGLY <globalcve@gmail.com>
Description: Secure GTK4 note-taking app written in Rust
 A simple, secure note-taking application with a Rust backend and GTK4 frontend.
EOF

# Build the .deb
dpkg-deb --build $PKGDIR

echo "Package built: ${PKGDIR}.deb"

# Usage:
# chmod +x build.sh
# ./build.sh
# sudo apt install ./nocturne-notes-deb.deb
