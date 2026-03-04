#!/usr/bin/env bash
# convert_gifs.sh — Recursively find all GIFs and convert them to MP4 using
# ffmpeg, renaming each <filename>.gif to <filename>.mp4 in place.
#
# Usage: ./scripts/convert_gifs.sh [--dry-run] [--jobs <N>] <target-directory>
#
# Options:
#   --dry-run      List files that would be converted without modifying them
#   --jobs <N>     Number of parallel ffmpeg workers (default: 1)
#
# ffmpeg flags used:
#   -nostdin                   prevent ffmpeg from consuming stdin
#   -movflags faststart        move MP4 metadata to front for fast playback
#   -pix_fmt yuv420p           broad player compatibility
#   -vf scale=...              ensure even dimensions (H.264 requirement)
#
# The original .gif is removed only after a successful conversion.

set -euo pipefail

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
DRY_RUN=false
JOBS=1
TARGET_DIR=""

usage() {
    echo "Usage: $0 [--dry-run] [--jobs <N>] <target-directory>" >&2
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --jobs)
            if [[ -z "${2:-}" || ! "$2" =~ ^[0-9]+$ || "$2" -eq 0 ]]; then
                echo "Error: --jobs requires a positive integer" >&2
                usage
            fi
            JOBS="$2"
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
if ! command -v ffmpeg &>/dev/null; then
    echo "Error: ffmpeg is not installed or not in PATH." >&2
    echo "  macOS: brew install ffmpeg" >&2
    echo "  Linux: sudo apt install ffmpeg  (or equivalent)" >&2
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
    if (( bytes >= 1024 * 1024 * 1024 )); then
        awk "BEGIN { printf \"%.2f GB\", $bytes / (1024*1024*1024) }"
    elif (( bytes >= 1024 * 1024 )); then
        awk "BEGIN { printf \"%.2f MB\", $bytes / (1024*1024) }"
    else
        awk "BEGIN { printf \"%.2f KB\", $bytes / 1024 }"
    fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

# Dry-run mode: list files and exit
if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] Scanning '$TARGET_DIR' for GIFs to convert ..."
    found=0
    total_before=0
    while IFS= read -r -d '' gif; do
        found=$(( found + 1 ))
        gif_size=$(file_size_bytes "$gif")
        total_before=$(( total_before + gif_size ))
        echo "  [would convert] $gif  ($(human_readable "$gif_size"))"
    done < <(find "$TARGET_DIR" -type f -iname "*.gif" -print0)
    echo ""
    echo "Dry-run complete."
    echo "  Files that would be converted : $found"
    echo "  Total GIF size                : $(human_readable "$total_before")"
    exit 0
fi

# Create a temp directory for per-file result logs
RESULTS_DIR=$(mktemp -d)
trap 'rm -rf "$RESULTS_DIR"' EXIT

# Worker function: converts a single GIF and writes stats to a result file
convert_one() {
    local gif="$1"
    local results_dir="$2"
    local result_file="$results_dir/$(basename "$gif").result"

    if [[ ! -f "$gif" ]]; then
        echo "  Skipping (no longer exists): $gif" >&2
        return
    fi

    local gif_size mp4 mp4_size
    gif_size=$(file_size_bytes "$gif")
    mp4="${gif%.gif}.mp4"

    echo "  Converting: $gif  ($(human_readable "$gif_size"))"
    if ffmpeg -nostdin -hide_banner -loglevel error \
        -i "$gif" \
        -movflags faststart \
        -pix_fmt yuv420p \
        -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2" \
        "$mp4"; then
        mp4_size=$(file_size_bytes "$mp4")
        rm "$gif"
        echo "    -> $(basename "$mp4")  ($(human_readable "$mp4_size"))"
        echo "ok $gif_size $mp4_size" > "$result_file"
    else
        echo "    -> ffmpeg failed, original kept: $gif" >&2
        [[ -f "$mp4" ]] && rm "$mp4"
        echo "fail $gif_size" > "$result_file"
    fi
}

export -f convert_one file_size_bytes human_readable

echo "Scanning '$TARGET_DIR' for GIFs to convert (jobs: $JOBS) ..."

find "$TARGET_DIR" -type f -iname "*.gif" -print0 \
    | xargs -0 -P "$JOBS" -I {} bash -c 'convert_one "$@"' _ {} "$RESULTS_DIR"

# ---------------------------------------------------------------------------
# Aggregate results
# ---------------------------------------------------------------------------
found=0
converted=0
failed=0
total_before=0
total_after=0

for result_file in "$RESULTS_DIR"/*.result; do
    [[ -f "$result_file" ]] || continue
    read -r status before after < "$result_file"
    found=$(( found + 1 ))
    total_before=$(( total_before + before ))
    if [[ "$status" == "ok" ]]; then
        converted=$(( converted + 1 ))
        total_after=$(( total_after + after ))
    else
        failed=$(( failed + 1 ))
        total_after=$(( total_after + before ))
    fi
done

saved=$(( total_before - total_after ))
echo ""
echo "Done."
echo "  Found      : $found"
echo "  Converted  : $converted"
echo "  Failed     : $failed"
echo "  Size before: $(human_readable "$total_before")"
echo "  Size after : $(human_readable "$total_after")"
echo "  Space saved: $(human_readable "$saved")"
