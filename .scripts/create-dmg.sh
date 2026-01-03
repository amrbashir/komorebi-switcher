# Exit if komorebi-switcher is not found in ./dist
if [ ! -d "./dist/komorebi-switcher.app" ]; then
    echo "\033[0;31mError: komorebi-switcher.app not found in ./dist, run create-app.sh first\033[0m"
    exit 1
fi

# Remove existing DMG if it exists
if [ -f "./dist/komorebi-switcher.dmg" ]; then
    rm "./dist/komorebi-switcher.dmg"
fi

# Create DMG
create-dmg \
  --volname "komorebi-switcher" \
  --volicon "./dist/komorebi-switcher.app/Contents/Resources/icon.icns" \
  --icon "komorebi-switcher.app" 100 100 \
  --app-drop-link 400 100 \
  --skip-jenkins \
  "./dist/komorebi-switcher.dmg" \
  "./dist/komorebi-switcher.app"

# Touch the DMG to update its info
touch ./dist/komorebi-switcher.dmg