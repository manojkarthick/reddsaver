use std::env;
use std::fs;
use std::process::ExitCode;

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use env_logger::Env;
use log::{debug, error, info, warn};

use auth::Client;

use crate::download::Downloader;
use crate::errors::ReddSaverError;
use crate::user::{ListingType, Mode, SubredditSort, TimePeriod, User};
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
        String::from(matches.get_one::<String>("environment").map_or(".env", |s| s.as_str()));

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
                .value_parser(clap::value_parser!(Mode))
                .default_value("saved")
                .help("Operation mode"),
        )
        .arg(
            Arg::new("listing_type")
                .short('t')
                .long("listing-type")
                .value_name("TYPE")
                .value_parser(clap::value_parser!(SubredditSort))
                .default_value("hot")
                .help("Subreddit listing sort"),
        )
        .arg(
            Arg::new("time_filter")
                .short('T')
                .long("time-filter")
                .value_name("PERIOD")
                .value_parser(clap::value_parser!(TimePeriod))
                .default_value("all")
                .help("Time period for top/controversial listings"),
        )
        .arg(
            Arg::new("limit")
                .short('l')
                .long("limit")
                .value_name("LIMIT")
                .value_parser(clap::value_parser!(usize))
                .help("Max posts to process per source (default: unlimited for saved/upvoted, 500 for feed/user)"),
        )
        .arg(
            Arg::new("target_user")
                .short('u')
                .long("user")
                .value_name("USERNAME")
                .help("Reddit username to download submissions from (required for user mode)"),
        )
}

async fn run(matches: ArgMatches) -> Result<(), ReddSaverError> {
    let env_file = matches.get_one::<String>("environment").map_or(".env", |s| s.as_str());

    let data_directory =
        String::from(matches.get_one::<String>("data_directory").map_or("data", |s| s.as_str()));
    // generate the URLs to download from without actually downloading the media
    let should_download = !matches.get_flag("dry_run");
    // check if ffmpeg is present for combining video streams
    let ffmpeg_available = application_present(String::from("ffmpeg"));
    // check if yt-dlp is present on the system
    let ytdlp_available = application_present(String::from("yt-dlp"));
    // restrict downloads to these subreddits (filter for saved/upvoted; source for feed mode)
    let subreddits: Option<Vec<&str>> =
        matches.get_many::<String>("subreddits").map(|vals| vals.map(|s| s.as_str()).collect());

    let mode = matches.get_one::<Mode>("mode").cloned().unwrap_or(Mode::Saved);
    let target_user: Option<String> = matches.get_one::<String>("target_user").cloned();

    // Parse listing-type and time-filter (only meaningful in feed mode)
    let listing_type = matches.get_one::<SubredditSort>("listing_type").cloned().unwrap_or(SubredditSort::Hot);
    let time_filter = matches.get_one::<TimePeriod>("time_filter").cloned().unwrap_or(TimePeriod::All);

    // Validate that listing-type / time-filter are not used outside feed mode.
    // For time-filter, we only care whether the user passed the flag explicitly.
    if !matches!(mode, Mode::Feed | Mode::User) {
        let listing_type_explicit = listing_type != SubredditSort::Hot
            && matches.value_source("listing_type") == Some(clap::parser::ValueSource::CommandLine);
        let time_filter_explicit =
            matches.value_source("time_filter") == Some(clap::parser::ValueSource::CommandLine);

        if listing_type_explicit {
            return Err(ReddSaverError::InvalidArgument(
                "--listing-type is only valid with --mode feed or --mode user".to_string(),
            ));
        }
        if time_filter_explicit {
            return Err(ReddSaverError::InvalidArgument(
                "--time-filter is only valid with --mode feed or --mode user".to_string(),
            ));
        }
    }

    // In user mode, --user is required
    if mode == Mode::User && target_user.is_none() {
        return Err(ReddSaverError::InvalidArgument(
            "--mode user requires a username via --user".to_string(),
        ));
    }

    // --user is only valid with user mode
    if mode != Mode::User && target_user.is_some() {
        return Err(ReddSaverError::InvalidArgument(
            "--user is only valid with --mode user".to_string(),
        ));
    }

    // In feed mode, --subreddits is required
    if mode == Mode::Feed && subreddits.is_none() {
        return Err(ReddSaverError::InvalidArgument(
            "--mode feed requires at least one subreddit via --subreddits".to_string(),
        ));
    }

    let time_filter_explicit =
        matches.value_source("time_filter") == Some(clap::parser::ValueSource::CommandLine);
    if matches!(mode, Mode::Feed | Mode::User)
        && time_filter_explicit
        && !matches!(listing_type, SubredditSort::Top | SubredditSort::Controversial)
    {
        warn!("--time-filter is only supported for top and controversial listing types, ignoring.");
    }

    // Determine effective limit
    let explicit_limit: Option<usize> = matches.get_one::<usize>("limit").copied();
    let effective_limit: Option<usize> = match mode {
        Mode::Feed | Mode::User => Some(explicit_limit.unwrap_or(500)),
        _ => explicit_limit, // None means unlimited for saved/upvoted
    };

    let period = match listing_type {
        SubredditSort::Top | SubredditSort::Controversial => Some(&time_filter),
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
        info!("MODE = {}", mode);
        if let Some(ref tu) = target_user {
            info!("TARGET_USER = {}", tu);
        }
        info!("LISTING_TYPE = {}", listing_type);
        info!("TIME_FILTER = {}", time_filter);
        info!(
            "LIMIT = {}",
            effective_limit.map(|n| n.to_string()).unwrap_or_else(|| "unlimited".to_string())
        );
        info!("YT-DLP AVAILABLE = {}", ytdlp_available);

        return Ok(());
    }

    if !ffmpeg_available {
        return Err(ReddSaverError::FfmpegNotFound);
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

    let listing = match &mode {
        Mode::Feed => {
            let subreddits_list = subreddits.as_ref().expect("feed mode requires --subreddits");
            let limit = effective_limit.expect("feed/user mode always has a limit");
            let mut all_listings = Vec::new();
            for sub in subreddits_list {
                info!("Fetching r/{} ({}, limit {})", sub, listing_type, limit);
                let mut sub_listing =
                    user.subreddit_listing(sub, &listing_type, period, limit).await?;
                all_listings.append(&mut sub_listing);
            }
            all_listings
        }
        Mode::User => {
            let target = target_user.as_ref().expect("user mode requires --user");
            let limit = effective_limit.expect("feed/user mode always has a limit");
            info!("Fetching submissions from u/{} ({}, limit {})", target, listing_type, limit);
            user.user_listing(target, &listing_type, period, limit).await?
        }
        Mode::Saved => user.listing(&ListingType::Saved, effective_limit).await?,
        Mode::Upvoted => user.listing(&ListingType::Upvoted, effective_limit).await?,
    };

    debug!("Posts: {:#?}", listing);

    // In feed mode subreddits were used as sources; no further filtering is needed
    let subreddit_filter = match mode {
        Mode::Feed => None,
        _ => subreddits,
    };

    let downloader = Downloader::new(
        &listing,
        &data_directory,
        &subreddit_filter,
        should_download,
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
        client_id: client_id.expect("checked above"),
        client_secret: client_secret.expect("checked above"),
        username: username.expect("checked above"),
        password: password.expect("checked above"),
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
