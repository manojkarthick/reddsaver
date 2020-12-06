mod auth;
mod errors;
mod structures;
mod user;
mod utils;

use crate::errors::ReddSaverError;
use crate::errors::ReddSaverError::DataDirNotFound;
use crate::structures::Summary;
use crate::user::User;
use crate::utils::{check_path_present, get_images_parallel, get_user_agent_string};
use auth::Client;
use dotenv::dotenv;
use env_logger::Env;
use log::{debug, info};
use std::env;
use std::ops::Add;

#[tokio::main]
async fn main() -> Result<(), ReddSaverError> {
    // initialize environment from the .env file
    dotenv().ok();

    // initialize logger for the app and set logging level to info if no environment variable present
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let client_id = env::var("CLIENT_ID")?;
    let client_secret = env::var("CLIENT_SECRET")?;
    let username = env::var("USERNAME")?;
    let password = env::var("PASSWORD")?;
    let user_agent = get_user_agent_string(None, None);
    let data_directory = env::var("DATA_DIR")?;

    if !check_path_present(&data_directory) {
        return Err(DataDirNotFound);
    }

    // login to reddit using the credentials provided and get API bearer token
    let auth = Client::new(
        &client_id,
        &client_secret,
        &username,
        &password,
        &user_agent,
    )
    .login()
    .await?;
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
    // get the saved posts for this particular user
    let saved_posts = user.saved().await?;
    debug!("Saved Posts: {:#?}", saved_posts);

    let mut full_summary = Summary {
        images_supported: 0,
        images_downloaded: 0,
        images_skipped: 0,
    };
    for collection in &saved_posts {
        full_summary = full_summary.add(get_images_parallel(&collection, &data_directory).await?);
    }

    info!("#####################################");
    info!("Download Summary:");
    info!(
        "Number of supported images: {}",
        full_summary.images_supported
    );
    info!(
        "Number of images downloaded: {}",
        full_summary.images_downloaded
    );
    info!("Number of images skipped: {}", full_summary.images_skipped);
    info!("#####################################");
    info!("FIN.");

    Ok(())
}
