#!/bin/bash

set -e

# Script to bump version in both Cargo.toml and pixi.toml files
# Usage: ./scripts/bump-version.sh <new_version>
# Example: ./scripts/bump-version.sh 0.2.0

if [ $# -eq 0 ]; then
    echo "Usage: $0 <new_version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

NEW_VERSION="$1"

# Validate version format (basic check for semantic versioning)
if ! echo "$NEW_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)*$'; then
    echo "Error: Version must follow semantic versioning format (e.g., 1.2.3 or 1.2.3-alpha)"
    exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -n1 | sed 's/version = "\(.*\)"/\1/')

echo "Current version: $CURRENT_VERSION"
echo "New version: $NEW_VERSION"

# Check if files exist
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found"
    exit 1
fi

if [ ! -f "pixi.toml" ]; then
    echo "Error: pixi.toml not found"
    exit 1
fi

# Update Cargo.toml
echo "Updating Cargo.toml..."
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Update pixi.toml
echo "Updating pixi.toml..."
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" pixi.toml

# Update Cargo.lock
echo "Updating Cargo.lock..."
cargo update -p fcos-ignition-coder

# Show changes
echo ""
echo "Changes made:"
echo "============="
echo "Cargo.toml:"
grep "^version = " Cargo.toml
echo "pixi.toml:"
grep "^version = " pixi.toml

# Ask for confirmation before committing
echo ""
read -p "Commit these changes? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Clean up backup files
    rm -f Cargo.toml.bak pixi.toml.bak

    # Commit changes
    git add Cargo.toml Cargo.lock pixi.toml
    git commit -m "Bump version to $NEW_VERSION"

    echo "Changes committed!"
    echo ""
    echo "To create a release, run:"
    echo "  git tag v$NEW_VERSION"
    echo "  git push origin main"
    echo "  git push origin v$NEW_VERSION"
else
    # Restore original files
    echo "Restoring original files..."
    mv Cargo.toml.bak Cargo.toml
    mv pixi.toml.bak pixi.toml
    git checkout Cargo.lock
    echo "Changes reverted."
fi
