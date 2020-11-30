use crate::auth::Auth;
use crate::errors::ReddSaverError;
use crate::structures::{UserAbout, UserSaved};
use crate::API_USER_AGENT;
use reqwest::header::USER_AGENT;

pub struct User<'a> {
    /// Contains authentication information about the user
    auth: &'a Auth,
    /// Username of the user who authorized the application
    name: &'a str,
}

impl<'a> User<'a> {
    pub fn new(auth: &'a Auth, name: &'a str) -> Self {
        User { auth, name }
    }

    pub async fn about(&self) -> Result<UserAbout, ReddSaverError> {
        // all API requests that use a bearer token should be made to oauth.reddit.com instead
        let url = format!("https://oauth.reddit.com/user/{}/about", self.name);
        let client = reqwest::Client::new();

        let response = client
            .get(&url)
            .bearer_auth(&self.auth.access_token)
            // reddit will forbid you from accessing the API if the provided user agent is not unique
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
            // pass a limit to the API if provided by the user
            // currently the API returns a maximum of 100 posts in a single request
            // todo: add options to exit prematurely if asked for?
            // todo: get all posts by iterating
            .query(&[("limit", limit)])
            .send()
            .await?
            .json::<UserSaved>()
            .await?;

        Ok(response)
    }
}
