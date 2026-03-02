use crate::auth::Auth;
use crate::errors::ReddSaverError;
use crate::structures::{Listing, UserAbout};
use crate::utils::get_user_agent_string;
use log::{debug, info};
use reqwest::header::USER_AGENT;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct User<'a> {
    /// Contains authentication information about the user
    auth: &'a Auth,
    /// Username of the user who authorized the application
    name: &'a str,
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

/// Top-level operation mode selected via --mode.
#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
pub enum Mode {
    Saved,
    Upvoted,
    Feed,
    User,
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Saved => write!(f, "saved"),
            Mode::Upvoted => write!(f, "upvoted"),
            Mode::Feed => write!(f, "feed"),
            Mode::User => write!(f, "user"),
        }
    }
}

/// Sort order for subreddit feed listings.
#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
pub enum SubredditSort {
    Hot,
    Best,
    Rising,
    Top,
    New,
    Controversial,
}

impl Display for SubredditSort {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            SubredditSort::Hot => write!(f, "hot"),
            SubredditSort::Best => write!(f, "best"),
            SubredditSort::Rising => write!(f, "rising"),
            SubredditSort::Top => write!(f, "top"),
            SubredditSort::New => write!(f, "new"),
            SubredditSort::Controversial => write!(f, "controversial"),
        }
    }
}

/// Time period filter for top/controversial subreddit listings.
#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
pub enum TimePeriod {
    Hour,
    Day,
    Week,
    Month,
    Year,
    All,
}

impl Display for TimePeriod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            TimePeriod::Hour => write!(f, "hour"),
            TimePeriod::Day => write!(f, "day"),
            TimePeriod::Week => write!(f, "week"),
            TimePeriod::Month => write!(f, "month"),
            TimePeriod::Year => write!(f, "year"),
            TimePeriod::All => write!(f, "all"),
        }
    }
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
            .header(USER_AGENT, get_user_agent_string(None, None))
            .send()
            .await?
            .json::<UserAbout>()
            .await?;

        debug!("About Response: {:#?}", response);

        Ok(response)
    }

    /// Fetch saved or upvoted posts for the authenticated user.
    ///
    /// `limit` caps the total number of posts returned. Pass `None` to fetch all
    /// available posts (the original behaviour).
    pub async fn listing(
        &self,
        listing_type: &ListingType,
        limit: Option<usize>,
    ) -> Result<Vec<Listing>, ReddSaverError> {
        let client = reqwest::Client::new();

        let mut complete = false;
        let mut processed: usize = 0;
        let mut after: Option<String> = None;
        let mut listing: Vec<Listing> = Vec::new();
        while !complete {
            // during the first call to the API, we would not provide the after query parameter
            // in subsequent calls, we use the value for after from the response of the
            //  previous request and continue doing so till the value of after is null
            let base_url = format!("https://oauth.reddit.com/user/{}/{}", self.name, listing_type);

            let mut request = client
                .get(&base_url)
                .bearer_auth(&self.auth.access_token)
                .header(USER_AGENT, get_user_agent_string(None, None))
                // the maximum number of items returned by the API in a single request is 100
                .query(&[("limit", "100")]);

            if let Some(ref a) = after {
                request = request.query(&[("after", a)]);
            }

            let mut response = request
                .send()
                .await?
                .json::<Listing>()
                .await?;

            let page_count = response.data.dist as usize;

            // if a limit is set and this page would exceed it, trim the children
            if let Some(cap) = limit {
                let remaining = cap.saturating_sub(processed);
                if page_count > remaining {
                    response.data.children.truncate(remaining);
                    response.data.dist = remaining as i32;
                }
            }

            processed += response.data.dist as usize;
            info!("Number of items processed : {}", processed);

            let hit_limit = limit.map(|cap| processed >= cap).unwrap_or(false);

            // if there is a response, continue collecting them into a vector
            match response.data.after {
                None => {
                    info!("Data gathering complete. Yay.");
                    listing.push(response);
                    complete = true;
                }
                Some(ref next) if !hit_limit => {
                    debug!("Processing till: {}", next);
                    after = Some(next.clone());
                    listing.push(response);
                }
                _ => {
                    info!("Data gathering complete. Yay.");
                    listing.push(response);
                    complete = true;
                }
            }
        }

        Ok(listing)
    }

    /// Fetch posts from a subreddit's feed.
    ///
    /// `sort` selects the listing type (hot, best, rising, top, new, controversial).
    /// `period` applies a time filter for `top` and `controversial` sorts.
    /// `limit` caps the total number of posts returned (per subreddit).
    pub async fn subreddit_listing(
        &self,
        subreddit: &str,
        sort: &SubredditSort,
        period: Option<&TimePeriod>,
        limit: usize,
    ) -> Result<Vec<Listing>, ReddSaverError> {
        let client = reqwest::Client::new();

        let mut complete = false;
        let mut processed: usize = 0;
        let mut after: Option<String> = None;
        let mut listing: Vec<Listing> = Vec::new();

        while !complete {
            let base_url = format!("https://oauth.reddit.com/r/{}/{}", subreddit, sort);

            let mut request = client
                .get(&base_url)
                .bearer_auth(&self.auth.access_token)
                .header(USER_AGENT, get_user_agent_string(None, None))
                .query(&[("limit", "100")]);

            if let Some(ref a) = after {
                request = request.query(&[("after", a)]);
            }

            // top and controversial support an optional time period filter
            if let Some(p) = period {
                match sort {
                    SubredditSort::Top | SubredditSort::Controversial => {
                        request = request.query(&[("t", p.to_string())]);
                    }
                    _ => {}
                }
            }

            let mut response = request.send().await?.json::<Listing>().await?;

            let page_count = response.data.dist as usize;
            let remaining = limit.saturating_sub(processed);
            if page_count > remaining {
                response.data.children.truncate(remaining);
                response.data.dist = remaining as i32;
            }

            processed += response.data.dist as usize;
            info!("Number of items processed from r/{}: {}", subreddit, processed);

            let hit_limit = processed >= limit;

            match response.data.after {
                None => {
                    info!("Data gathering complete for r/{}.", subreddit);
                    listing.push(response);
                    complete = true;
                }
                Some(ref next) if !hit_limit => {
                    debug!("Processing till: {}", next);
                    after = Some(next.clone());
                    listing.push(response);
                }
                _ => {
                    info!("Data gathering complete for r/{}.", subreddit);
                    listing.push(response);
                    complete = true;
                }
            }
        }

        Ok(listing)
    }

    /// Fetch submitted posts from a given Reddit user's profile.
    ///
    /// `target_user` is the username whose submissions to fetch.
    /// `sort` selects the listing type (hot, new, top, controversial).
    /// `period` applies a time filter for `top` and `controversial` sorts.
    /// `limit` caps the total number of posts returned.
    pub async fn user_listing(
        &self,
        target_user: &str,
        sort: &SubredditSort,
        period: Option<&TimePeriod>,
        limit: usize,
    ) -> Result<Vec<Listing>, ReddSaverError> {
        let client = reqwest::Client::new();

        let mut complete = false;
        let mut processed: usize = 0;
        let mut after: Option<String> = None;
        let mut listing: Vec<Listing> = Vec::new();

        while !complete {
            let base_url = format!(
                "https://oauth.reddit.com/user/{}/submitted",
                target_user
            );

            let mut request = client
                .get(&base_url)
                .bearer_auth(&self.auth.access_token)
                .header(USER_AGENT, get_user_agent_string(None, None))
                .query(&[("limit", "100")])
                .query(&[("sort", sort.to_string())]);

            if let Some(ref a) = after {
                request = request.query(&[("after", a)]);
            }

            if let Some(p) = period {
                match sort {
                    SubredditSort::Top | SubredditSort::Controversial => {
                        request = request.query(&[("t", p.to_string())]);
                    }
                    _ => {}
                }
            }

            let mut response = request.send().await?.json::<Listing>().await?;

            let page_count = response.data.dist as usize;
            let remaining = limit.saturating_sub(processed);
            if page_count > remaining {
                response.data.children.truncate(remaining);
                response.data.dist = remaining as i32;
            }

            processed += response.data.dist as usize;
            info!(
                "Number of items processed from u/{}: {}",
                target_user, processed
            );

            let hit_limit = processed >= limit;

            match response.data.after {
                None => {
                    info!("Data gathering complete for u/{}.", target_user);
                    listing.push(response);
                    complete = true;
                }
                Some(ref next) if !hit_limit => {
                    debug!("Processing till: {}", next);
                    after = Some(next.clone());
                    listing.push(response);
                }
                _ => {
                    info!("Data gathering complete for u/{}.", target_user);
                    listing.push(response);
                    complete = true;
                }
            }
        }

        Ok(listing)
    }
}
