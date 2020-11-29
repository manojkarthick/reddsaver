use std::collections::HashMap;
use reqwest::header::{USER_AGENT, AUTHORIZATION};
use base64;
use serde::{Deserialize, Serialize};


pub struct Client {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
    user_agent: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Auth {
    pub access_token: String,
    token_type: String,
    expires_in: i32,
    scope: String,
}

impl Client {

    pub fn new(id: String, secret: String, username: String, password: String, agent: String) -> Self {
        Self{
            client_id: id,
            client_secret: secret,
            username,
            password,
            user_agent: agent,
        }
    }

    pub async fn login(&self) -> Result<Auth, Box<dyn std::error::Error>> {

        let basic_token = base64::encode(format!("{}:{}", self.client_id, self.client_secret));
        let grant_type = String::from("password");

        let mut body = HashMap::new();
        body.insert("username", &self.username);
        body.insert("password", &self.password);
        body.insert("grant_type", &grant_type);

        let client = reqwest::Client::new();
        let auth = client.post("https://www.reddit.com/api/v1/access_token")
            .header(USER_AGENT, &self.user_agent)
            .header(AUTHORIZATION, format!("Basic {}", basic_token))
            .form(&body)
            .send()
            .await?
            .json::<Auth>()
            .await?;

        Ok(auth)

    }
}
