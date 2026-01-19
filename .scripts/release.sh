#!/bin/bash

version="$1"

path="Cargo.toml"
sed -i -E "s/version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$version\"/" "$path"

cargo update -p komorebi-switcher # update the lock file

path="installer/installer.nsi"
sed -i -E "s/VERSION \"[0-9]+\.[0-9]+\.[0-9]+\"/VERSION \"$version\"/" "$path"

path="installer/Info.plist"
sed -i -E "s/<string>[0-9]+\.[0-9]+\.[0-9]+<\/string>/<string>$version<\/string>/" "$path"

path="CHANGELOG.md"
date=$(date +%Y-%m-%d)
sed -i "s/## \[Unreleased\]/## [Unreleased]\n\n## [$version] - $date/" "$path"

git add .
git commit -m "release: v$version"
git push
git tag "v$version"
git push --tags
