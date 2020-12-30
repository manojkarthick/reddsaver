# Reddsaver ![build](https://github.com/manojkarthick/reddsaver/workflows/build/badge.svg)

* Download images present in your saved posts on Reddit 
* Experimental. You can download release binaries [here](https://github.com/manojkarthick/reddsaver/releases) 

Limitations: 
* Supports png/jpg images only
* Supports downloading images from reddit image galleries 
* Does not support downloading images from Imgur web posts (direct imgur image links will work)

## Instructions

### Pre-requisites
1. Create a new script application at https://www.reddit.com/prefs/apps
2. Copy the client ID and client secret information returned
3. Create a .env file with the following keys:  
```
CLIENT_ID=<client_id>
CLIENT_SECRET=<client_secret>
USERNAME=<username>
PASSWORD=<password>
```

When running the application beyond the first time, if you reuse the directory used to save images, the application will
 skip downloading the images already present. 


### Building and running from source

1. Build the application: `cargo build --release`
2. Run the application: `./target/release/reddsaver`

### Running with Docker

1. Pre-built docker images are available at https://hub.docker.com/u/manojkarthick
2. Currently using manual builds. Docker autobuild is still WIP

### Building for Raspberry Pi Zero W

To cross-compile for raspberry pi, this project uses [rust-cross](https://github.com/rust-embedded/cross). Make sure you have docker installed on your development machine.

1. Build the docker image for rust-cross: `docker build -t rust-rpi-zerow:v1-openssl -f Dockerfile.raspberrypizerow .`
2. Make sure that the image name used here matches the image name in your `Cross.toml` configuration
3. Run `cross build --target arm-unknown-linux-gnueabi --release` to build the project
4. You can find the compiled binary under `target/arm-unknown-linux-gnueabi/release/`

### Description and command line arguments

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

### Running with docker: 
```
mkdir -pv data/
docker build -t reddsaver:v0.1.0 .
docker run --rm \
    --volume="$PWD/data:/app/data" \
    --volume="$PWD/.env:/app/.env" \
    reddsaver:v0.1.0
```

 

### TODO
- [x] Separate dockerfiles
- [ ] Update logging to use `pretty_env_logger`
- [ ] Switch to `structopt` instead of `clap-rs`
- [ ] Option to toggle verbose mode for `clap`
- [ ] Stop on first skip
- [ ] Sample video
- [ ] Update email
- [ ] Publish to Crates.io
- [ ] Experimental GIF support
