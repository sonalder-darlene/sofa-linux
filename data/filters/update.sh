#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# Refreshes the two filter lists built into Sofa, as domain-only lists (one domain per line, `#` comments), each
# with its source and licence on top. Run it before a release, from anywhere; it needs curl.
#   StevenBlack hosts (MIT): https://github.com/StevenBlack/hosts
#   HaGeZi Pop-Up Ads (GPL-3.0): https://github.com/hagezi/dns-blocklists
set -euo pipefail
cd "$(dirname "$0")"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

src="https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts"
curl -fsSL "$src" -o "$tmp"
{
  echo "# Source: $src"
  grep -m1 '^# Date:' "$tmp" || true
  echo "# License: MIT, Copyright (c) Steven Black"
  awk '$1 == "0.0.0.0" && $2 != "0.0.0.0" { print $2 }' "$tmp"
} > stevenblack-hosts.txt

src="https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/popupads-onlydomains.txt"
curl -fsSL "$src" -o "$tmp"
{
  echo "# Source: $src"
  grep -m1 '^# Version:' "$tmp" || true
  echo "# License: GPL-3.0, HaGeZi (https://github.com/hagezi/dns-blocklists)"
  grep -v '^#' "$tmp" | grep -v '^$'
} > hagezi-popupads.txt

for f in stevenblack-hosts.txt hagezi-popupads.txt; do
  echo "$f: $(grep -vc "^#" "$f") domains, $(stat -c %s "$f") bytes"
done
