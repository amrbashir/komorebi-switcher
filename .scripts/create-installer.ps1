# Exit if komorebi-switcher.exe is not found in ./dist
if (-not (Test-Path -Path "./dist/komorebi-switcher.exe")) {
    Write-Host -ForegroundColor Red "Error: komorebi-switcher.exe not found in ./dist, run build.ps1 first"
    exit 1
}

# Copy the komorebi-switcher.exe to the installer directory
Copy-Item -Force "./dist/komorebi-switcher.exe" "./installer/komorebi-switcher.exe"

# Copy the icon.ico to the installer directory
Copy-Item -Force "./assets/icon.ico" "./installer/icon.ico"

# Create the installer
makensis /V4 "./installer/installer.nsi"

# Move the installer to the dist directory
Move-Item -Force "./installer/komorebi-switcher-setup.exe" "./dist/komorebi-switcher-setup.exe"

# Remove artifacts
Remove-Item -Force "./installer/komorebi-switcher.exe"
Remove-Item -Force "./installer/icon.ico"
