use crate::errors::ReddSaverError;
use mime::Mime;
use rand::Rng;
use random_names::RandomName;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;
use log::debug;
use std::path::Path;
use std::str::FromStr;
use which::which;

// Because the User_Agent field has to be the same every time you wield a RedGifs token,
//   we can use this static block to pass hash checks.
// todo: Combine this with the get_user_agent_string() function to make a single random agent string.
static LOC_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.15";
// This is the RedGifs API endpoint to call to fetch an authentication token
static RG_API_URL: &str = "https://api.redgifs.com/v2/auth/temporary";
// This is the one we'll call to pull the JSON location of the actual content
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
///   back into an Option. There's a more elegant way to do this, I'm sure of it.
pub fn coerce_subreddits(subreddits: Vec<&str>) -> Option<Vec<&str>> {
    if subreddits.len() > 0 && subreddits[0] != "" {
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

/// New RedGifs API requires fetching an auth token first, which is probably their way
///   of blocking mass content downloads and scrapers. Tokens seem to be good for two
///   weeks or so — this function grabs one and hands it back so that other things can
///   wield it. It's important to note that the browser User_Agent field has to match
///   when wielding the token — hence the use of a constant for that value.
pub async fn fetch_redgif_token() -> Result<String, ReddSaverError> {
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
    let fulltoken = format!("Bearer {}", token);
    Ok(fulltoken.to_string())
}

/// Fetching content from RedGifs is a circus of back and forth. You have to fetch an
///   auth token first (see fetch_redgif_token), then you use that token to call the API
///   which gives you a JSON blob that, when decoded, gives you the URL to fetch the actual
///   media from...all of which require using the same token and User_Agent field for every call.
/// It's creative, I'll give them that!
pub async fn fetch_redgif_url(rg_token: &str, orig_url: &str) -> reqwest::Result<reqwest::Response> {
    debug!("Original URL: {}", orig_url);
    let rex: &str;
    // Redgifs seems to have two url styles from saved posts:
    if orig_url.contains("?") {
        // This matches thumbs44.redgifs.com/ThisIsATokenName-mobile.mp4?hash=foo&thing=other
        //   (I think these are older)
        rex = r".*redgifs.com*\/(?P<token>[a-zA-Z0-9]+)\-.*\.[mp4gif]+\?.*";
    } else {
        // This matches newer(?) thumbs44.redgifs.com/watch/thisisatokenname
        rex = r".*redgifs.com*\/[a-zA-Z0-9]+\/(?P<token>[a-z]+)";
    }
    let re = regex::Regex::new(&rex).unwrap();
    let caps = match re.captures(orig_url) {
        Some(t) => t,
        None => panic!("Match error on URL {}", orig_url)
    };

    let title = caps.name("token").map_or("", |m| m.as_str());
    debug!("Token: {}", title);
    // So now we've gone from the original url to just 'thisisatokenname'
    let gifloc = format!("{}/{}", RG_GIFLOC_URL, &title.to_lowercase());
    debug!("Gifloc: {}", gifloc);
    debug!("RGToken: {}", rg_token);
    let response = match reqwest::Client::new()
    .get(&gifloc)
    .header("User-Agent", LOC_AGENT)
    .header("Authorization", rg_token)
    .send().await {
        Ok(e) => {
            debug!("URL Response: {:#?}", e);
            e.text().await?
        }
        Err(e) => return Err(e)
    };
    debug!("Response for {}: {}", &gifloc, &response.as_str());
    let resp_data: Value = match serde_json::from_str(&response) {
        Ok(t) => t,
        Err(t) => panic!("{} - No parseable json for {} at {}", t, orig_url, &response)
    };
    // Now we can finally grab the location of the HD-MP4 version of the video!
    let final_url = match resp_data["gif"]["urls"]["hd"].as_str() {
        Some(x) => x,
        // This keeps us from panicking if we get back an error — RG likes to return 200 OK
        //   and then hand you an Error JSON indicating the file is gone. Our calling
        //   function expects a reqwest::Response object and you can't create a reqwest::Error
        //   object by hand because...reasons? I don't know, seems silly. Hence this solution:
        //   make it create a request to something that should fail, which will properly return
        //   an error to the outer calling function.
        //
        // Yes, it's a kluge. Such is life.
        None => "http://127.0.0.1/invalid"
    };
    reqwest::Client::new()
    .get(final_url)
    .header("User-Agent", LOC_AGENT)
    .header("Authorization", rg_token)
    .send().await
}
