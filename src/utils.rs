use crate::errors::ReddSaverError;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use std::path::Path;
use std::str::FromStr;
use std::env;
use which::which;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Generate user agent string of the form <name>:<version>.
/// If no arguments passed generate random name and number
pub fn get_user_agent_string(username: &str) -> String {
    format!("reddsaver:{} (by u/{}", VERSION, username)
}

/// Check if a particular path is present on the filesystem
pub fn check_path_present(file_path: &str) -> bool {
    Path::new(file_path).exists()
}

/// Function that masks sensitive data such as password and client secrets
pub fn mask_sensitive(word: &str) -> String {
    let word_length = word.len();
    return if word.is_empty() {
        // return with indication if string is empty
        String::from("<EMPTY>")
    } else if word_length > 0 && word_length <= 3 {
        // if string length is between 1-3, mask all characters
        "*".repeat(word_length)
    } else {
        // if string length greater than 5, mask all characters
        // except the first two and the last characters
        word.chars()
            .enumerate()
            .map(|(i, c)| if i == 0 || i == 1 || i == word_length - 1 { c } else { '*' })
            .collect()
    };
}

/// Return delimited subreddit names or EMPTY if None
pub fn print_subreddits(subreddits: &Option<Vec<&str>>) -> String {
    return if let Some(s) = subreddits { s.join(",") } else { String::from("<ALL>") };
}

/// Check if the given application is present in the $PATH
pub fn application_present(name: String) -> bool {
    let result = which(name);
    match result {
        Ok(_) => true,
        _ => false,
    }
}

/// Check if the given URL contains an MP4 track using the content type
pub async fn check_url_is_mp4(url: &str) -> Result<Option<bool>, ReddSaverError> {
    let response = reqwest::get(url).await?;
    let headers = response.headers();

    match headers.get(CONTENT_TYPE) {
        None => Ok(None),
        Some(content_type) => {
            let content_type = Mime::from_str(content_type.to_str()?)?;
            let is_video = match (content_type.type_(), content_type.subtype()) {
                (mime::VIDEO, mime::MP4) => true,
                (mime::APPLICATION, mime::XML) => false,
                _ => false,
            };
            Ok(Some(is_video))
        }
    }
}
