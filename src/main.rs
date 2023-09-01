use std::env;

use clap::{crate_version, Command, Arg};
use env_logger::Env;
use log::{debug, info, warn};

use auth::Client;

use crate::download::Downloader;
use crate::errors::ReddSaverError;
use crate::errors::ReddSaverError::DataDirNotFound;
use crate::user::{ListingType, User};
use crate::utils::*;

mod auth;
mod download;
mod errors;
mod structures;
mod user;
mod utils;

#[tokio::main]
async fn main() -> Result<(), ReddSaverError> {
    let matches = Command::new("ReddSaver")
        .version(crate_version!())
        .author("Manoj Karthick Selva Kumar")
        .about("Simple CLI tool to download saved media from Reddit")
        .arg(
            Arg::with_name("environment")
                .short("e")
                .long("from-env")
                .value_name("ENV_FILE")
                .help("Set a custom .env style file with secrets")
                .default_value(".env")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("data_directory")
                .short("d")
                .long("data-dir")
                .value_name("DATA_DIR")
                .help("Directory to save the media to")
                .default_value("data")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("show_config")
                .short("s")
                .long("show-config")
                .takes_value(false)
                .help("Show the current config being used"),
        )
        .arg(
            Arg::with_name("dry_run")
                .short("r")
                .long("dry-run")
                .takes_value(false)
                .help("Dry run and print the URLs of saved media to download"),
        )
        .arg(
            Arg::with_name("human_readable")
                .short("H")
                .long("human-readable")
                .takes_value(false)
                .help("Use human readable names for files"),
        )
        .arg(
            Arg::with_name("subreddits")
                .short("S")
                .long("subreddits")
                .multiple(true)
                .value_name("SUBREDDITS")
                .value_delimiter(",")
                .help("Download media from these subreddits only")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("upvoted")
                .short("u")
                .long("--upvoted")
                .takes_value(false)
                .help("Download media from upvoted posts"),
        )
        .arg(
            Arg::with_name("undo")
                .short("U")
                .long("undo")
                .takes_value(false)
                .help("Unsave or remote upvote for post after processing"),
        )
        .get_matches();

    let env_file = matches.value_of("environment").unwrap();
    let data_directory = String::from(matches.value_of("data_directory").unwrap());
    // generate the URLs to download from without actually downloading the media
    let should_download = !matches.is_present("dry_run");
    // check if ffmpeg is present for combining video streams
    let ffmpeg_available = application_present(String::from("ffmpeg"));
    // generate human readable file names instead of MD5 Hashed file names
    let use_human_readable = matches.is_present("human_readable");
    // restrict downloads to these subreddits
    let subreddits: Option<Vec<&str>> = if matches.is_present("subreddits") {
        Some(matches.values_of("subreddits").unwrap().collect())
    } else {
        None
    };
    let upvoted = matches.is_present("upvoted");
    let listing_type = if upvoted { &ListingType::Upvoted } else { &ListingType::Saved };

    let undo = matches.is_present("undo");

    // initialize environment from the .env file
    dotenv::from_filename(env_file).ok();

    // initialize logger for the app and set logging level to info if no environment variable present
    let env = Env::default().filter("RS_LOG").default_filter_or("info");
    env_logger::Builder::from_env(env).init();

    let client_id = env::var("CLIENT_ID")?;
    let client_secret = env::var("CLIENT_SECRET")?;
    let username = env::var("USERNAME")?;
    let password = env::var("PASSWORD")?;
    let user_agent = get_user_agent_string(None, None);

    if !check_path_present(&data_directory) {
        return Err(DataDirNotFound);
    }

    // if the option is show-config, show the configuration and return immediately
    if matches.is_present("show_config") {
        info!("Current configuration:");
        info!("ENVIRONMENT_FILE = {}", &env_file);
        info!("DATA_DIRECTORY = {}", &data_directory);
        info!("CLIENT_ID = {}", &client_id);
        info!("CLIENT_SECRET = {}", mask_sensitive(&client_secret));
        info!("USERNAME = {}", &username);
        info!("PASSWORD = {}", mask_sensitive(&password));
        info!("USER_AGENT = {}", &user_agent);
        info!("SUBREDDITS = {}", print_subreddits(&subreddits));
        info!("UPVOTED = {}", upvoted);
        info!("UNDO = {}", undo);
        info!("FFMPEG AVAILABLE = {}", ffmpeg_available);

        return Ok(());
    }

    if !ffmpeg_available {
        warn!(
            "No ffmpeg Installation available. \
            Videos hosted by Reddit use separate video and audio streams. \
            Ffmpeg needs be installed to combine the audio and video into a single mp4."
        );
    }

    // login to reddit using the credentials provided and get API bearer token
    let auth =
        Client::new(&client_id, &client_secret, &username, &password, &user_agent).login().await?;
    info!("Successfully logged in to Reddit as {}", username);
    debug!("Authentication details: {:#?}", auth);

    // get information about the user to display
    let user = User::new(&auth, &username);

    let user_info = user.about().await?;
    info!("The user details are: ");
    info!("Account name: {:#?}", user_info.data.name);
    info!("Account ID: {:#?}", user_info.data.id);
    info!("Comment Karma: {:#?}", user_info.data.comment_karma);
    info!("Link Karma: {:#?}", user_info.data.link_karma);

    info!("Starting data gathering from Reddit. This might take some time. Hold on....");
    // get the saved/upvoted posts for this particular user
    let listing = user.listing(listing_type).await?;
    debug!("Posts: {:#?}", listing);

    let downloader = Downloader::new(
        &user,
        &listing,
        &listing_type,
        &data_directory,
        &subreddits,
        should_download,
        use_human_readable,
        undo,
        ffmpeg_available,
    );

    downloader.run().await?;

    Ok(())
}
