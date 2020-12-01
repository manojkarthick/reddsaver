use crate::auth::Auth;
use crate::errors::ReddSaverError;
use crate::structures::{UserAbout, UserSaved};
use crate::API_USER_AGENT;
use log::{debug, info};
use reqwest::header::USER_AGENT;
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;

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

            processed += response.borrow().data.dist;
            info!("Number of items processed : {}", processed);

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
                // info!("Completed?????: {}", &complete);
                saved.push(response);
            }
        }

        Ok(saved)
    }
}
