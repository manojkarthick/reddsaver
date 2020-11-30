use crate::errors::ReddSaverError;

use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Client<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    username: &'a str,
    password: &'a str,
    user_agent: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Auth {
    pub access_token: String,
    token_type: String,
    expires_in: i32,
    scope: String,
}

impl<'a> Client<'a> {
    pub fn new(
        id: &'a str,
        secret: &'a str,
        username: &'a str,
        password: &'a str,
        agent: &'a str,
    ) -> Self {
        Self {
            client_id: &id,
            client_secret: &secret,
            username: &username,
            password: &password,
            user_agent: &agent,
        }
    }

    pub async fn login(&self) -> Result<Auth, ReddSaverError> {
        let basic_token = base64::encode(format!("{}:{}", self.client_id, self.client_secret));
        let grant_type = String::from("password");

        let mut body = HashMap::new();
        body.insert("username", self.username);
        body.insert("password", self.password);
        body.insert("grant_type", &grant_type);

        let client = reqwest::Client::new();
        let auth = client
            .post("https://www.reddit.com/api/v1/access_token")
            .header(USER_AGENT, self.user_agent)
            .header(AUTHORIZATION, format!("Basic {}", basic_token))
            .form(&body)
            .send()
            .await?
            .json::<Auth>()
            .await?;

        Ok(auth)
    }
}
