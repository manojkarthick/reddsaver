use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct UserSaved {
    pub kind: String,
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
    pub kind: String,
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
