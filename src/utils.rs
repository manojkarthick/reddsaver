use crate::errors::ReddSaverError;
use log::{debug, warn};
use mime::Mime;
use rand::Rng;
use random_names::RandomName;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;
use std::path::Path;
use std::str::FromStr;
use which::which;

// RedGifs requires the same User-Agent for both token fetch and media fetch calls
static LOC_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.15";
// RedGifs v2 auth endpoint — returns a temporary bearer token
static RG_API_URL: &str = "https://api.redgifs.com/v2/auth/temporary";
// RedGifs v2 gif-info endpoint — returns JSON with the actual HD mp4 URL
static RG_GIFLOC_URL: &str = "https://api.redgifs.com/v2/gifs";

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

/// Fetch a temporary bearer token from the RedGifs v2 auth endpoint
pub async fn fetch_redgif_token() -> Result<String, ReddSaverError> {
    let response = reqwest::Client::new()
        .get(RG_API_URL)
        .header("User-Agent", LOC_AGENT)
        .send()
        .await?
        .text()
        .await?;
    let resp_data: Value = serde_json::from_str(&response).unwrap();
    let tok_val = resp_data["token"].as_str();
    let token = match tok_val {
        Some(t) => t,
        None => return Err(ReddSaverError::CouldNotSaveImageError("".to_string())),
    };
    Ok(format!("Bearer {}", token))
}

/// Resolve a RedGifs URL to the actual HD mp4 response using the v2 API.
/// Returns Ok(None) if the gif is unavailable (deleted or private), with a
/// warning already emitted — the caller should skip without further logging.
pub async fn fetch_redgif_url(rg_token: &str, orig_url: &str) -> Result<Option<reqwest::Response>, ReddSaverError> {
    // RedGifs has two URL styles:
    //   older: thumbs44.redgifs.com/Token-mobile.mp4?hash=…
    //   newer: thumbs44.redgifs.com/watch/tokenname  OR  redgifs.com/watch/tokenname
    let re_old = regex::Regex::new(r"/([A-Za-z]+)-mobile\.mp4").unwrap();
    let re_new = regex::Regex::new(r"/(?:watch/)?([A-Za-z]+)(?:[^/]*)$").unwrap();

    let gif_id = if let Some(caps) = re_old.captures(orig_url) {
        caps[1].to_string()
    } else if let Some(caps) = re_new.captures(orig_url) {
        caps[1].to_string()
    } else {
        String::new()
    };

    debug!("RedGifs gif ID: {}", gif_id);

    let client = reqwest::Client::new();
    let api_url = format!("{}/{}", RG_GIFLOC_URL, gif_id.to_lowercase());
    debug!("RedGifs API URL: {}", api_url);

    let api_resp = client
        .get(&api_url)
        .header("User-Agent", LOC_AGENT)
        .header("Authorization", rg_token)
        .send()
        .await?
        .text()
        .await?;

    let api_data: Value = serde_json::from_str(&api_resp).unwrap_or(Value::Null);
    let hd_url = match api_data["gif"]["urls"]["hd"].as_str() {
        Some(u) => u.to_string(),
        None => {
            warn!("Skipping RedGifs URL {}: no HD URL in API response (gif may be deleted or private)", orig_url);
            return Ok(None);
        }
    };
    debug!("RedGifs HD URL: {}", hd_url);

    let response = client
        .get(&hd_url)
        .header("User-Agent", LOC_AGENT)
        .header("Authorization", rg_token)
        .send()
        .await?;
    Ok(Some(response))
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
