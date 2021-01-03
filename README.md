# Reddsaver ![build](https://github.com/manojkarthick/reddsaver/workflows/build/badge.svg) [![Crates.io](https://img.shields.io/crates/v/reddsaver.svg)](https://crates.io/crates/reddsaver)

* Command line tool to download saved images from Reddit 
* Supports png/jpg images only
* Also supports downloading images from Reddit image galleries 

## Installation

### Recommended method

You can download release binaries [here](https://github.com/manojkarthick/reddsaver/releases)

### Alternative methods

#### Using cargo

If you already have Rust installed, you can also install using `cargo`: 
```
cargo install reddsaver
```

#### Building and running from source

Make sure you have rustc `v1.48.0` and cargo installed on your machine.
```
git clone https://github.com/manojkarthick/reddsaver.git
cargo build --release
./target/release/reddsaver
```

#### Docker support

Pre-built docker images are available on [Docker Hub](https://hub.docker.com/u/manojkarthick) 
 
```
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
```
CLIENT_ID=<client_id>
CLIENT_SECRET=<client_secret>
USERNAME=<username>
PASSWORD=<password>
```
4. Run the app! 
```
mkdir -pv reddsaver/
reddsaver --help
reddsaver --e reddsaver.env -f reddsaver/
```

NOTE: When running the application beyond the first time, if you use the directory as the initial run, the application will skip downloading the images that have already been downloaded.

View it in action here: [![asciicast](https://asciinema.org/a/382339.svg)](https://asciinema.org/a/382339)

## Description and command line arguments

Optionally override the values for the directory to save and the env file to read from

```
ReddSaver 0.1.0
Manoj Karthick Selva Kumar
Simple CLI tool to download saved images from Reddit

USAGE:
    reddsaver [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --data-dir <DATA_DIR>    Directory to save the images to [default: data]
    -e, --from-env <ENV_FILE>    Set a custom .env style file with secrets [default: .env]
```

## TODO
- [x] Separate dockerfiles
- [x] Publish to Crates.io
- [x] Publish docker images

## Other Information

### Building for Raspberry Pi Zero W

To cross-compile for raspberry pi, this project uses [rust-cross](https://github.com/rust-embedded/cross). Make sure you have docker installed on your development machine.

1. Build the docker image for rust-cross: `docker build -t rust-rpi-zerow:v1-openssl -f Dockerfile.raspberrypizerow .`
2. Make sure that the image name used here matches the image name in your `Cross.toml` configuration
3. Run `cross build --target arm-unknown-linux-gnueabi --release` to build the project
4. You can find the compiled binary under `target/arm-unknown-linux-gnueabi/release/`
