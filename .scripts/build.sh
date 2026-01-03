# Build the project
cargo build --release

# Create the dist directory if it doesn't exist
mkdir -p ./dist

# Copy the komorebi-switcher to the dist directory
targetDir=${CARGO_TARGET_DIR:-./target}
cp -f "$targetDir/release/komorebi-switcher" "./dist/komorebi-switcher"