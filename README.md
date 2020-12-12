# Reddsaver

Download images present in your saved posts on Reddit. Experimental. 
Validated to be working on macOS 10.15 using rust 1.47.0. 

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

 