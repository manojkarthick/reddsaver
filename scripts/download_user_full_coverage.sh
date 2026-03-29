#!/usr/bin/env bash
# download_user_full_coverage.sh — Download a Reddit user's submissions with maximum coverage.
#
# Usage:
#   ./scripts/download_user_full_coverage.sh --env-file <env-file> --user <username> --data-dir <data-dir>
#
# Runs:
#   - top/all   limit 1000
#   - top/year  limit 500
#   - top/month limit 250
#   - top/day   limit 25
#   - new        limit 1000
#   - best       limit 1000

set -euo pipefail

usage() {
    echo "Usage: $0 --env-file <env-file> --user <username> --data-dir <data-dir>" >&2
    exit 1
}

env_file=""
username=""
data_dir=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --env-file)
            [[ $# -ge 2 ]] || usage
            env_file="$2"
            shift 2
            ;;
        --user)
            [[ $# -ge 2 ]] || usage
            username="$2"
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

[[ -n "$env_file" && -n "$username" && -n "$data_dir" ]] || usage

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
    local listing_type="$1"
    local limit="$2"
    local period="${3:-}"

    local label="$listing_type"
    [[ -n "$period" ]] && label="${listing_type}/${period}"
    echo "Running ${label} for u/${username} (limit ${limit})"

    local args=(
        --from-env "$env_file"
        --mode user
        --user "$username"
        --listing-type "$listing_type"
        --limit "$limit"
        --data-dir "$data_dir"
    )
    [[ -n "$period" ]] && args+=(--time-filter "$period")
    "$reddsaver_bin" "${args[@]}"
}

run_preset top 1000 all
run_preset top  500 year
run_preset top  250 month
run_preset top   25 day
run_preset new  1000
run_preset best 1000
