#!/usr/bin/env bash
# download_top_presets.sh — Download common top-listing presets for one subreddit.
#
# Usage:
#   ./scripts/download_top_presets.sh --env-file <env-file> --subreddit <subreddit> --data-dir <data-dir>
#
# Runs:
#   - top/all   limit 1000
#   - top/year  limit 500
#   - top/month limit 250
#   - top/day   limit 25

set -euo pipefail

usage() {
    echo "Usage: $0 --env-file <env-file> --subreddit <subreddit> --data-dir <data-dir>" >&2
    exit 1
}

env_file=""
subreddit=""
data_dir=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --env-file)
            [[ $# -ge 2 ]] || usage
            env_file="$2"
            shift 2
            ;;
        --subreddit)
            [[ $# -ge 2 ]] || usage
            subreddit="$2"
            shift 2
            ;;
        --data-dir)
            [[ $# -ge 2 ]] || usage
            data_dir="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Error: unknown argument: $1" >&2
            usage
            ;;
    esac
done

[[ -n "$env_file" && -n "$subreddit" && -n "$data_dir" ]] || usage

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

if command -v reddsaver >/dev/null 2>&1; then
    reddsaver_bin="reddsaver"
elif [[ -x "${repo_root}/target/release/reddsaver" ]]; then
    reddsaver_bin="${repo_root}/target/release/reddsaver"
else
    echo "Error: could not find 'reddsaver' in PATH or under target/release." >&2
    exit 1
fi

if [[ ! -f "$env_file" ]]; then
    echo "Error: env file not found: $env_file" >&2
    exit 1
fi

run_preset() {
    local period="$1"
    local limit="$2"

    echo "Running top/${period} for r/${subreddit} (limit ${limit})"
    "$reddsaver_bin" \
        --from-env "$env_file" \
        --mode feed \
        --listing-type top \
        --time-filter "$period" \
        --subreddits "$subreddit" \
        --limit "$limit" \
        --data-dir "$data_dir"
}

run_preset all 1000
run_preset year 500
run_preset month 250
run_preset day 25
