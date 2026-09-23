#!/bin/sh
set -e

echo "=== Amber NPM Package Publisher ==="

if ! npm whoami >/dev/null 2>&1; then
  echo "Error: You are not logged in to npm."
  echo "Please run 'npm login' first, then re-run this script."
  exit 1
fi

user=$(npm whoami)
echo "Logged in as: $user"

echo "\n--- 1. Publishing @amberjs/types ---"
cd packages/types
npm publish --access public || echo "@amberjs/types publish failed or already published"
cd ../..

echo "\n--- 2. Publishing amberjs ---"
cd packages/amberjs
npm publish --access public || echo "amberjs publish failed or already published"
cd ../..

echo "\n✅ NPM publishing finished!"
