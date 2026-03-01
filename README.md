# Reddsaver ![build](https://github.com/manojkarthick/reddsaver/workflows/build/badge.svg) [![Crates.io](https://img.shields.io/crates/v/reddsaver.svg)](https://crates.io/crates/reddsaver)

Command line tool to download saved, upvoted, or subreddit feed media from Reddit.

**Supported sources:**
| Source | Media types |
|---|---|
| Reddit | Images (JPG/PNG), GIFs, image galleries, videos |
| Imgur | Direct images, GIFVs (downloaded as mp4) |
| Gfycat / Redgifs | GIFs (downloaded as mp4) |
| Giphy | GIFs |
| YouTube | Videos (requires `yt-dlp`) |

> Does *not* support Imgur post/album links — only direct media links.

## Prerequisites

**ffmpeg** — required to merge the separate audio and video streams that Reddit uses for hosted videos.
Install from https://www.ffmpeg.org/download.html or via your package manager.

**yt-dlp** — required only if you want to download YouTube videos linked in your saved/upvoted posts.
Install from https://github.com/yt-dlp/yt-dlp or via your package manager.

## Installation

### Release binaries

Download a pre-built binary for your platform from the [releases page](https://github.com/manojkarthick/reddsaver/releases).

### MacPorts

```shell
sudo port selfupdate
sudo port install reddsaver
```

### Homebrew

```shell
brew tap manojkarthick/reddsaver
brew install reddsaver
```

### Arch Linux

```shell
yay -S reddsaver
```

### cargo

```shell
cargo install reddsaver
```

### nix

```shell
nix-env --install reddsaver
```

Or via [home-manager](https://github.com/nix-community/home-manager):

```nix
home.packages = [ pkgs.reddsaver ];
```

### Build from source

Requires a current stable Rust toolchain with Cargo support for lock file version 4.
If `cargo build` fails with a lock file version error, upgrade Rust and Cargo first.

```shell
git clone https://github.com/manojkarthick/reddsaver.git
cargo build --release
./target/release/reddsaver
```

### Docker

```shell
mkdir -pv data/
docker run --rm \
    --volume="$PWD/data:/app/data" \
    --volume="$PWD/reddsaver.env:/app/reddsaver.env" \
    ghcr.io/manojkarthick/reddsaver:latest -d /app/data -e /app/reddsaver.env
```

## Setup

1. Create a Reddit script application at https://www.reddit.com/prefs/apps
   - Click **create an app** at the bottom of the page
   - Give it a name (e.g. `<username>-reddsaver`)
   - Choose **script** as the type
   - Set any redirect URL (e.g. `http://localhost:8080`)
   - Click **create app** — the string beneath the app name is your **client ID**; the string next to **secret** is your **client secret**

2. Create a `.env` file (e.g. `reddsaver.env`) with your credentials:

```shell
REDDSAVER_CLIENT_ID='<client_id>'
REDDSAVER_CLIENT_SECRET='<client_secret>'
REDDSAVER_USERNAME='<username>'
REDDSAVER_PASSWORD='<password>'
```

> [!IMPORTANT]
> Use single quotes for values in the env file, especially if a value contains `$`. Unquoted `$` triggers variable substitution, and double-quoted values containing special characters can be parsed unexpectedly.
>
> If you have 2FA enabled: `REDDSAVER_PASSWORD='<password>:<2FA_TOTP_token>'`

## Usage

```shell
# Create a directory to save media to
mkdir -pv data/

# Verify the configuration is correct
reddsaver -e reddsaver.env -d data --show-config

# Download saved media (default)
reddsaver -e reddsaver.env -d data

# Download upvoted media instead
reddsaver -e reddsaver.env -d data --mode upvoted

# Download from a subreddit's hot feed (default listing type, limit 1000)
reddsaver -e reddsaver.env -d data --mode feed --subreddits pics

# Download top posts of the week from multiple subreddits (500 per subreddit)
reddsaver -e reddsaver.env -d data --mode feed --subreddits pics,aww --listing-type top --time-filter week --limit 500

# Download new posts from a subreddit
reddsaver -e reddsaver.env -d data --mode feed --subreddits earthporn --listing-type new

# Limit saved/upvoted downloads
reddsaver -e reddsaver.env -d data --limit 200

# Dry run — print URLs without downloading
reddsaver -e reddsaver.env -d data --dry-run

# Restrict saved/upvoted downloads to specific subreddits
reddsaver -e reddsaver.env -d data --subreddits pics,aww,videos
```

On subsequent runs, files that already exist in the data directory are skipped automatically.

## Command line reference

```
Simple CLI tool to download saved media from Reddit

Usage: reddsaver [OPTIONS]

Options:
  -e, --from-env <ENV_FILE>      Set a custom .env style file with secrets [default: .env]
  -d, --data-dir <DATA_DIR>      Directory to save the media to [default: data]
  -s, --show-config              Show the current config being used
  -r, --dry-run                  Dry run and print the URLs of saved media to download
  -S, --subreddits <SUBREDDITS>  Subreddits to filter (saved/upvoted) or fetch from (feed mode)
  -m, --mode <MODE>              Operation mode [default: saved] [possible values: saved, upvoted, feed]
  -t, --listing-type <TYPE>      Subreddit listing sort [default: hot] [possible values: hot, top, new, controversial]
  -T, --time-filter <PERIOD>     Time period for top/controversial listings [default: all] [possible values: hour, day, week, month, year, all]
  -l, --limit <LIMIT>            Max posts to process per source (default: unlimited for saved/upvoted, 1000 for feed)
  -h, --help                     Print help
  -V, --version                  Print version
```

### Modes

| Mode | Flag | Description |
|------|------|-------------|
| `saved` | `--mode saved` (default) | Download media from your saved posts |
| `upvoted` | `--mode upvoted` | Download media from your upvoted posts |
| `feed` | `--mode feed` | Download media from a subreddit's listing feed |

### Feed mode options

`--listing-type` and `--time-filter` are only valid with `--mode feed`. `--subreddits` is required in feed mode and specifies which subreddit(s) to fetch from. The `--limit` defaults to **1000 per subreddit**.

| Listing type | Time filter applies? | Description |
|---|---|---|
| `hot` (default) | No | Currently trending posts |
| `top` | Yes | Highest-scoring posts in the given time period |
| `new` | No | Most recently submitted posts |
| `controversial` | Yes | Most controversial posts in the given time period |

Time filter values for `top` and `controversial`: `hour`, `day`, `week`, `month`, `year`, `all` (default `all`).

## File naming

Downloaded files are named using the format:

```
{subreddit}/{author}_{post_id}_{index}_{hash8}.{ext}
```

- **author** — Reddit username of the poster; files sort naturally by user in any file browser
- **post_id** — base-36 Reddit post ID; use it to navigate directly to the source post
- **index** — `0` for single media; `0`, `1`, … for gallery items; `component_0`/`component_1` for Reddit videos with separate audio/video tracks
- **hash8** — first 8 characters of the MD5 hash of the media URL, used as a collision guard

Example: `aww/thunderbird42_abc123_0_f3a8b2c1.jpg`

## Utilities

### parse_file.sh

Given any file downloaded by reddsaver, prints the subreddit, username, and a direct link to the source Reddit post.

```shell
./scripts/parse_file.sh data/aww/thunderbird42_abc123_0_f3a8b2c1.jpg
# Subreddit: aww
# Username:  thunderbird42
# Post link: https://www.reddit.com/r/aww/comments/abc123/
```

## Download summary

At the end of each run, reddsaver prints a summary broken down by media source:

```
#####################################
Download Summary:
  Total supported:  467
  Total downloaded: 459
  Total skipped:    8

  Source                   | Supported | Downloaded | Skipped
  -----------------------------------------------------------
  Imgur GIF                |         8 |          8 |       0
  Imgur Image              |        48 |         48 |       0
  Reddit GIF               |        14 |         14 |       0
  Reddit Image             |       320 |        320 |       0
  Reddit Video (no audio)  |         3 |          3 |       0
  Redgifs                  |        74 |         66 |       8
#####################################
```

Skipped items are either already present on disk or could not be retrieved (a `WARN` log line is printed for each).
