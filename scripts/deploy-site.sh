#!/bin/bash
# Publish dist/site to the gh-pages branch (GitHub Pages serves it at niconistal.github.io/kartu).
# Runs scripts/build-site.sh first. The branch holds only build output; history is squashed.
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/build-site.sh
src=$(git rev-parse --short HEAD)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cp -r dist/site/. "$tmp"/
cd "$tmp"
git init -q -b gh-pages
git -C "$OLDPWD" config user.name  | xargs -I{} git config user.name  {}
git -C "$OLDPWD" config user.email | xargs -I{} git config user.email {}
git add -A
git commit -q -m "site: build from $src"
git push -f "$(git -C "$OLDPWD" remote get-url origin)" gh-pages
echo "pushed gh-pages (from $src) → https://niconistal.github.io/kartu/"
