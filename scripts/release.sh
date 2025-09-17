#!/bin/bash

# Script to create a new release with semantic versioning
# Usage: ./scripts/release.sh <bump_type>
# Example: ./scripts/release.sh patch|minor|major

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <bump_type>"
    echo "  patch - Bug fixes (0.1.0 -> 0.1.1)"
    echo "  minor - New features (0.1.0 -> 0.2.0)"
    echo "  major - Breaking changes (0.1.0 -> 1.0.0)"
    exit 1
fi

BUMP_TYPE=$1

# Check if we're in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Error: Not in a git repository"
    exit 1
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "Error: You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' bot/Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $CURRENT_VERSION"

# Parse version components
IFS='.' read -r -a VERSION_PARTS <<< "$CURRENT_VERSION"
MAJOR=${VERSION_PARTS[0]}
MINOR=${VERSION_PARTS[1]}
PATCH=${VERSION_PARTS[2]}

# Bump version based on type
case $BUMP_TYPE in
    "patch")
        PATCH=$((PATCH + 1))
        ;;
    "minor")
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    "major")
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    *)
        echo "Error: Invalid bump type. Use patch, minor, or major."
        exit 1
        ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"
TAG="v$NEW_VERSION"

echo "Bumping $BUMP_TYPE: $CURRENT_VERSION -> $NEW_VERSION"

# Update version in Cargo.toml
echo "Updating version to $NEW_VERSION in bot/Cargo.toml..."
sed -i.bak "0,/^version = /s/^version = \".*\"/version = \"$NEW_VERSION\"/" bot/Cargo.toml
rm bot/Cargo.toml.bak

# Update Cargo.lock
echo "Updating Cargo.lock..."
cd bot && cargo update && cd ..

# Commit version bump
echo "Committing version bump..."
git add bot/Cargo.toml Cargo.lock
git commit -m "Bump version to $NEW_VERSION"

# Create and push tag
echo "Creating and pushing tag $TAG..."
git tag -a "$TAG" -m "Release $TAG"
git push origin main
git push origin "$TAG"

echo "✅ Release $TAG created successfully!"
echo "GitHub Actions will build and create the release automatically."
echo "Check: https://github.com/$(git remote get-url origin | sed 's/.*github.com[/:]\([^/]*\/[^.]*\).*/\1/')/actions"
