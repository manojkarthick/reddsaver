use crate::about::UserAbout;
use crate::auth::Auth;
use crate::errors::ReddSaverError;
use crate::saved::UserSaved;
use crate::API_USER_AGENT;
use reqwest::header::USER_AGENT;

pub struct User<'a> {
    auth: &'a Auth,
    name: &'a str,
}

impl<'a> User<'a> {
    pub fn new(auth: &'a Auth, name: &'a str) -> Self {
        User { auth, name }
    }

    pub async fn about(&self) -> Result<UserAbout, ReddSaverError> {
        let url = format!("https://oauth.reddit.com/user/{}/about", self.name);
        let client = reqwest::Client::new();

        let response = client
            .get(&url)
            .bearer_auth(&self.auth.access_token)
            .header(USER_AGENT, API_USER_AGENT)
            .send()
            .await?
            .json::<UserAbout>()
            .await?;

        Ok(response)
    }

    pub async fn saved(&self, limit: &i32) -> Result<UserSaved, ReddSaverError> {
        let url = format!("https://oauth.reddit.com/user/{}/saved", self.name);
        let client = reqwest::Client::new();

        let response = client
            .get(&url)
            .bearer_auth(&self.auth.access_token)
            .header(USER_AGENT, API_USER_AGENT)
            .query(&[("limit", limit)])
            .send()
            .await?
            .json::<UserSaved>()
            .await?;

        Ok(response)
    }
}
