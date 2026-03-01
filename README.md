# Reddsaver ![build](https://github.com/manojkarthick/reddsaver/workflows/build/badge.svg) [![Crates.io](https://img.shields.io/crates/v/reddsaver.svg)](https://crates.io/crates/reddsaver)

Command line tool to download saved/upvoted media from Reddit.

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
REDDSAVER_CLIENT_ID="<client_id>"
REDDSAVER_CLIENT_SECRET="<client_secret>"
REDDSAVER_USERNAME="<username>"
REDDSAVER_PASSWORD="<password>"
```

> If you have 2FA enabled: `REDDSAVER_PASSWORD=<password>:<2FA_TOTP_token>`

## Usage

```shell
# Create a directory to save media to
mkdir -pv data/

# Verify the configuration is correct
reddsaver -e reddsaver.env -d data --show-config

# Download saved media
reddsaver -e reddsaver.env -d data

# Download upvoted media instead
reddsaver -e reddsaver.env -d data --upvoted

# Dry run — print URLs without downloading
reddsaver -e reddsaver.env -d data --dry-run

# Restrict to specific subreddits
reddsaver -e reddsaver.env -d data --subreddits pics,aww,videos
```

On subsequent runs, files that already exist in the data directory are skipped automatically.

## Command line reference

```
ReddSaver 1.0.0
Manoj Karthick Selva Kumar
Simple CLI tool to download saved media from Reddit

USAGE:
    reddsaver [FLAGS] [OPTIONS]

FLAGS:
    -r, --dry-run       Dry run and print the URLs of saved media to download
    -h, --help          Prints help information
    -s, --show-config   Show the current config being used
    -U, --undo          Unsave or remove upvote for post after processing
    -u, --upvoted       Download media from upvoted posts
    -V, --version       Prints version information

OPTIONS:
    -d, --data-dir <DATA_DIR>           Directory to save the media to [default: data]
    -e, --from-env <ENV_FILE>           Set a custom .env style file with secrets [default: .env]
    -S, --subreddits <SUBREDDITS>...    Download media from these subreddits only
```

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
