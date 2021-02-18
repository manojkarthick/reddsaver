# Reddsaver ![build](https://github.com/manojkarthick/reddsaver/workflows/build/badge.svg) [![Crates.io](https://img.shields.io/crates/v/reddsaver.svg)](https://crates.io/crates/reddsaver)

* Command line tool to download saved media from Reddit
* Supports:
  - Reddit: PNG/JPG images, GIFs, Image galleries, videos
  - Giphy: GIFs
  - Imgur: Direct images and GIFVs
  - Gfycat/Redgifs: GIFs
* GIF/GIFV from Imgur/Gfycat/Redgifs are downloaded as mp4
* Does *not* support downloading images from Imgur post links

## Installation

### Recommended method

You can download release binaries [here](https://github.com/manojkarthick/reddsaver/releases)

### Alternative methods

#### Using cargo

If you already have Rust installed, you can also install using `cargo`: 
```shell script
cargo install reddsaver
```

#### Using nix

If you are a [nix](https://github.com/NixOS/nix) user, you can install reddsaver from [nixpkgs](https://github.com/NixOS/nixpkgs/blob/master/pkgs/applications/misc/reddsaver/default.nix)
```shell script
nix-env --install reddsaver
```

or, if you manage your installation using [home-manager](https://github.com/nix-community/home-manager), add to your `home.packages`:
```shell script
home.packages = [
    pkgs.reddsaver
]; 
```

#### Building and running from source

Make sure you have rustc `v1.50.0` and cargo installed on your machine.
```shell script
git clone https://github.com/manojkarthick/reddsaver.git
cargo build --release
./target/release/reddsaver
```

#### Docker support

Pre-built docker images are available on [Docker Hub](https://hub.docker.com/u/manojkarthick) 
 
```shell script
mkdir -pv data/
docker run --rm \
    --volume="$PWD/data:/app/data" \
    --volume="$PWD/reddsaver.env:/app/reddsaver.env" \
    reddsaver:latest -d /app/data -e /app/reddsaver.env
```

## Running

1. Create a new script application at https://www.reddit.com/prefs/apps
    * Click on create an app at the bottom of the page
    * Input a name for your application, for example: <username>-reddsaver
    * Choose "script" as the type of application
    * Set "http://localhost:8080" or any other URL for the redirect url
    * Click on "create app" - you should now see the application has been created
    * Under your application name, you should see a random string - that is your client ID
    * The random string next to the field "secret" is your client secret 
2. Copy the client ID and client secret information returned
3. Create a .env file with the following keys, for example `reddsaver.env`:  
```shell script
CLIENT_ID=<client_id>
CLIENT_SECRET=<client_secret>
USERNAME=<username>
PASSWORD=<password>
```
_NOTE_: If you have 2FA enabled, please make sure you set `PASSWORD=<password>:<2FA_TOTP_token>` instead

4. Run the app! 
```shell script

# Create a directory to save your images to
mkdir -pv reddsaver/

# Check if you installation is working properly
reddsaver --help

# Check if the right configuration has been picked up
reddsaver -e reddsaver.env -d reddsaver --show-config  

# Run the app to download the saved images
reddsaver -e reddsaver.env -d reddsaver
```

NOTE: When running the application beyond the first time, if you use the directory as the initial run, the application will skip downloading the images that have already been downloaded.

View it in action here: 

[![asciicast](https://asciinema.org/a/382339.svg)](https://asciinema.org/a/382339)

## Description and command line arguments

Optionally override the values for the directory to save and the env file to read from:

```shell script
ReddSaver 0.3.0
Manoj Karthick Selva Kumar
Simple CLI tool to download saved media from Reddit

USAGE:
    reddsaver [FLAGS] [OPTIONS]

FLAGS:
    -r, --dry-run           Dry run and print the URLs of saved media to download
    -h, --help              Prints help information
    -H, --human-readable    Use human readable names for files
    -s, --show-config       Show the current config being used
    -U, --unsave            Unsave post after processing
    -V, --version           Prints version information

OPTIONS:
    -d, --data-dir <DATA_DIR>           Directory to save the media to [default: data]
    -e, --from-env <ENV_FILE>           Set a custom .env style file with secrets [default: .env]
    -S, --subreddits <SUBREDDITS>...    Download media from these subreddits only
```

Some points to note:

* By default, reddsaver generates filenames for the images using a MD5 Hash of the URLs. You can instead generate human readable names using the `--human-readable` flag.
* You can check the configuration used by ReddSaver by using the `--show-config` flag.

## Other Information

### Building for Raspberry Pi Zero W

To cross-compile for raspberry pi, this project uses [rust-cross](https://github.com/rust-embedded/cross). Make sure you have docker installed on your development machine.

1. Build the docker image for rust-cross: `docker build -t rust-rpi-zerow:v1-openssl -f Dockerfile.raspberrypizerow .`
2. Make sure that the image name used here matches the image name in your `Cross.toml` configuration
3. Run `cross build --target arm-unknown-linux-gnueabi --release` to build the project
4. You can find the compiled binary under `target/arm-unknown-linux-gnueabi/release/`
