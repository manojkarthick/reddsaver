use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Data structure that represents a user's info
#[derive(Debug, Serialize, Deserialize)]
pub struct AboutData {
    /// Comment karma of the user
    pub comment_karma: i64,
    /// The time the user was created in seconds
    pub created: f64,
    /// The time the user was created in seconds (UTC)
    pub created_utc: f64,
    /// Undocumented
    pub has_subscribed: bool,
    /// Whether the user has verified their email
    pub has_verified_email: bool,
    /// Don't know
    pub hide_from_robots: bool,
    /// The id of the user
    pub id: String,
    /// Whether the user is a Reddit employee
    pub is_employee: bool,
    /// Whether the user is friend of the current user
    pub is_friend: bool,
    /// Whether the user has Reddit gold or not
    pub is_gold: bool,
    /// Whether the user is a moderator
    pub is_mod: bool,
    /// Link karma of the user
    pub link_karma: i64,
    /// The user's username
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAbout {
    /// The kind of object this is. eg: Comment, Account, Subreddit, etc.
    pub kind: String,
    /// Contains data about the reddit user
    pub data: AboutData,
}

#[derive(Deserialize, Debug)]
pub struct UserSaved {
    /// The kind of object this is. eg: Comment, Account, Subreddit, etc.
    pub kind: String,
    /// Contains the data for the children of the listing.
    /// Listings are collections of data. For example, saved posts, hot posts in a subreddit
    pub data: ListingData,
}

/// The contents of a call to a 'listing' endpoint.
#[derive(Deserialize, Debug)]
pub struct ListingData {
    /// A modhash (essentially a CSRF token) generated for this request. This is generally
    /// not required for any use-case, but is provided nevertheless.
    pub modhash: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub children: Vec<Post>,
    pub dist: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Post {
    /// The kind of object this is. eg: Comment, Account, Subreddit, etc.
    pub kind: String,
    /// Contains data about this particular reddit post
    pub data: PostData,
}

/// Represents all types of link posts and self posts on Reddit.
#[derive(Deserialize, Debug, Clone)]
pub struct PostData {
    pub subreddit: String,
    /// The ID of the post in base-36 form, as used in Reddit's links.
    pub id: String,
    /// The overall points score of this post, as shown on the upvote counter. This is the
    /// same as upvotes - downvotes (however, this figure may be fuzzed by Reddit, and may not
    /// be exact)
    pub score: i64,
    /// The URL to the link thumbnail. This is "self" if this is a self post, or "default" if
    /// a thumbnail is not available.
    pub thumbnail: Option<String>,
    /// The Reddit ID for the subreddit where this was posted, **including the leading `t5_`**.
    pub subreddit_id: String,
    /// True if the logged-in user has saved this submission.
    pub saved: bool,
    /// The permanent, long link for this submission.
    pub permalink: String,
    /// The full 'Thing ID', consisting of a 'kind' and a base-36 identifier. The valid kinds are:
    /// - t1_ - Comment
    /// - t2_ - Account
    /// - t3_ - Link
    /// - t4_ - Message
    /// - t5_ - Subreddit
    /// - t6_ - Award
    /// - t8_ - PromoCampaign
    pub name: String,
    /// A timestamp of the time when the post was created, in the logged-in user's **local**
    /// time.
    pub created: Value,
    /// The linked URL, if this is a link post.
    pub url: Option<String>,
    /// The title of the post.
    pub title: Option<String>,
    /// A timestamp of the time when the post was created, in **UTC**.
    pub created_utc: Value,
}
