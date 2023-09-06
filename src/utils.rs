use crate::errors::ReddSaverError;
use mime::Mime;
use rand::Rng;
use random_names::RandomName;
use reqwest::header::CONTENT_TYPE;
// use reqwest::Client;
use serde_json::Value;
// use serde::{Deserialize, Serialize};
// use regex::Regex;
use log::debug;
use std::path::Path;
use std::str::FromStr;
use which::which;

static LOC_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.15";
static RG_API_URL: &str = "https://api.redgifs.com/v2/auth/temporary";
static RG_GIFLOC_URL: &str = "https://api.redgifs.com/v2/gifs";
//static REDGIFS_DOMAIN: &str = "redgifs.com";

/// Generate user agent string of the form <name>:<version>.
/// If no arguments passed generate random name and number
pub fn get_user_agent_string(name: Option<String>, version: Option<String>) -> String {
    if let (Some(v), Some(n)) = (version, name) {
        format!("{}:{}", n, v)
    } else {
        let random_name = RandomName::new().to_string().replace(" ", "").to_lowercase();

        let mut rng = rand::thread_rng();
        let random_version = rng.gen::<u32>();
        format!("{}:{}", random_name, random_version)
    }
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

/// Since we had to convert the set of subreddits into a vec, we'll coerce them
///   back into an Option.
pub fn coerce_subreddits(subreddits: Vec<&str>) -> Option<Vec<&str>> {
    if subreddits.len() > 0 {
        Some(subreddits)
    } else {
        None
    }
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

/// New RedGifs API requires fetching an auth token first
pub async fn fetch_redgif_token() -> Result<String, ReddSaverError> {
    //let user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.15";
    let response = reqwest::Client::new()
        .get(RG_API_URL)
        .header("User-Agent", LOC_AGENT)
        .send().await?.text().await?;
    let resp_data: Value = serde_json::from_str(&response).unwrap();
    let tok_val = resp_data["token"].as_str();
    let token = match tok_val {
        Some(t) => t,
        None => return Err(ReddSaverError::CouldNotSaveImageError("".to_string())),
    };
    Ok(token.to_string())
}

pub async fn fetch_redgif_url(orig_url: &str) -> reqwest::Result<reqwest::Response> {
    debug!("Original URL: {}", orig_url);
    let re = regex::Regex::new(r".*redgifs.com*/(?P<token>[^-]+)\-?.*\.(?P<ext>[a-z0-9]{3,4})").unwrap();
    let caps = re.captures(orig_url).unwrap();

    let title = caps.name("token").map_or("", |m| m.as_str());
    debug!("Token: {}", title);
    //let extension = caps.name("ext").map_or("", |n| n.as_str());
    // So now we've gone from the original url to 'whisperedfunbunny' and 'mp4'
    let gifloc = format!("{}/{}", RG_GIFLOC_URL, &title.to_lowercase());
    debug!("Gifloc: {}", gifloc);
    let rgtoken = format!("Bearer {}", fetch_redgif_token().await.unwrap());
    debug!("RGToken: {}", rgtoken);
    let response = reqwest::Client::new()
    .get(&gifloc)
    .header("User-Agent", LOC_AGENT)
    .header("Authorization", &rgtoken)
    .send().await?.text().await?;
    let resp_data: Value = serde_json::from_str(&response).unwrap();
    let final_url = resp_data["gif"]["urls"]["hd"].as_str().unwrap();
    reqwest::Client::new()
    .get(final_url)
    .header("User-Agent", LOC_AGENT)
    .header("Authorization", &rgtoken)
    .send().await
}
