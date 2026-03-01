#!/usr/bin/env bash
# parse_file.sh — Extract subreddit, username, and Reddit post link from a
# reddsaver-downloaded file path.
#
# File name format: {author}_{post_id}_{index}_{hash8}.{ext}
#   - author:  Reddit username (may contain underscores and hyphens)
#   - post_id: base-36 post ID (alphanumeric, no underscores)
#   - index:   "0", "1", … for single/gallery media;
#              "component_0", "component_1" for Reddit video+audio pairs
#   - hash8:   first 8 hex chars of MD5(media_url)
#
# All parsing is done right-to-left so that underscores in the username
# do not cause ambiguity — the three rightmost components are all
# fixed-format and unambiguous.
#
# Usage: ./scripts/parse_file.sh <path-to-file>

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <path-to-file>" >&2
    exit 1
fi

filepath="$1"

if [[ ! -f "$filepath" ]]; then
    echo "Error: file not found: $filepath" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# 1. Subreddit — immediate parent directory of the file
# ---------------------------------------------------------------------------
subreddit=$(basename "$(dirname "$filepath")")

# ---------------------------------------------------------------------------
# 2. Parse the filename stem right-to-left
# ---------------------------------------------------------------------------
filename=$(basename "$filepath")
stem="${filename%.*}"

# Strip hash8 — always the last underscore-delimited component (8 hex chars)
hash8="${stem##*_}"
if [[ ! "$hash8" =~ ^[0-9a-f]{8}$ ]]; then
    echo "Error: does not look like a reddsaver file (expected 8-char hex hash, got '$hash8')" >&2
    exit 1
fi
rest="${stem%_${hash8}}"

# Strip index — either a plain number ("0", "1", …) or "component_N" for
# Reddit videos that download audio and video as separate tracks.
index_tail="${rest##*_}"
if [[ "$index_tail" =~ ^[0-9]+$ ]]; then
    rest2="${rest%_${index_tail}}"
    prev="${rest2##*_}"
    if [[ "$prev" == "component" ]]; then
        # Index was "component_N" — drop the "component" token too
        rest="${rest2%_component}"
    else
        rest="$rest2"
    fi
else
    rest="${rest%_${index_tail}}"
fi

# Strip post_id — now the rightmost component (base-36, no underscores)
post_id="${rest##*_}"

# Author — everything to the left of the post_id
author="${rest%_${post_id}}"

# ---------------------------------------------------------------------------
# 3. Output
# ---------------------------------------------------------------------------
echo "Subreddit: $subreddit"
echo "Username:  $author"
echo "Post link: https://www.reddit.com/r/${subreddit}/comments/${post_id}/"
