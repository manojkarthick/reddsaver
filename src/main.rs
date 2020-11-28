mod auth;

use dotenv::dotenv;
use std::env;
use auth::Client;

// TODO: error handling

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{

    dotenv().ok();

    let client_id = env::var("CLIENT_ID")?;
    let client_secret = env::var("CLIENT_SECRET")?;
    let username = env::var("USERNAME")?;
    let password = env::var("PASSWORD")?;
    let user_agent = env::var("USERAGENT")?;


    let auth = Client::new(client_id, client_secret, username, password, user_agent).login().await?;
    println!("Successfully logged in!");
    println!("{:#?}", auth);

    Ok(())
}
