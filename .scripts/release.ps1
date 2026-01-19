$version = $args[0];

$path = "Cargo.toml"
(Get-Content $path) -replace "version = `"[0-9]+.[0-9]+.[0-9]+`"", "version = `"$version`"" | Set-Content $path

cargo update -p komorebi-switcher # update the lock file

$path = "installer/installer.nsi"
(Get-Content $path) -replace "VERSION `"[0-9]+.[0-9]+.[0-9]+`"", "VERSION `"$version`"" | Set-Content $path

$path = "installer/Info.plist"
(Get-Content $path) -replace "<string>[0-9]+.[0-9]+.[0-9]+</string>", "<string>$version</string>" | Set-Content $path

$path = "CHANGELOG.md"
$date = Get-Date -Format "yyyy-MM-dd"
(Get-Content $path) -replace "## \[Unreleased\]", "## [Unreleased]`n`n## [$version] - $date" | Set-Content $path

git add .
git commit -m "release: v$version"
git push
git tag "v$version"
git push --tags
