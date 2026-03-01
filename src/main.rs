use std::env;
use std::fs;
use std::process::ExitCode;

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use env_logger::Env;
use log::{debug, error, info, warn};

use auth::Client;

use crate::download::Downloader;
use crate::errors::ReddSaverError;
use crate::user::{ListingType, SubredditSort, TimePeriod, User};
use crate::utils::*;

mod auth;
mod download;
mod errors;
mod structures;
mod user;
mod utils;

#[tokio::main]
async fn main() -> ExitCode {
    let matches = cli().get_matches();
    let env_file =
        String::from(matches.get_one::<String>("environment").map(|s| s.as_str()).unwrap());

    let env_file_result = load_env_file(&env_file);

    // initialize logger for the app and set logging level to info if no environment variable present
    let env = Env::default().filter("RS_LOG").default_filter_or("info");
    env_logger::Builder::from_env(env).init();

    if let Err(err) = env_file_result {
        error!("{err}");
        return ExitCode::FAILURE;
    }

    match run(matches).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn cli() -> Command {
    Command::new("ReddSaver")
        .version(crate_version!())
        .author("Manoj Karthick Selva Kumar")
        .about("Simple CLI tool to download saved media from Reddit")
        .arg(
            Arg::new("environment")
                .short('e')
                .long("from-env")
                .value_name("ENV_FILE")
                .help("Set a custom .env style file with secrets")
                .default_value(".env"),
        )
        .arg(
            Arg::new("data_directory")
                .short('d')
                .long("data-dir")
                .value_name("DATA_DIR")
                .help("Directory to save the media to")
                .default_value("data"),
        )
        .arg(
            Arg::new("show_config")
                .short('s')
                .long("show-config")
                .action(ArgAction::SetTrue)
                .help("Show the current config being used"),
        )
        .arg(
            Arg::new("dry_run")
                .short('r')
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Dry run and print the URLs of saved media to download"),
        )
        .arg(
            Arg::new("subreddits")
                .short('S')
                .long("subreddits")
                .action(ArgAction::Append)
                .value_name("SUBREDDITS")
                .value_delimiter(',')
                .help("Download media from these subreddits only"),
        )
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .value_parser(["saved", "upvoted", "feed"])
                .default_value("saved")
                .help("Operation mode"),
        )
        .arg(
            Arg::new("listing_type")
                .short('t')
                .long("listing-type")
                .value_name("TYPE")
                .value_parser(["hot", "top", "new", "controversial"])
                .default_value("hot")
                .help("Subreddit listing sort"),
        )
        .arg(
            Arg::new("time_filter")
                .short('T')
                .long("time-filter")
                .value_name("PERIOD")
                .value_parser(["hour", "day", "week", "month", "year", "all"])
                .default_value("all")
                .help("Time period for top/controversial listings"),
        )
        .arg(
            Arg::new("limit")
                .short('l')
                .long("limit")
                .value_name("LIMIT")
                .value_parser(clap::value_parser!(usize))
                .help("Max posts to process per source (default: unlimited for saved/upvoted, 1000 for feed)"),
        )
}

async fn run(matches: ArgMatches) -> Result<(), ReddSaverError> {
    let env_file = matches.get_one::<String>("environment").map(|s| s.as_str()).unwrap();

    let data_directory =
        String::from(matches.get_one::<String>("data_directory").map(|s| s.as_str()).unwrap());
    // generate the URLs to download from without actually downloading the media
    let should_download = !matches.get_flag("dry_run");
    // check if ffmpeg is present for combining video streams
    let ffmpeg_available = application_present(String::from("ffmpeg"));
    // check if yt-dlp is present on the system
    let ytdlp_available = application_present(String::from("yt-dlp"));
    // restrict downloads to these subreddits (filter for saved/upvoted; source for feed mode)
    let subreddits: Option<Vec<&str>> =
        matches.get_many::<String>("subreddits").map(|vals| vals.map(|s| s.as_str()).collect());

    let effective_mode = matches.get_one::<String>("mode").map(|s| s.as_str()).unwrap_or("saved");

    // Parse listing-type and time-filter (only meaningful in feed mode)
    let listing_type_str =
        matches.get_one::<String>("listing_type").map(|s| s.as_str()).unwrap_or("hot");
    let time_filter_str =
        matches.get_one::<String>("time_filter").map(|s| s.as_str()).unwrap_or("all");

    // Validate that listing-type / time-filter are not used outside feed mode.
    // We detect "explicit" use by checking whether the value differs from the default.
    if effective_mode != "feed" {
        let listing_type_explicit = listing_type_str != "hot"
            && matches.value_source("listing_type") == Some(clap::parser::ValueSource::CommandLine);
        let time_filter_explicit = time_filter_str != "all"
            && matches.value_source("time_filter") == Some(clap::parser::ValueSource::CommandLine);

        if listing_type_explicit {
            return Err(ReddSaverError::InvalidArgument(
                "--listing-type is only valid with --mode feed".to_string(),
            ));
        }
        if time_filter_explicit {
            return Err(ReddSaverError::InvalidArgument(
                "--time-filter is only valid with --mode feed".to_string(),
            ));
        }
    }

    // In feed mode, --subreddits is required
    if effective_mode == "feed" && subreddits.is_none() {
        return Err(ReddSaverError::InvalidArgument(
            "--mode feed requires at least one subreddit via --subreddits".to_string(),
        ));
    }

    // Determine effective limit
    let explicit_limit: Option<usize> = matches.get_one::<usize>("limit").copied();
    let effective_limit: Option<usize> = match effective_mode {
        "feed" => Some(explicit_limit.unwrap_or(1000)),
        _ => explicit_limit, // None means unlimited for saved/upvoted
    };

    let sort = match listing_type_str {
        "top" => SubredditSort::Top,
        "new" => SubredditSort::New,
        "controversial" => SubredditSort::Controversial,
        _ => SubredditSort::Hot,
    };

    let period: Option<TimePeriod> = match listing_type_str {
        "top" | "controversial" => Some(match time_filter_str {
            "hour" => TimePeriod::Hour,
            "day" => TimePeriod::Day,
            "week" => TimePeriod::Week,
            "month" => TimePeriod::Month,
            "year" => TimePeriod::Year,
            _ => TimePeriod::All,
        }),
        _ => None,
    };

    let required_env = load_required_env()?;
    let client_id = required_env.client_id;
    let client_secret = required_env.client_secret;
    let username = required_env.username;
    let password = required_env.password;
    let user_agent = get_user_agent_string(None, None);

    ensure_data_directory(&data_directory)?;

    // if the option is show-config, show the configuration and return immediately
    if matches.get_flag("show_config") {
        info!("Current configuration:");
        info!("ENVIRONMENT_FILE = {}", &env_file);
        info!("DATA_DIRECTORY = {}", &data_directory);
        info!("REDDSAVER_CLIENT_ID = {}", &client_id);
        info!("REDDSAVER_CLIENT_SECRET = {}", mask_sensitive(&client_secret));
        info!("REDDSAVER_USERNAME = {}", &username);
        info!("REDDSAVER_PASSWORD = {}", mask_sensitive(&password));
        info!("USER_AGENT = {}", &user_agent);
        info!("SUBREDDITS = {}", print_subreddits(&subreddits));
        info!("MODE = {}", effective_mode);
        info!("LISTING_TYPE = {}", listing_type_str);
        info!("TIME_FILTER = {}", time_filter_str);
        info!(
            "LIMIT = {}",
            effective_limit.map(|n| n.to_string()).unwrap_or_else(|| "unlimited".to_string())
        );
        info!("FFMPEG AVAILABLE = {}", ffmpeg_available);
        info!("YT-DLP AVAILABLE = {}", ytdlp_available);

        return Ok(());
    }

    if !ffmpeg_available {
        warn!(
            "No ffmpeg Installation available. \
            Videos hosted by Reddit use separate video and audio streams. \
            Ffmpeg needs be installed to combine the audio and video into a single mp4."
        );
    }

    if !ytdlp_available {
        warn!(
            "yt-dlp is not installed. Youtube videos will not be downloaded. \
            Install yt-dlp (https://github.com/yt-dlp/yt-dlp)."
        );
    }

    // login to reddit using the credentials provided and get API bearer token
    let auth =
        Client::new(&client_id, &client_secret, &username, &password, &user_agent).login().await?;
    info!("Successfully logged in to Reddit as {}", username);
    debug!("Authentication details: {:#?}", auth);

    // get information about the user to display
    let user = User::new(&auth, &username);

    let user_info = user.about().await?;
    info!("The user details are: ");
    info!("Account name: {:#?}", user_info.data.name);
    info!("Account ID: {:#?}", user_info.data.id);
    info!("Comment Karma: {:#?}", user_info.data.comment_karma);
    info!("Link Karma: {:#?}", user_info.data.link_karma);

    info!("Starting data gathering from Reddit. This might take some time. Hold on....");

    let (listing, downloader_subreddits) = match effective_mode {
        "feed" => {
            let subreddits_list = subreddits.as_ref().unwrap();
            let limit = effective_limit.unwrap(); // always Some in subreddit mode
            let mut all_listings = Vec::new();
            for sub in subreddits_list {
                info!("Fetching r/{} ({}, limit {})", sub, sort, limit);
                let mut sub_listing =
                    user.subreddit_listing(sub, &sort, period.as_ref(), limit).await?;
                all_listings.append(&mut sub_listing);
            }
            // subreddits filter not needed — we already fetched per-subreddit
            (all_listings, None)
        }
        _ => {
            let listing_type = if effective_mode == "upvoted" {
                ListingType::Upvoted
            } else {
                ListingType::Saved
            };
            let listing = user.listing(&listing_type, effective_limit).await?;
            (listing, subreddits)
        }
    };

    debug!("Posts: {:#?}", listing);

    let downloader = Downloader::new(
        &listing,
        &data_directory,
        &downloader_subreddits,
        should_download,
        ffmpeg_available,
        ytdlp_available,
    );

    downloader.run().await?;

    Ok(())
}

fn load_env_file(env_file: &str) -> Result<(), ReddSaverError> {
    match dotenvy::from_filename(env_file) {
        Ok(_) => Ok(()),
        Err(err) if err.not_found() => Ok(()),
        Err(err) => Err(ReddSaverError::EnvironmentFileLoadError {
            path: String::from(env_file),
            source: err,
        }),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RequiredEnvConfig {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
}

fn load_required_env() -> Result<RequiredEnvConfig, ReddSaverError> {
    load_required_env_with(|name| env::var(name))
}

fn load_required_env_with<F>(mut read_var: F) -> Result<RequiredEnvConfig, ReddSaverError>
where
    F: FnMut(&str) -> Result<String, env::VarError>,
{
    let mut missing = Vec::new();
    let client_id = read_required_env(&mut read_var, &mut missing, "REDDSAVER_CLIENT_ID")?;
    let client_secret = read_required_env(&mut read_var, &mut missing, "REDDSAVER_CLIENT_SECRET")?;
    let username = read_required_env(&mut read_var, &mut missing, "REDDSAVER_USERNAME")?;
    let password = read_required_env(&mut read_var, &mut missing, "REDDSAVER_PASSWORD")?;

    if !missing.is_empty() {
        return Err(ReddSaverError::MissingEnvironmentVariables(missing));
    }

    Ok(RequiredEnvConfig {
        client_id: client_id.unwrap(),
        client_secret: client_secret.unwrap(),
        username: username.unwrap(),
        password: password.unwrap(),
    })
}

fn read_required_env<F>(
    read_var: &mut F,
    missing: &mut Vec<String>,
    name: &str,
) -> Result<Option<String>, ReddSaverError>
where
    F: FnMut(&str) -> Result<String, env::VarError>,
{
    match read_var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => {
            missing.push(String::from(name));
            Ok(None)
        }
        Err(env::VarError::NotUnicode(_)) => {
            Err(ReddSaverError::InvalidEnvironmentVariableEncoding(String::from(name)))
        }
    }
}

fn ensure_data_directory(path: &str) -> Result<(), ReddSaverError> {
    if !check_path_present(path) {
        info!("Data directory not found, creating {}", path);
        fs::create_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::Write;

    use super::*;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn creates_missing_data_directory() {
        let parent = tempdir().unwrap();
        let new_dir = parent.path().join("data");
        assert!(!new_dir.exists());

        ensure_data_directory(new_dir.to_str().unwrap()).unwrap();

        assert!(new_dir.exists());
    }

    #[test]
    fn does_not_fail_when_data_directory_already_exists() {
        let dir = tempdir().unwrap();
        assert!(dir.path().exists());

        ensure_data_directory(dir.path().to_str().unwrap()).unwrap();

        assert!(dir.path().exists());
    }

    #[test]
    fn creates_nested_data_directory() {
        let parent = tempdir().unwrap();
        let nested = parent.path().join("a").join("b").join("c");
        assert!(!nested.exists());

        ensure_data_directory(nested.to_str().unwrap()).unwrap();

        assert!(nested.exists());
    }

    #[test]
    fn loads_required_env_when_all_values_are_present() {
        let values = HashMap::from([
            ("REDDSAVER_CLIENT_ID", Ok(String::from("client-id"))),
            ("REDDSAVER_CLIENT_SECRET", Ok(String::from("client-secret"))),
            ("REDDSAVER_USERNAME", Ok(String::from("username"))),
            ("REDDSAVER_PASSWORD", Ok(String::from("password"))),
        ]);

        let config = load_required_env_with(|name| values.get(name).cloned().unwrap()).unwrap();

        assert_eq!(
            config,
            RequiredEnvConfig {
                client_id: String::from("client-id"),
                client_secret: String::from("client-secret"),
                username: String::from("username"),
                password: String::from("password"),
            }
        );
    }

    #[test]
    fn reports_a_single_missing_env_var_by_name() {
        let values = HashMap::from([
            ("REDDSAVER_CLIENT_ID", Ok(String::from("client-id"))),
            ("REDDSAVER_CLIENT_SECRET", Ok(String::from("client-secret"))),
            ("REDDSAVER_USERNAME", Err(env::VarError::NotPresent)),
            ("REDDSAVER_PASSWORD", Ok(String::from("password"))),
        ]);

        let err = load_required_env_with(|name| values.get(name).cloned().unwrap()).unwrap_err();

        assert!(matches!(
            err,
            ReddSaverError::MissingEnvironmentVariables(names)
                if names == vec![String::from("REDDSAVER_USERNAME")]
        ));
    }

    #[test]
    fn reports_all_missing_env_vars_in_a_stable_order() {
        let values = HashMap::from([
            ("REDDSAVER_CLIENT_ID", Err(env::VarError::NotPresent)),
            ("REDDSAVER_CLIENT_SECRET", Ok(String::from("client-secret"))),
            ("REDDSAVER_USERNAME", Err(env::VarError::NotPresent)),
            ("REDDSAVER_PASSWORD", Err(env::VarError::NotPresent)),
        ]);

        let err = load_required_env_with(|name| values.get(name).cloned().unwrap()).unwrap_err();

        assert!(matches!(
            err,
            ReddSaverError::MissingEnvironmentVariables(names)
                if names
                    == vec![
                        String::from("REDDSAVER_CLIENT_ID"),
                        String::from("REDDSAVER_USERNAME"),
                        String::from("REDDSAVER_PASSWORD"),
                    ]
        ));
    }

    #[test]
    fn reports_invalid_unicode_with_the_env_var_name() {
        let values = HashMap::from([
            ("REDDSAVER_CLIENT_ID", Ok(String::from("client-id"))),
            ("REDDSAVER_CLIENT_SECRET", Err(env::VarError::NotUnicode(OsString::from("bad")))),
            ("REDDSAVER_USERNAME", Ok(String::from("username"))),
            ("REDDSAVER_PASSWORD", Ok(String::from("password"))),
        ]);

        let err = load_required_env_with(|name| values.get(name).cloned().unwrap()).unwrap_err();

        assert!(matches!(
            err,
            ReddSaverError::InvalidEnvironmentVariableEncoding(name)
                if name == "REDDSAVER_CLIENT_SECRET"
        ));
    }

    #[test]
    fn ignores_missing_env_files() {
        let result = load_env_file("/path/that/does/not/exist/reddsaver.env");

        assert!(result.is_ok());
    }

    #[test]
    fn reports_invalid_env_file_syntax() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "REDDSAVER_CLIENT_ID=\"abc$def\nREDDSAVER_CLIENT_SECRET=\"secret\"")
            .unwrap();

        let err = load_env_file(file.path().to_str().unwrap()).unwrap_err();

        assert!(matches!(
            err,
            ReddSaverError::EnvironmentFileLoadError { ref path, .. }
                if path == file.path().to_str().unwrap()
        ));
        assert!(err.to_string().contains("Could not load environment file"));
    }
}
