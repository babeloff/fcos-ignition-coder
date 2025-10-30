#!/usr/bin/env nu

# Script to bump version in both Cargo.toml and pixi.toml files
# Usage: nu scripts/bump-version.nu <new_version>
# Example: nu scripts/bump-version.nu 0.2.0

def main [new_version?: string] {
    if ($new_version | is-empty) {
        print "Usage: nu scripts/bump-version.nu <new_version>"
        print "Example: nu scripts/bump-version.nu 0.2.0"
        exit 1
    }

    # Validate version format (basic check for semantic versioning)
    let version_parts = $new_version | split row "."
    if ($version_parts | length) < 3 {
        print $"Error: Version must follow semantic versioning format \(e.g., 1.2.3 or 1.2.3-alpha\)"
        exit 1
    }

    # Check if first three parts are numbers
    try {
        let major = $version_parts | get 0 | into int
        let minor = $version_parts | get 1 | into int
        let patch_full = $version_parts | get 2
        let patch = $patch_full | split row "-" | get 0 | into int
    } catch {
        print $"Error: Version must follow semantic versioning format \(e.g., 1.2.3 or 1.2.3-alpha\)"
        exit 1
    }

    # Check if files exist
    if not ("Cargo.toml" | path exists) {
        print "Error: Cargo.toml not found"
        exit 1
    }

    if not ("pixi.toml" | path exists) {
        print "Error: pixi.toml not found"
        exit 1
    }

    # Get current version from Cargo.toml
    let cargo_content = open Cargo.toml --raw | decode utf-8
    let current_version_line = $cargo_content | lines | where ($it | str starts-with "version = ") | first
    let current_version = $current_version_line | str replace 'version = "' '' | str replace '"' ''

    print $"Current version: ($current_version)"
    print $"New version: ($new_version)"

    # Create backup copies
    cp Cargo.toml Cargo.toml.bak
    cp pixi.toml pixi.toml.bak

    try {
        # Update Cargo.toml
        print "Updating Cargo.toml..."
        let updated_cargo = $cargo_content | str replace $'version = "($current_version)"' $'version = "($new_version)"'
        $updated_cargo | save --force Cargo.toml

        # Update pixi.toml
        print "Updating pixi.toml..."
        let pixi_content = open pixi.toml --raw | decode utf-8
        let updated_pixi = $pixi_content | str replace $'version = "($current_version)"' $'version = "($new_version)"'
        $updated_pixi | save --force pixi.toml

        # Update Cargo.lock
        print "Updating Cargo.lock..."
        ^cargo update -p fcos-ignition-coder

        # Show changes
        print ""
        print "Changes made:"
        print "============="
        print "Cargo.toml:"
        let new_cargo_version = open Cargo.toml --raw | decode utf-8 | lines | where ($it | str starts-with "version = ") | first
        print $new_cargo_version
        print "pixi.toml:"
        let new_pixi_version = open pixi.toml --raw | decode utf-8 | lines | where ($it | str starts-with "version = ") | first
        print $new_pixi_version

        # Ask for confirmation before committing
        print ""
        let confirm = input "Commit these changes? (y/N): "

        if ($confirm | str downcase) in ["y" "yes"] {
            # Clean up backup files
            rm Cargo.toml.bak pixi.toml.bak

            # Commit changes
            ^git add Cargo.toml Cargo.lock pixi.toml
            ^git commit -m $"Bump version to ($new_version)"

            print "Changes committed!"
            print ""
            print "To create a release, run:"
            print $"  git tag v($new_version)"
            print "  git push origin main"
            print $"  git push origin v($new_version)"
        } else {
            # Restore original files
            print "Restoring original files..."
            mv Cargo.toml.bak Cargo.toml
            mv pixi.toml.bak pixi.toml
            ^git checkout Cargo.lock
            print "Changes reverted."
        }
    } catch { |e|
        # Restore original files on error
        print $"Error occurred: ($e.msg)"
        print "Restoring original files..."
        if ("Cargo.toml.bak" | path exists) { mv Cargo.toml.bak Cargo.toml }
        if ("pixi.toml.bak" | path exists) { mv pixi.toml.bak pixi.toml }
        ^git checkout Cargo.lock
        print "Changes reverted."
        exit 1
    }
}
