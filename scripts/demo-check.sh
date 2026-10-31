#!/usr/bin/env bash
# The demo, end to end (task 1.22). Builds the demo site, serves it with
# `khalid serve`, and checks every page in its sitemaps plus the not-found
# page in each language:
#
#   1. the W3C HTML validator (vnu) on the HTML the server sends,
#   2. Lighthouse in all four categories, where any score under 100 fails,
#   3. axe and the layout checks at 320, 375, 768, 1024, 1280 and 1920 px,
#      first with scripts on and then again with scripts off.
#
# Every check runs even when an earlier one fails, so one run lists all
# problems. Headless browsers run on a server or in CI, never on the
# founder's laptop.
#
# Needs Node 22 or later, Java 17 or later, and the pinned tools:
#   npm install --prefix scripts/demo-check
#   npx --prefix scripts/demo-check playwright install --with-deps chromium
#
# Usage: scripts/demo-check.sh [SITE_FOLDER]   (default fixtures/demo-site)
# DEMO_PORT picks the port (default 8765).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
site="${1:-fixtures/demo-site}"
port="${DEMO_PORT:-8765}"
tools="$root/scripts/demo-check"
work="$root/target/demo-check"

stop() { echo "$1"; exit 1; }
command -v node >/dev/null || stop "Node is missing. The browser checks run on it. Install Node 22 or later, then run this again."
command -v java >/dev/null || stop "Java is missing. The W3C HTML validator runs on it. Install Java 17 or later, then run this again."
[ -d "$tools/node_modules" ] || stop "The check tools are not installed. Lighthouse, axe and the validator come from them. Run: npm install --prefix scripts/demo-check"
[ -f "$site/site.toml" ] || stop "$site has no site.toml. The check needs a site folder. Pass one, or run this from the repository with the demo site in it."

cargo build --release -p khalid
bin="${CARGO_TARGET_DIR:-$root/target}/release/khalid"

# The site answers at its own address, so canonical links stay on the
# domain Lighthouse checks, and localhost counts as secure without HTTPS.
address="http://localhost:$port"
export KHALID_ADDRESS="$address"
rm -rf "$work"
mkdir -p "$work/html" "$work/lighthouse"
"$bin" --site "$site" build --out "$work/public"

# Every page the sitemaps list, then the not-found page in each language.
sed -n 's:.*<loc>\(.*\)</loc>.*:\1:p' "$work/public"/sitemaps/*.xml > "$work/pages.txt"
mount="$(sed -n 's/^mount *= *"\(.*\)"/\1/p' "$site/site.toml")"
mount="${mount:-/}"
default="$(sed -n 's/^default_language *= *"\(.*\)"/\1/p' "$site/site.toml")"
languages="$(sed -n 's/^languages *= *\[\(.*\)\]/\1/p' "$site/site.toml" | tr -d '" ' | tr ',' ' ')"
: > "$work/missing.txt"
for language in ${languages:-$default}; do
  prefix="$language/"
  [ "$language" = "$default" ] && prefix=""
  echo "$address$mount${prefix}no-such-page-here/" >> "$work/missing.txt"
done
[ -s "$work/pages.txt" ] || stop "The sitemaps list no pages. There is nothing to check. Publish at least one post, then run this again."

"$bin" --site "$site" serve --listen "127.0.0.1:$port" > "$work/serve.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
for _ in $(seq 60); do
  curl -fsS -o /dev/null "$address$mount" 2>/dev/null && break
  kill -0 "$server" 2>/dev/null || stop "khalid serve stopped before it answered. Its output is in $work/serve.log. Fix what it says, then run this again."
  sleep 0.5
done
curl -fsS -o /dev/null "$address$mount" || stop "khalid serve did not answer within 30 seconds. Nothing can be checked without it. Read $work/serve.log, fix what it says, then run this again."

failed=0
number=0
while read -r url; do
  number=$((number + 1))
  curl -fsS -o "$work/html/$number.html" "$url" || { echo "Could not fetch $url"; failed=1; }
done < "$work/pages.txt"
while read -r url; do
  number=$((number + 1))
  curl -sS -o "$work/html/$number.html" "$url" || { echo "Could not fetch $url"; failed=1; }
done < "$work/missing.txt"

echo "HTML validator:"
vnu="$(node -p 'require(process.argv[1])' "$tools/node_modules/vnu-jar")"
if java -jar "$vnu" --errors-only --format text "$work"/html/*.html; then
  echo "Every page is valid HTML."
else
  echo "The validator found errors. Browsers and readers guess at invalid HTML. Fix each one in the template that writes it."
  failed=1
fi

echo "Lighthouse:"
CHROME_PATH="$(cd "$tools" && node -p 'require("playwright").chromium.executablePath()')"
export CHROME_PATH
# Lighthouse runs inside `while read` loops, so it reads nothing from them.
lighthouse() {
  "$tools/node_modules/.bin/lighthouse" "$1" --quiet --output=json \
    --output-path="$work/lighthouse/$2.json" \
    --chrome-flags="--headless=new --no-sandbox" "${@:3}" < /dev/null || failed=1
}
number=0
while read -r url; do
  number=$((number + 1))
  lighthouse "$url" "$number"
done < "$work/pages.txt"
# A not-found page answers 404 on purpose, which SEO and best practices
# count against it, so it is held to the other two categories.
while read -r url; do
  number=$((number + 1))
  lighthouse "$url" "$number" --only-categories=performance,accessibility
done < "$work/missing.txt"
node "$tools/scores.mjs" "$work"/lighthouse/*.json || failed=1

echo "axe and layout, six widths, scripts on and off:"
cat "$work/pages.txt" "$work/missing.txt" > "$work/all.txt"
node "$tools/browse.mjs" "$work/all.txt" || failed=1

if [ "$failed" -ne 0 ]; then
  echo "The demo check failed. A reader would meet the problems above. Fix each one in the crate that owns it, then run scripts/demo-check.sh again."
  exit 1
fi
echo "The demo check passed: valid HTML, Lighthouse 100, and no axe or layout problem at six widths with scripts on and off."
