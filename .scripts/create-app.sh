# Exit if komorebi-switcher is not found in ./dist
if [ ! -f "./dist/komorebi-switcher" ]; then
    echo "\033[0;31mError: komorebi-switcher not found in ./dist, run build.sh first\033[0m"
    exit 1
fi

# Remove existing .app if it exists
if [ -d "./dist/komorebi-switcher.app" ]; then
    rm -rf "./dist/komorebi-switcher.app"
fi

# Create the .app bundle structure
mkdir -p ./dist/komorebi-switcher.app/Contents/MacOS

# Copy the komorebi-switcher binary into the .app bundle
cp ./dist/komorebi-switcher ./dist/komorebi-switcher.app/Contents/MacOS/komorebi-switcher

# Convert PNG to ICNS and move to Resources
mkdir -p ./dist/komorebi-switcher.app/Contents/Resources
mkdir icon.iconset
sips -z 16 16     ./assets/icon.png --out icon.iconset/icon_16x16.png
sips -z 32 32     ./assets/icon.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32     ./assets/icon.png --out icon.iconset/icon_32x32.png
sips -z 64 64     ./assets/icon.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128   ./assets/icon.png --out icon.iconset/icon_128x128.png
sips -z 256 256   ./assets/icon.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256   ./assets/icon.png --out icon.iconset/icon_256x256.png
sips -z 512 512   ./assets/icon.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512   ./assets/icon.png --out icon.iconset/icon_512x512.png
sips -z 1024 1024 ./assets/icon.png --out icon.iconset/icon_512x512@2x.png
iconutil -c icns icon.iconset
mv icon.icns ./dist/komorebi-switcher.app/Contents/Resources/
rm -rf icon.iconset

# Make the binary executable
chmod +x ./dist/komorebi-switcher.app/Contents/MacOS/komorebi-switcher

# Copy the Info.plist file into the .app bundle
cp ./installer/Info.plist ./dist/komorebi-switcher.app/Contents/Info.plist

# Touch the .app to update its info
touch ./dist/komorebi-switcher.app