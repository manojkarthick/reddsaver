use rand::Rng;
use random_names::RandomName;
use std::path::Path;

/// Generate user agent string of the form <name>:<version>.
/// If no arguments passed generate random name and number
pub fn get_user_agent_string(name: Option<String>, version: Option<String>) -> String {
    if let (Some(v), Some(n)) = (version, name) {
        format!("{}:{}", n, v)
    } else {
        let random_name = RandomName::new()
            .to_string()
            .replace(" ", "")
            .to_lowercase();

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
            .map(|(i, c)| {
                if i == 0 || i == 1 || i == word_length - 1 {
                    c
                } else {
                    '*'
                }
            })
            .collect()
    };
}

/// Return delimited subreddit names or EMPTY if None
pub fn print_subreddits(subreddits: &Option<Vec<&str>>) -> String {
    return if let Some(s) = subreddits {
        s.join(",")
    } else {
        String::from("<ALL>")
    };
}
