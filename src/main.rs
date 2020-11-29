mod auth;
mod about;
mod user;
mod saved;
mod utils;
mod errors;

use dotenv::dotenv;
use std::env;
use auth::Client;
use crate::user::User;
use crate::utils::get_images_parallel;
use crate::errors::ReddSaverError;

// todo: logging
// todo: restart later? (or ignore if saved)
// todo: limits + pagination (current max is 100)
// todo: generic thing struct

static API_USER_AGENT: &str = "com.manojkarthick.reddsaver:v0.0.1";

#[tokio::main]
async fn main() -> Result<(), ReddSaverError>{

    dotenv().ok();

    let client_id = env::var("CLIENT_ID")?;
    let client_secret = env::var("CLIENT_SECRET")?;
    let username = env::var("USERNAME")?;
    let password = env::var("PASSWORD")?;
    let user_agent = String::from(API_USER_AGENT);
    let num_images: i32 = env::var("NUM_IMAGES")?.parse()?;


    let auth = Client::new(client_id, client_secret, username, password, user_agent).login().await?;
    println!("Successfully logged in!");
    println!("{:#?}", auth);

    let username = String::from("mellinam");
    let mellinam = User::new(&auth, username);

    let user_info = mellinam.about().await?;
    println!("{:#?}", user_info.data.name);

    let saved_posts = mellinam.saved(&num_images).await?;
    println!("{:#?}", saved_posts);

    // 9s
    get_images_parallel(&saved_posts).await?;

    Ok(())
}
