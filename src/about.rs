use serde::{Deserialize, Serialize};

/// Data structure that represents a user's info
#[derive(Debug, Serialize, Deserialize)]
pub struct AboutData {
    /// Comment karma of the user
    pub comment_karma: i64,
    /// The time the user was created in seconds
    pub created: f64,
    /// I DON't KNoowW
    pub created_utc: f64,
    /// also don't know
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
    pub data: AboutData,
}
