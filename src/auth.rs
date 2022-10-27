use crate::errors::ReddSaverError;

use log::debug;
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// To generate the Reddit Client ID and secret, go to reddit [preferences](https://www.reddit.com/prefs/apps)
pub struct Client<'a> {
    /// Client ID for the application
    client_id: &'a str,
    /// Client Secret for the application
    client_secret: &'a str,
    /// Login username
    username: &'a str,
    /// Login password
    password: &'a str,
    /// Reqwest client
    session: &'a reqwest::Client
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Auth {
    /// The generated bearer access token for the application
    pub access_token: String,
    /// Type of access token: "bearer"
    token_type: String,
    /// Expiry duration. Defaults to 3600/1 hour
    expires_in: i32,
    /// Scope of the access token. This app requires * scope
    scope: String,
}

impl<'a> Client<'a> {
    pub fn new(
        id: &'a str,
        secret: &'a str,
        username: &'a str,
        password: &'a str,
        session: &'a reqwest::Client,
    ) -> Self {
        Self {
            client_id: &id,
            client_secret: &secret,
            username: &username,
            password: &password,
            session: &session
        }
    }

    pub async fn login(&self) -> Result<Auth, ReddSaverError> {
        let basic_token = base64::encode(format!("{}:{}", self.client_id, self.client_secret));
        let grant_type = String::from("password");

        let mut body = HashMap::new();
        body.insert("username", self.username);
        body.insert("password", self.password);
        body.insert("grant_type", &grant_type);

        let auth = self.session
            .post("https://www.reddit.com/api/v1/access_token")
            // base64 encoded <clientID>:<clientSecret> should be sent as a basic token
            // along with the body of the message
            .header(AUTHORIZATION, format!("Basic {}", basic_token))
            // make sure the username and password is sent as form encoded values
            // the API does not accept JSON body when trying to obtain a bearer token
            .form(&body)
            .send()
            .await?
            .json::<Auth>()
            .await?;

        debug!("Access token is: {}", auth.access_token);
        Ok(auth)
    }
}
