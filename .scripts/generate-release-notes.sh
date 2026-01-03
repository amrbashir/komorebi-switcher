#!/usr/bin/env bash

path="CHANGELOG.md"
out="RELEASE_NOTES.md"

is_latest_release=false
is_in_latest_release=false

# Clear output file
> "$out"

# Extract latest release notes
while IFS= read -r line; do
    # Skip [Unreleased] section
    if [[ "$line" =~ ^##[[:space:]]+\[Unreleased\] ]]; then
        continue
    # Check for release header (e.g. ## [1.0.0] - 2023-01-01)
    elif [[ "$line" =~ ^##[[:space:]]+\[ ]]; then
        # First actual release header found
        if [[ "$is_latest_release" == false ]]; then
            is_latest_release=true
            is_in_latest_release=true
        else
            # Next release header found, stop capturing
            break
        fi
    elif [[ "$is_in_latest_release" == true ]]; then
        echo "$line" >> "$out"
    fi
done < "$path"
