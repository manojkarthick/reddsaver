mod auth;
mod about;
mod user;
mod saved;
mod utils;

use dotenv::dotenv;
use std::env;
use auth::Client;
use crate::user::User;
use crate::utils::get_images;

// TODO: error handling
// todo: logging
// todo: limits + pagination
// todo: restart later
// todo: async download loop

static API_USER_AGENT: &str = "com.manojkarthick.reddsaver:v0.0.1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{

    dotenv().ok();

    let client_id = env::var("CLIENT_ID")?;
    let client_secret = env::var("CLIENT_SECRET")?;
    let username = env::var("USERNAME")?;
    let password = env::var("PASSWORD")?;
    let user_agent = String::from(API_USER_AGENT);


    let auth = Client::new(client_id, client_secret, username, password, user_agent).login().await?;
    println!("Successfully logged in!");
    println!("{:#?}", auth);

    let username = String::from("mellinam");
    let mellinam = User::new(&auth, username);

    let user_info = mellinam.about().await?;
    println!("{:#?}", user_info.data.name);

    let saved_posts = mellinam.saved(&100).await?;

    get_images(&saved_posts).await?;

    Ok(())
}
