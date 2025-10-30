#!/usr/bin/env nu

# Complete release script for fcos-ignition-coder
# Usage: nu scripts/release.nu <new_version>
# Example: nu scripts/release.nu 0.2.0

def main [new_version?: string] {
    if ($new_version | is-empty) {
        print "Usage: nu scripts/release.nu <new_version>"
        print "Example: nu scripts/release.nu 0.2.0"
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

    print $"🚀 Starting release process for version ($new_version)"
    print ""

    # Step 1: Check git status
    print "📋 Checking git status..."
    let git_status = ^git status --porcelain | str trim
    if ($git_status | is-not-empty) {
        print "❌ Working directory is not clean. Please commit or stash changes first."
        ^git status
        exit 1
    }
    print "✅ Working directory is clean"

    # Step 2: Check current branch
    let current_branch = ^git branch --show-current | str trim
    if $current_branch != "main" {
        print $"⚠️  You are on branch '($current_branch)'. It's recommended to release from 'main' branch."
        let confirm = input "Continue anyway? (y/N): "
        if not (($confirm | str downcase) in ["y" "yes"]) {
            print "Release cancelled."
            exit 1
        }
    }

    # Step 3: Run tests
    print "🧪 Running tests..."
    try {
        ^cargo test
    } catch {
        print "❌ Tests failed. Please fix issues before releasing."
        exit 1
    }
    print "✅ All tests passed"

    # Step 4: Build project
    print "🔨 Building project..."
    try {
        ^cargo build --release
    } catch {
        print "❌ Build failed. Please fix issues before releasing."
        exit 1
    }
    print "✅ Build successful"

    # Step 5: Update version numbers
    print $"📝 Updating version to ($new_version)..."

    # Check if files exist
    if not ("Cargo.toml" | path exists) {
        print "Error: Cargo.toml not found"
        exit 1
    }

    if not ("pixi.toml" | path exists) {
        print "Error: pixi.toml not found"
        exit 1
    }

    # Get current version
    let cargo_content = open Cargo.toml --raw | decode utf-8
    let current_version_line = $cargo_content | lines | where ($it | str starts-with "version = ") | first
    let current_version = $current_version_line | str replace 'version = "' '' | str replace '"' ''
    print $"Current version: ($current_version) → New version: ($new_version)"

    # Update Cargo.toml
    let updated_cargo = $cargo_content | str replace $'version = "($current_version)"' $'version = "($new_version)"'
    $updated_cargo | save --force Cargo.toml

    # Update pixi.toml
    let pixi_content = open pixi.toml --raw | decode utf-8
    let updated_pixi = $pixi_content | str replace $'version = "($current_version)"' $'version = "($new_version)"'
    $updated_pixi | save --force pixi.toml

    # Update Cargo.lock
    ^cargo update -p fcos-ignition-coder

    print "✅ Version files updated"

    # Step 6: Commit version changes
    print "💾 Committing version changes..."
    ^git add Cargo.toml Cargo.lock pixi.toml
    ^git commit -m $"Bump version to ($new_version)"
    print "✅ Version changes committed"

    # Step 7: Create git tag
    print $"🏷️  Creating git tag v($new_version)..."
    try {
        ^git tag $"v($new_version)"
    } catch {
        print $"❌ Failed to create tag. Tag v($new_version) might already exist."
        exit 1
    }
    print "✅ Git tag created"

    # Step 8: Push changes
    print "📤 Pushing changes to remote..."
    try {
        ^git push origin $current_branch
    } catch {
        print "❌ Failed to push changes to remote"
        exit 1
    }
    print "✅ Changes pushed to remote"

    # Step 9: Push tag
    print "📤 Pushing tag to remote..."
    try {
        ^git push origin $"v($new_version)"
    } catch {
        print "❌ Failed to push tag to remote"
        exit 1
    }
    print "✅ Tag pushed to remote"

    # Step 10: Success message
    print ""
    print "🎉 Release process completed successfully!"
    print ""
    print $"📦 Version ($new_version) has been released"
    print "🔗 GitHub Actions will now build and create the release automatically"
    print $"📋 Check the progress at: https://github.com/(^git remote get-url origin | str replace 'git@github.com:' '' | str replace '.git' '')/actions"
    print ""
    print "Next steps:"
    print "  1. Monitor the GitHub Actions workflow"
    print "  2. Edit the release notes on GitHub when ready"
    print "  3. Announce the release to users"
}
