use crate::auth::Auth;
use crate::errors::ReddSaverError;
use crate::structures::{UserAbout, UserSaved};
use crate::API_USER_AGENT;
use log::info;
use reqwest::header::USER_AGENT;
use std::borrow::Borrow;

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

    pub async fn saved(&self, limit: &i32) -> Result<Vec<UserSaved>, ReddSaverError> {
        let client = reqwest::Client::new();

        let mut complete = false;
        let mut processed = 0;
        let mut after: Option<String> = None;
        let mut saved: Vec<UserSaved> = vec![];

        while !complete {
            // during the first call to the API, we would not provide the after query parameter
            // in subsequent calls, we use the value for after from the response of the
            //  previous request and continue doing so till the value of after is null
            let url = if processed == 0 {
                format!("https://oauth.reddit.com/user/{}/saved", self.name)
            } else {
                format!(
                    "https://oauth.reddit.com/user/{}/saved?after={}",
                    self.name,
                    after.as_ref().unwrap()
                )
            };

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

            // total number of items processed by the method
            // note that not all of these items are images, so the downloaded images will be
            // lesser than or equal to the number of items present
            processed += response.borrow().data.dist;
            info!("Number of items processed : {}", processed);

            // if there is a response, continue collecting them into a vector
            if response.borrow().data.after.as_ref().is_none() {
                info!("Data gathering complete. Yay.");
                saved.push(response);
                complete = true;
            } else {
                info!(
                    "Processing till: {}",
                    response.borrow().data.after.as_ref().unwrap()
                );
                after = response.borrow().data.after.clone();
                saved.push(response);
            }
        }

        // return the vector the caller method for downloading
        Ok(saved)
    }
}
