use std::env;

use clap::{crate_version, Arg, ArgAction, Command};
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
            Arg::new("environment")
                .short('e')
                .long("from-env")
                .value_name("ENV_FILE")
                .help("Set a custom .env style file with secrets")
                .default_value(".env"),
        )
        .arg(
            Arg::new("data_directory")
                .short('d')
                .long("data-dir")
                .value_name("DATA_DIR")
                .help("Directory to save the media to")
                .default_value("data"),
        )
        .arg(
            Arg::new("show_config")
                .short('s')
                .long("show-config")
                .action(ArgAction::SetTrue)
                .help("Show the current config being used"),
        )
        .arg(
            Arg::new("dry_run")
                .short('r')
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Dry run and print the URLs of saved media to download"),
        )
        .arg(
            Arg::new("subreddits")
                .short('S')
                .long("subreddits")
                .action(ArgAction::Append)
                .value_name("SUBREDDITS")
                .value_delimiter(',')
                .help("Download media from these subreddits only"),
        )
        .arg(
            Arg::new("upvoted")
                .short('u')
                .long("upvoted")
                .action(ArgAction::SetTrue)
                .help("Download media from upvoted posts"),
        )
        .get_matches();

    let env_file = matches.get_one::<String>("environment").map(|s| s.as_str()).unwrap();
    let data_directory =
        String::from(matches.get_one::<String>("data_directory").map(|s| s.as_str()).unwrap());
    // generate the URLs to download from without actually downloading the media
    let should_download = !matches.get_flag("dry_run");
    // check if ffmpeg is present for combining video streams
    let ffmpeg_available = application_present(String::from("ffmpeg"));
    // check if yt-dlp is present on the system
    let ytdlp_available = application_present(String::from("yt-dlp"));
    // restrict downloads to these subreddits
    let subreddits: Option<Vec<&str>> =
        matches.get_many::<String>("subreddits").map(|vals| vals.map(|s| s.as_str()).collect());
    let upvoted = matches.get_flag("upvoted");
    let listing_type = if upvoted { &ListingType::Upvoted } else { &ListingType::Saved };

    // initialize environment from the .env file
    dotenvy::from_filename(env_file).ok();

    // initialize logger for the app and set logging level to info if no environment variable present
    let env = Env::default().filter("RS_LOG").default_filter_or("info");
    env_logger::Builder::from_env(env).init();

    let client_id = env::var("REDDSAVER_CLIENT_ID")?;
    let client_secret = env::var("REDDSAVER_CLIENT_SECRET")?;
    let username = env::var("REDDSAVER_USERNAME")?;
    let password = env::var("REDDSAVER_PASSWORD")?;
    let user_agent = get_user_agent_string(None, None);

    if !check_path_present(&data_directory) {
        return Err(DataDirNotFound);
    }

    // if the option is show-config, show the configuration and return immediately
    if matches.get_flag("show_config") {
        info!("Current configuration:");
        info!("ENVIRONMENT_FILE = {}", &env_file);
        info!("DATA_DIRECTORY = {}", &data_directory);
        info!("REDDSAVER_CLIENT_ID = {}", &client_id);
        info!("REDDSAVER_CLIENT_SECRET = {}", mask_sensitive(&client_secret));
        info!("REDDSAVER_USERNAME = {}", &username);
        info!("REDDSAVER_PASSWORD = {}", mask_sensitive(&password));
        info!("USER_AGENT = {}", &user_agent);
        info!("SUBREDDITS = {}", print_subreddits(&subreddits));
        info!("UPVOTED = {}", upvoted);
        info!("FFMPEG AVAILABLE = {}", ffmpeg_available);
        info!("YT-DLP AVAILABLE = {}", ytdlp_available);

        return Ok(());
    }

    if !ffmpeg_available {
        warn!(
            "No ffmpeg Installation available. \
            Videos hosted by Reddit use separate video and audio streams. \
            Ffmpeg needs be installed to combine the audio and video into a single mp4."
        );
    }

    if !ytdlp_available {
        warn!(
            "yt-dlp is not installed. Youtube videos will not be downloaded. \
            Install yt-dlp (https://github.com/yt-dlp/yt-dlp)."
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
        &listing,
        &data_directory,
        &subreddits,
        should_download,
        ffmpeg_available,
        ytdlp_available,
    );

    downloader.run().await?;

    Ok(())
}
