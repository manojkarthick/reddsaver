#!/usr/bin/env bash
# compress_gifs.sh — Recursively find and compress GIFs above a size threshold
# using gifsicle.
#
# Usage: ./scripts/compress_gifs.sh [--dry-run] [--threshold <MB>] <target-directory>
#
# Options:
#   --dry-run          List files that would be processed without modifying them
#   --threshold <MB>   Size threshold in MB (default: 10)
#
# Compresses any .gif file larger than THRESHOLD_MB in-place using gifsicle
# with maximum optimisation (-O3). Skips files that cannot be read or that
# gifsicle fails to compress.

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
THRESHOLD_MB=10
THRESHOLD_BYTES=$(( THRESHOLD_MB * 1024 * 1024 ))

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
DRY_RUN=false
TARGET_DIR=""

usage() {
    echo "Usage: $0 [--dry-run] [--threshold <MB>] <target-directory>" >&2
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --threshold)
            if [[ -z "${2:-}" || ! "${2}" =~ ^[0-9]+$ ]]; then
                echo "Error: --threshold requires a positive integer (MB)" >&2
                usage
            fi
            THRESHOLD_MB="$2"
            THRESHOLD_BYTES=$(( THRESHOLD_MB * 1024 * 1024 ))
            shift 2
            ;;
        -*)
            echo "Unknown option: $1" >&2
            usage
            ;;
        *)
            if [[ -n "$TARGET_DIR" ]]; then
                echo "Error: unexpected argument '$1'" >&2
                usage
            fi
            TARGET_DIR="$1"
            shift
            ;;
    esac
done

if [[ -z "$TARGET_DIR" ]]; then
    usage
fi

# ---------------------------------------------------------------------------
# Preflight checks
# ---------------------------------------------------------------------------
if ! command -v gifsicle &>/dev/null; then
    echo "Error: gifsicle is not installed or not in PATH." >&2
    echo "  macOS: brew install gifsicle" >&2
    echo "  Linux: sudo apt install gifsicle  (or equivalent)" >&2
    exit 1
fi

if [[ ! -d "$TARGET_DIR" ]]; then
    echo "Error: target directory not found: $TARGET_DIR" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# Portable file-size in bytes (macOS stat differs from GNU stat)
file_size_bytes() {
    if [[ "$(uname -s)" == "Darwin" ]]; then
        gstat -c%s "$1"
    else
        stat -c%s "$1"
    fi
}

human_readable() {
    local bytes=$1
    if (( bytes >= 1024 * 1024 )); then
        awk "BEGIN { printf \"%.1f MB\", $bytes / (1024*1024) }"
    else
        awk "BEGIN { printf \"%.1f KB\", $bytes / 1024 }"
    fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] Scanning '$TARGET_DIR' for GIFs larger than ${THRESHOLD_MB} MB ..."
else
    echo "Scanning '$TARGET_DIR' for GIFs larger than ${THRESHOLD_MB} MB ..."
fi

found=0
processed=0
skipped=0
failed=0

while IFS= read -r -d '' gif; do
    size=$(file_size_bytes "$gif")
    if (( size <= THRESHOLD_BYTES )); then
        continue
    fi

    found=$(( found + 1 ))
    human=$(human_readable "$size")

    if [[ "$DRY_RUN" == true ]]; then
        echo "  [would compress] $gif  ($human)"
        continue
    fi

    echo "  Compressing: $gif  ($human)"
    if gifsicle -O3 --batch "$gif"; then
        new_size=$(file_size_bytes "$gif")
        new_human=$(human_readable "$new_size")
        saved=$(( size - new_size ))
        saved_human=$(human_readable "$saved")
        echo "    -> $new_human  (saved $saved_human)"
        processed=$(( processed + 1 ))
    else
        echo "    -> gifsicle failed, skipping: $gif" >&2
        failed=$(( failed + 1 ))
    fi
done < <(find "$TARGET_DIR" -type f -iname "*.gif" -print0)

echo ""
if [[ "$DRY_RUN" == true ]]; then
    echo "Dry-run complete. Files that would be processed: $found"
else
    echo "Done. Found: $found | Compressed: $processed | Failed: $failed"
fi
