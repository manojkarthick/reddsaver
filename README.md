# Reddsaver

Download images present in your saved posts on Reddit. Experimental. 
Validated to be working on macOS 10.15 using rust 1.47.0. 

Limitations: 
* Supports png/jpg images only 
* Does not support downloading images from Imgur web posts (direct imgur image links will work)
* Does not support downloading images from reddit image galleries

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
DATA_DIR=<directory_to_save_images_to>
```

When running the application beyond the first time, if you reuse the directory used to save images, the application will
 skip downloading the images already present. 


### Building and running from source

1. Build the application: `cargo build --release`
2. Run the application: `./target/release/reddsaver`


### Running with docker: 
```
mkdir -pv data/
docker build -t reddsaver:v0.1.0 .
docker run --rm \
    --volume="$PWD/data:/app/data" \
    --volume="$PWD/.env:/app/.env" \
    reddsaver:v0.1.0
```

 