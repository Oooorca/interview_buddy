#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  echo "mac:start can only run on macOS." >&2
  exit 1
fi

for required_command in pnpm cargo xcode-select security codesign tccutil; do
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

app_path="$project_dir/src-tauri/target/release/bundle/macos/Interview Buddy.app"
bundle_id="com.oooorca.interview-buddy"
previous_requirement=""
if [ -d "$app_path" ]; then
  previous_requirement=$(codesign -d -r- "$app_path" 2>&1 || true)
fi

signing_identity=${APPLE_SIGNING_IDENTITY:-}
if [ -z "$signing_identity" ]; then
  signing_identity=$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -nE 's/.*"((Developer ID Application|Apple Development):[^"]+)".*/\1/p' \
    | head -n 1 || true)
fi
if [ -n "$signing_identity" ]; then
  export APPLE_SIGNING_IDENTITY="$signing_identity"
  echo "Using stable macOS signing identity: $signing_identity"
  ad_hoc_build=false
else
  export APPLE_SIGNING_IDENTITY="-"
  echo "No Apple code-signing identity found; using an ad-hoc local signature."
  ad_hoc_build=true
fi

if pgrep -x interview_buddy >/dev/null 2>&1; then
  pkill -x interview_buddy
fi

pnpm install --frozen-lockfile
pnpm exec tauri build --bundles app

current_requirement=$(codesign -d -r- "$app_path" 2>&1 || true)
if [ "$ad_hoc_build" = true ] && [ "$current_requirement" != "$previous_requirement" ]; then
  echo "Ad-hoc code identity changed; clearing stale macOS privacy approvals."
  tccutil reset ScreenCapture "$bundle_id" >/dev/null 2>&1 || true
  tccutil reset Microphone "$bundle_id" >/dev/null 2>&1 || true
fi

open "$app_path"

started=false
attempt=0
while [ "$attempt" -lt 10 ]; do
  if pgrep -x interview_buddy >/dev/null 2>&1; then
    started=true
    break
  fi
  attempt=$((attempt + 1))
  sleep 1
done
if [ "$started" != true ]; then
  echo "Interview Buddy did not stay running after launch." >&2
  exit 1
fi

echo "Interview Buddy started."
if [ "$ad_hoc_build" = true ]; then
  echo "This build uses an ad-hoc identity. Grant Microphone and Screen & System Audio Recording again when prompted after an update."
else
  echo "The stable signing identity keeps privacy approvals valid across rebuilds."
fi
