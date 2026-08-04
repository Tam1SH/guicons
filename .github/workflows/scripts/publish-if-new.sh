#!/usr/bin/env bash
# Publishes crate $1 to crates.io only if its current workspace version
# isn't already live there - makes release-crates.yml idempotent, so a
# plain push (no version bump) just skips every crate instead of failing
# on "already uploaded". Shared between all crates published by that
# workflow rather than copy-pasted per step.
set -euo pipefail

crate="$1"
version=$(cargo metadata --no-deps --format-version 1 | jq -r --arg name "$crate" '.packages[] | select(.name == $name) | .version')

if [ -z "$version" ]; then
  echo "::error::couldn't determine $crate's version from cargo metadata"
  exit 1
fi

status=$(curl -s -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/$crate/$version")
if [ "$status" = "200" ]; then
  echo "$crate $version is already published - skipping"
  exit 0
fi

echo "publishing $crate $version"
# The pre-check above hits crates.io's REST API, which can lag behind the
# sparse index `cargo publish` itself checks against - hit this exactly
# once (a prior run's publish step succeeded, but a rerun's pre-check
# still saw a stale 404 minutes later). Treat cargo's own "already
# exists" as the authoritative, race-free answer and fall through to
# skip instead of failing the whole workflow over a slow-to-propagate
# API response.
if ! output=$(cargo publish -p "$crate" 2>&1); then
  echo "$output"
  if echo "$output" | grep -q "already exists on crates.io index"; then
    echo "$crate $version is already published (crates.io's REST API just hadn't caught up yet) - skipping"
    exit 0
  fi
  exit 1
fi
echo "$output"
# A freshly published crate needs a few seconds to show up in the index
# before the next crate's `cargo publish` (which depends on it) can
# resolve it.
sleep 30
