use crate::auth::Auth;
use crate::errors::ReddSaverError;
use crate::structures::{Listing, UserAbout};
use log::{debug, info};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct User<'a> {
    /// Contains authentication information about the user
    auth: &'a Auth,
    /// Username of the user who authorized the application
    name: &'a str,
    /// Reqwest client
    session: &'a reqwest::Client,
}

#[derive(Debug)]
pub enum ListingType {
    Saved,
    Upvoted,
}

impl Display for ListingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            ListingType::Saved => write!(f, "saved"),
            ListingType::Upvoted => write!(f, "upvoted"),
        }
    }
}

impl<'a> User<'a> {
    pub fn new(auth: &'a Auth, name: &'a str, session: &'a reqwest::Client) -> Self {
        User { auth, name, session }
    }

    pub async fn about(&self) -> Result<UserAbout, ReddSaverError> {
        // all API requests that use a bearer token should be made to oauth.reddit.com instead
        let url = format!("https://oauth.reddit.com/user/{}/about", self.name);

        let response = self.session
            .get(&url)
            .bearer_auth(&self.auth.access_token)
            // reddit will forbid you from accessing the API if the provided user agent is not unique
            .send()
            .await?
            .json::<UserAbout>()
            .await?;

        debug!("About Response: {:#?}", response);

        Ok(response)
    }

    pub async fn listing(
        &self,
        listing_type: &ListingType,
    ) -> Result<Vec<Listing>, ReddSaverError> {

        let mut complete = false;
        let mut processed = 0;
        let mut after: Option<String> = None;
        let mut listing: Vec<Listing> = Vec::new();
        while !complete {
            // during the first call to the API, we would not provide the after query parameter
            // in subsequent calls, we use the value for after from the response of the
            //  previous request and continue doing so till the value of after is null
            let url = if processed == 0 {
                format!("https://oauth.reddit.com/user/{}/{}", self.name, listing_type.to_string())
            } else {
                format!(
                    "https://oauth.reddit.com/user/{}/{}?after={}",
                    self.name,
                    listing_type.to_string(),
                    after.as_ref().unwrap()
                )
            };

            let response = self.session
                .get(&url)
                .bearer_auth(&self.auth.access_token)
                // the maximum number of items returned by the API in a single request is 100
                .query(&[("limit", 100)])
                .send()
                .await?
                .json::<Listing>()
                .await?;

            // total number of items processed by the method
            // note that not all of these items are media, so the downloaded media will be
            // lesser than or equal to the number of items present
            processed += response.borrow().data.dist;
            info!("Number of items processed : {}", processed);

            // if there is a response, continue collecting them into a vector
            if response.borrow().data.after.as_ref().is_none() {
                info!("Data gathering complete. Yay.");
                listing.push(response);
                complete = true;
            } else {
                debug!("Processing till: {}", response.borrow().data.after.as_ref().unwrap());
                after = response.borrow().data.after.clone();
                listing.push(response);
            }
        }

        Ok(listing)
    }

    pub async fn undo(&self, name: &str, listing_type: &ListingType) -> Result<(), ReddSaverError> {
        let url: String;
        let mut map = HashMap::new();
        map.insert("id", name);

        match listing_type {
            ListingType::Upvoted => {
                url = format!("https://oauth.reddit.com/api/vote");
                map.insert("dir", "0");
            }
            ListingType::Saved => {
                url = format!("https://oauth.reddit.com/api/unsave");
            }
        }

        let response = self.session
            .post(&url)
            .bearer_auth(&self.auth.access_token)
            .form(&map)
            .send()
            .await?;

        debug!("Response: {:#?}", response);

        Ok(())
    }
}
