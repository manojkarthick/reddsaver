mod about;
mod auth;
mod errors;
mod saved;
mod user;
mod utils;

use crate::errors::ReddSaverError;
use crate::user::User;
use crate::utils::get_images_parallel;
use auth::Client;
use dotenv::dotenv;
use env_logger::Env;
use log::{debug, info};
use std::env;

// *Features to add:*
//
// todo: logging
// todo: restart later? (or ignore if saved)
// todo: iterator + limits + pagination (current max is 100)
// todo: generic thing struct
// todo: add rust
// todo: github actions CI
// todo: github artifacts
// todo: publish to crates.io
// todo: Dockerfile
// todo: Thread safe counters
// todo: Documentation
// todo: license
// todo: readme
// todo: test?
// todo: nix?
// todo: progress bar?

static API_USER_AGENT: &str = "com.manojkarthick.reddsaver:v0.0.1";

#[tokio::main]
async fn main() -> Result<(), ReddSaverError> {
    dotenv().ok();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let client_id = env::var("CLIENT_ID")?;
    let client_secret = env::var("CLIENT_SECRET")?;
    let username = env::var("USERNAME")?;
    let password = env::var("PASSWORD")?;
    let user_agent = String::from(API_USER_AGENT);
    let num_images: i32 = env::var("NUM_IMAGES")?.parse()?;

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

    let user = User::new(&auth, &username);

    let user_info = user.about().await?;
    info!("The user details are: ");
    info!("Account name: {:#?}", user_info.data.name);
    info!("Account ID: {:#?}", user_info.data.id);
    info!("Comment Karma: {:#?}", user_info.data.comment_karma);
    info!("Link Karma: {:#?}", user_info.data.link_karma);

    let saved_posts = user.saved(&num_images).await?;
    debug!("Saved posts: {:#?}", saved_posts);

    // 9s
    get_images_parallel(&saved_posts).await?;

    Ok(())
}
