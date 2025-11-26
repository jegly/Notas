#!/bin/bash
# Download DotGothic16 font for Notas

FONT_DIR="$HOME/.local/share/fonts"
FONT_FILE="$FONT_DIR/DotGothic16-Regular.ttf"

mkdir -p "$FONT_DIR"

echo "Downloading DotGothic16-Regular.ttf..."

if wget -q "https://fonts.gstatic.com/s/dotgothic16/v17/v6-QGYjBJFKgyw5nSOqouUENAxX4vSRk.ttf" -O "$FONT_FILE" 2>/dev/null; then
    echo "✓ Downloaded from Google Fonts"
elif curl -sL "https://fonts.gstatic.com/s/dotgothic16/v17/v6-QGYjBJFKgyw5nSOqouUENAxX4vSRk.ttf" -o "$FONT_FILE" 2>/dev/null; then
    echo "✓ Downloaded from Google Fonts (curl)"
else
    echo "❌ Failed to download. Please download manually:"
    echo "   https://fonts.google.com/specimen/DotGothic16"
    exit 1
fi

# Update font cache
if [ -x /usr/bin/fc-cache ]; then
    fc-cache -f "$FONT_DIR"
fi

echo "✓ Font installed to: $FONT_FILE"
echo "✓ Restart Notas to apply the font"
