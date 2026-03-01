use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;

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

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Listing {
    /// The kind of object this is. eg: Comment, Account, Subreddit, etc.
    pub kind: String,
    /// Contains the data for the children of the listing.
    /// Listings are collections of data. For example, saved posts, hot posts in a subreddit
    pub data: ListingData,
}

/// The contents of a call to a 'listing' endpoint.
#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Post {
    /// The kind of object this is. eg: Comment, Account, Subreddit, etc.
    pub kind: String,
    /// Contains data about this particular reddit post
    pub data: PostData,
}

/// Represents all types of link posts and self posts on Reddit.
#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct PostData {
    pub subreddit: String,
    /// The Reddit username of the post author.
    pub author: String,
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
    /// Gallery metadata
    pub gallery_data: Option<GalleryItems>,
    /// Per-item metadata for gallery posts, keyed by media_id.
    /// Each entry contains (among other fields) "m": the MIME type (e.g. "image/jpg").
    pub media_metadata: Option<HashMap<String, Value>>,
    /// Is post a video?
    pub is_video: Option<bool>,
    /// Reddit Media info
    pub media: Option<PostMedia>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PostMedia {
    pub reddit_video: Option<RedditVideo>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct RedditVideo {
    pub fallback_url: String,
    pub is_gif: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GalleryItems {
    /// Representation containing a list of gallery items
    pub items: Vec<GalleryItem>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct GalleryItem {
    /// The reddit media id, can be used to construct a redd.it URL
    pub media_id: String,
    /// Unique numerical ID for the specific media item
    pub id: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GfyData {
    #[serde(rename = "gfyItem")]
    pub gfy_item: GfyItem,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct GfyItem {
    #[serde(rename = "gifUrl")]
    pub gif_url: String,
    #[serde(rename = "mp4Url")]
    pub mp4_url: String,
}

/// Per-source download statistics
#[derive(Debug, Clone, Default)]
pub struct SourceStats {
    pub supported: i32,
    pub downloaded: i32,
    pub skipped: i32,
}

#[derive(Debug, Clone)]
pub struct Summary {
    /// Number of media downloaded
    pub media_downloaded: i32,
    /// Number of media skipping downloading
    pub media_skipped: i32,
    /// Number of media supported present and parsable
    pub media_supported: i32,
    /// Per-source breakdown
    pub by_source: HashMap<String, SourceStats>,
}

impl Summary {
    pub fn zero() -> Self {
        Self {
            media_downloaded: 0,
            media_skipped: 0,
            media_supported: 0,
            by_source: HashMap::new(),
        }
    }
}

impl Add for Summary {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut by_source = self.by_source;
        for (source, stats) in rhs.by_source {
            let entry = by_source.entry(source).or_default();
            entry.supported += stats.supported;
            entry.downloaded += stats.downloaded;
            entry.skipped += stats.skipped;
        }
        Self {
            media_supported: self.media_supported + rhs.media_supported,
            media_downloaded: self.media_downloaded + rhs.media_downloaded,
            media_skipped: self.media_skipped + rhs.media_skipped,
            by_source,
        }
    }
}
