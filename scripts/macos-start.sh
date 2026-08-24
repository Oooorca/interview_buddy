#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  echo "mac:start can only run on macOS." >&2
  exit 1
fi

for required_command in pnpm cargo xcode-select; do
  if ! command -v "$required_command" >/dev/null 2>&1; then
    echo "Missing required command: $required_command" >&2
    echo "Install Node.js + pnpm, Rust, and Xcode Command Line Tools, then retry." >&2
    exit 1
  fi
done

if ! xcode-select -p >/dev/null 2>&1; then
  echo "Xcode Command Line Tools are not configured. Run: xcode-select --install" >&2
  exit 1
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(dirname -- "$script_dir")
cd "$project_dir"

pnpm install --frozen-lockfile
pnpm exec tauri build --bundles app
open "$project_dir/src-tauri/target/release/bundle/macos/Interview Buddy.app"

echo "Interview Buddy started."
echo "On first use, allow Microphone and Screen & System Audio Recording, then restart the app."
