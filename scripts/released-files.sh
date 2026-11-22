#!/usr/bin/env bash
# A release's files, without downloading them from the release.
#
#   scripts/released-files.sh <tag | latest> <directory>
#
# GitHub counts every download from a release, and the dashboard shows those
# counts as people (website/app/dashboard). Our own tests and checks fetched
# the releases hundreds of times (2026-11-22: 480 of 543), so they take the
# files from where the release workflow built them instead: its artifacts,
# kept 90 days. Each file is then checked against the published release's own
# digest, so what is tested is byte for byte what people download, and a file
# the release does not carry, or carries differently, stops here.
#
# Gives the versioned names and the names without a version (QurbSpace-*), as
# the release does. Needs gh, signed in (GH_TOKEN in a workflow, with
# actions: read).
set -euo pipefail

tag=${1:?a tag such as v0.2.0-beta.13, or latest}
out=${2:?a directory}
repo=${REPO:-saqibsiddiq/QurbSpace}

if [ "$tag" = latest ]; then
  tag=$(gh release view -R "$repo" --json tagName -q .tagName)
fi
run=$(gh run list -R "$repo" --workflow release.yml --branch "$tag" --status success --limit 1 --json databaseId -q '.[0].databaseId')
if [ -z "$run" ]; then
  echo "no successful release build for $tag, or its files are past their 90 days" >&2
  exit 1
fi

mkdir -p "$out"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
gh run download "$run" -R "$repo" -D "$work"
find "$work" -type f -exec cp {} "$out/" \;

# The names without the version, made as release.yml makes them.
cd "$out"
for pair in "QurbSpace-*-android.apk QurbSpace-android.apk" "qurb_*_amd64.AppImage QurbSpace-linux.AppImage" \
  "qurb_*_amd64.deb QurbSpace-linux.deb" "qurb_*_x64-setup.exe QurbSpace-windows-setup.exe"; do
  set -- $pair
  for f in $1; do [ -f "$f" ] && cp "$f" "$2"; done
done

# Every file that the release carries must be here, and the same.
gh api "repos/$repo/releases/tags/$tag" -q '.assets[] | select(.name | test("\\.json$") | not) | "\(.digest) \(.name)"' |
  while read -r digest name; do
    if [ ! -f "$name" ]; then
      echo "$tag carries $name, which its build does not" >&2
      exit 1
    fi
    got="sha256:$(sha256sum "$name" | cut -d' ' -f1)"
    if [ "$got" != "$digest" ]; then
      echo "$name differs from the one published in $tag" >&2
      exit 1
    fi
  done
echo "$tag: $(ls | wc -l) files from release build $run, each the same as published"
