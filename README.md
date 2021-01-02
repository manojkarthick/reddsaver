# Reddsaver ![build](https://github.com/manojkarthick/reddsaver/workflows/build/badge.svg)

* Download images present in your saved posts on Reddit   
* Supports png/jpg images only
* Supports downloading images from reddit image galleries 

## Installation

### Recommended method

You can download release binaries [here](https://github.com/manojkarthick/reddsaver/releases)

### Building and running from source

1. Build the application: `cargo build --release`
2. Run the application: `./target/release/reddsaver`

### Docker support

1. Pre-built docker images are available at https://hub.docker.com/u/manojkarthick
2. Currently using manual builds. Docker autobuild is still WIP
 
```
mkdir -pv data/
docker build -t reddsaver:v0.1.0 .
docker run --rm \
    --volume="$PWD/data:/app/data" \
    --volume="$PWD/.env:/app/.env" \
    reddsaver:v0.1.0
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
reddsaver --e reddsaver.env -f reddsaver/
```

NOTE: When running the application beyond the first time, if you use the directory as the initial run, the application will skip downloading the images that have already been downloaded.

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
- [ ] Publish to Crates.io

## Other Information

### Building for Raspberry Pi Zero W

To cross-compile for raspberry pi, this project uses [rust-cross](https://github.com/rust-embedded/cross). Make sure you have docker installed on your development machine.

1. Build the docker image for rust-cross: `docker build -t rust-rpi-zerow:v1-openssl -f Dockerfile.raspberrypizerow .`
2. Make sure that the image name used here matches the image name in your `Cross.toml` configuration
3. Run `cross build --target arm-unknown-linux-gnueabi --release` to build the project
4. You can find the compiled binary under `target/arm-unknown-linux-gnueabi/release/`
