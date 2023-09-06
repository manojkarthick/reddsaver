use std::borrow::Borrow;
use std::fs::File;
use std::ops::Add;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use reqwest::StatusCode;
use tempfile::tempdir;
use url::{Position, Url};
use async_once::AsyncOnce;

use crate::errors::ReddSaverError;
use crate::structures::{GfyData, PostData};
use crate::structures::{Listing, Summary};
use crate::user::{ListingType, User};
use crate::utils::{check_path_present, check_url_is_mp4, fetch_redgif_url, fetch_redgif_token};

static JPG_EXTENSION: &str = "jpg";
static PNG_EXTENSION: &str = "png";
static GIF_EXTENSION: &str = "gif";
static GIFV_EXTENSION: &str = "gifv";
static MP4_EXTENSION: &str = "mp4";

static REDDIT_DOMAIN: &str = "reddit.com";
static REDDIT_IMAGE_SUBDOMAIN: &str = "i.redd.it";
static REDDIT_VIDEO_SUBDOMAIN: &str = "v.redd.it";
static REDDIT_GALLERY_PATH: &str = "gallery";

static IMGUR_DOMAIN: &str = "imgur.com";
static IMGUR_SUBDOMAIN: &str = "i.imgur.com";

static GFYCAT_DOMAIN: &str = "gfycat.com";
static GFYCAT_API_PREFIX: &str = "https://api.gfycat.com/v1/gfycats";

static REDGIFS_DOMAIN: &str = "redgifs.com";
static REDGIFS_API_PREFIX: &str = "https://api.redgifs.com/v1/gfycats";

static GIPHY_DOMAIN: &str = "giphy.com";
static GIPHY_MEDIA_SUBDOMAIN: &str = "media.giphy.com";
static GIPHY_MEDIA_SUBDOMAIN_0: &str = "media0.giphy.com";
static GIPHY_MEDIA_SUBDOMAIN_1: &str = "media1.giphy.com";
static GIPHY_MEDIA_SUBDOMAIN_2: &str = "media2.giphy.com";
static GIPHY_MEDIA_SUBDOMAIN_3: &str = "media3.giphy.com";
static GIPHY_MEDIA_SUBDOMAIN_4: &str = "media4.giphy.com";

lazy_static!{
    static ref RG_TOKEN : AsyncOnce<String> = AsyncOnce::new(async {
        let rgtoken = format!("Bearer {}", fetch_redgif_token().await.unwrap());
        rgtoken
    });
}

/// Status of media processing
enum MediaStatus {
    /// If we are able to successfully download the media
    Downloaded,
    /// If we are skipping downloading the media due to it already being present
    /// or because we could not find the media or because we are unable to decode
    /// the media
    Skipped,
}

/// Media Types Supported
#[derive(Debug, PartialEq)]
enum MediaType {
    RedditImage,
    RedditGif,
    RedditVideoWithAudio,
    RedditVideoWithoutAudio,
    GfycatGif,
    RedgifsVideo,
    GiphyGif,
    ImgurImage,
    ImgurGif,
}

/// Information about supported media for downloading
struct SupportedMedia {
    /// The components for the media. This is a vector of size one for
    /// all media types except Reddit videos and Reddit Galleries.
    /// For reddit videos, audio and video are provided separately.
    components: Vec<String>,
    media_type: MediaType,
}

#[derive(Debug)]
pub struct Downloader<'a> {
    user: &'a User<'a>,
    listing: &'a Vec<Listing>,
    listing_type: &'a ListingType,
    data_directory: &'a str,
    subreddits: &'a Option<Vec<&'a str>>,
    should_download: bool,
    use_human_readable: bool,
    undo: bool,
    ffmpeg_available: bool,
}

impl<'a> Downloader<'a> {
    pub fn new(
        user: &'a User,
        listing: &'a Vec<Listing>,
        listing_type: &'a ListingType,
        data_directory: &'a str,
        subreddits: &'a Option<Vec<&'a str>>,
        should_download: bool,
        use_human_readable: bool,
        undo: bool,
        ffmpeg_available: bool,
    ) -> Downloader<'a> {
        Downloader {
            user,
            listing,
            listing_type,
            data_directory,
            subreddits,
            should_download,
            use_human_readable,
            undo,
            ffmpeg_available,
        }
    }

    pub async fn run(self) -> Result<(), ReddSaverError> {
        let mut full_summary =
            Summary { media_downloaded: 0, media_skipped: 0, media_supported: 0 };

        for collection in self.listing {
            full_summary =
                full_summary.add(self.download_collection(collection, self.listing_type).await?);
        }

        info!("#####################################");
        info!("Download Summary:");
        info!("Number of supported media: {}", full_summary.media_supported);
        info!("Number of media downloaded: {}", full_summary.media_downloaded);
        info!("Number of media skipped: {}", full_summary.media_skipped);
        info!("#####################################");
        info!("FIN.");

        Ok(())
    }

    /// Download and save medias from Reddit in parallel
    async fn download_collection(
        &self,
        collection: &Listing,
        listing_type: &ListingType,
    ) -> Result<Summary, ReddSaverError> {
        let summary = Arc::new(Mutex::new(Summary {
            media_supported: 0,
            media_downloaded: 0,
            media_skipped: 0,
        }));

        collection
            .data
            .children
            .clone()
            .into_iter()
            // filter out the posts where a URL is present
            // not that this application cannot download URLs linked within the text of the post
            .filter(|item| item.data.url.is_some())
            .map(|item| {
                let summary_arc = summary.clone();
                // since the latency for downloading an media from the network is unpredictable
                // we spawn a new async task for the each of the medias to be downloaded
                async move {
                    let subreddit = item.data.subreddit.borrow();
                    let post_name = item.data.name.borrow();
                    let post_title = match item.data.title.as_ref() {
                        Some(t) => t,
                        None => "",
                    };

                    let is_valid = if let Some(s) = self.subreddits.as_ref() {
                        if s.contains(&subreddit) {
                            true
                        } else {
                            false
                        }
                    } else {
                        true
                    };

                    if is_valid {
                        debug!("Subreddit VALID: {} present in {:#?}", subreddit, subreddit);

                        let supported_media_items = get_media(item.data.borrow()).await?;

                        for supported_media in supported_media_items {
                            let media_urls = &supported_media.components;
                            let media_type = supported_media.media_type;
                            let mut media_files = Vec::new();

                            // the number of components in the supported media is the number available for download
                            summary_arc.lock().unwrap().media_supported += supported_media.components.len() as i32;

                            let mut local_skipped = 0;
                            for (index, url) in media_urls.iter().enumerate() {
                                let mut item_index = format!("{}", index);
                                let mut extension =
                                    String::from(url.split('.').last().unwrap_or("unknown")).replace("/", "_");

                                // if the media is a reddit video, they have separate audio and video components.
                                // to differentiate this from albums, which use the regular _0, _1, etc indices,
                                // we use _component_0, component_1 indices to explicitly inform that these are
                                // components rather than individual media.
                                if media_type == MediaType::RedditVideoWithAudio {
                                    item_index = format!("component_{}", index);
                                };
                                // some reddit videos don't have the mp4 extension, eg. DASH_<A>_<B>
                                // explicitly adding an mp4 extension to make it easy to recognize in the finder
                                if (media_type == MediaType::RedditVideoWithoutAudio
                                    || media_type == MediaType::RedditVideoWithAudio)
                                    && !extension.ends_with(".mp4") {
                                    extension = format!("{}.{}", extension, ".mp4");
                                }
                                if media_type == MediaType::RedgifsVideo {
                                    if url.contains(MP4_EXTENSION){
                                        extension = ".mp4".to_string();
                                    } else {
                                        extension = ".gif".to_string();
                                    }
                                }
                                let file_name = self.generate_file_name(
                                    &url,
                                    &subreddit,
                                    &extension,
                                    &post_name,
                                    &post_title,
                                    &item_index,
                                );

                                if self.should_download {
                                    let status = save_or_skip(url, &file_name);
                                    // update the summary statistics based on the status
                                    match status.await? {
                                        MediaStatus::Downloaded => {
                                            summary_arc.lock().unwrap().media_downloaded += 1;
                                        }
                                        MediaStatus::Skipped => {
                                            local_skipped += 1;
                                            summary_arc.lock().unwrap().media_skipped += 1;
                                        }
                                    }
                                } else {
                                    info!("Media available at URL: {}", &url);
                                    summary_arc.lock().unwrap().media_skipped += 1;
                                }

                                // push all the available media files into a vector
                                // this is needed in the next step to combine the components using ffmpeg
                                media_files.push(file_name);
                            }

                            debug!("Media type: {:#?}", media_type);
                            debug!("Media files: {:?}", media_files.len());
                            debug!("Locally skipped items: {:?}", local_skipped);

                            if (media_type == MediaType::RedditVideoWithAudio)
                                && (media_files.len() == 2)
                                && (local_skipped < 2) {
                                if self.ffmpeg_available {
                                    debug!("Assembling components together");
                                    let first_url = media_urls.first().unwrap();
                                    let extension =
                                        String::from(first_url.split('.').last().unwrap_or("unknown"));
                                    // this generates the name of the media without the component indices
                                    // this file name is used for saving the ffmpeg combined file
                                    let combined_file_name = self.generate_file_name(
                                        first_url,
                                        &subreddit,
                                        &extension,
                                        &post_name,
                                        &post_title,
                                        "0",
                                    );

                                    let temporary_dir = tempdir()?;
                                    let temporary_file_name = temporary_dir.path().join("combined.mp4");

                                    if self.should_download {
                                        // if the media is a reddit video and it has two components, then we
                                        // need to assemble them into one file using ffmpeg.
                                        let mut command = Command::new("ffmpeg");
                                        for media_file in &media_files {
                                            command.arg("-i").arg(media_file);
                                        }
                                        command.arg("-c").arg("copy")
                                            .arg("-map").arg("1:a")
                                            .arg("-map").arg("0:v")
                                            .arg(&temporary_file_name);

                                        debug!("Executing command: {:#?}", command);
                                        let output = command.output()?;

                                        // check the status code of the ffmpeg command. if the command is unsuccessful,
                                        // display the error and skip combining the media.
                                        if output.status.success() {
                                            debug!("Successfully combined into temporary file: {:?}", temporary_file_name);
                                            debug!("Renaming file: {} -> {}", temporary_file_name.display(), combined_file_name);
                                            fs::rename(&temporary_file_name, &combined_file_name)?;
                                        } else {
                                            // if we encountered an error, we will write logs from ffmpeg into a new log file
                                            let log_file_name = self.generate_file_name(
                                                first_url,
                                                &subreddit,
                                                "log",
                                                &post_name,
                                                &post_title,
                                                "0",
                                            );
                                            let err = String::from_utf8(output.stderr).unwrap();
                                            warn!("Could not combine video {} and audio {}. Saving log to: {}", 
                                                media_urls.first().unwrap(), media_urls.last().unwrap(), log_file_name);
                                            fs::write(log_file_name, err)?;
                                        }
                                    }
                                } else {
                                    warn!("Skipping combining the individual components since ffmpeg is not installed");
                                }
                            } else {
                                debug!("Skipping combining reddit video.");
                            }
                        }
                    } else {
                        debug!(
                            "Subreddit INVALID!: {} NOT present in {:#?}",
                            subreddit, self.subreddits
                        );
                    }

                    if self.undo {
                        self.user.undo(post_name, listing_type).await?;
                    }

                    Ok::<(), ReddSaverError>(())
                }
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<()>()
            .await?;

        let local_summary = *summary.lock().unwrap();

        debug!("Collection statistics: ");
        debug!("Number of supported media: {}", local_summary.media_supported);
        debug!("Number of media downloaded: {}", local_summary.media_downloaded);
        debug!("Number of media skipped: {}", local_summary.media_skipped);

        Ok(local_summary)
    }

    /// Generate a file name in the right format that Reddsaver expects
    fn generate_file_name(
        &self,
        url: &str,
        subreddit: &str,
        extension: &str,
        name: &str,
        title: &str,
        index: &str,
    ) -> String {
        return if !self.use_human_readable {
            // create a hash for the media using the URL the media is located at
            // this helps to make sure the media download always writes the same file
            // name irrespective of how many times it's run. If run more than once, the
            // media is overwritten by this method
            let hash = md5::compute(url);
            format!(
                // TODO: Fixme, use appropriate prefix
                "{}/{}/img-{:x}.{}",
                self.data_directory, subreddit, hash, extension
            )
        } else {
            let canonical_title: String = title
                .to_lowercase()
                .chars()
                // to make sure file names don't exceed operating system maximums, truncate at 200
                // you could possibly stretch beyond 200, but this is a conservative estimate that
                // leaves 55 bytes for the name string
                .take(200)
                .enumerate()
                .map(|(_, c)| {
                    if c.is_whitespace()
                        || c == '.'
                        || c == '/'
                        || c == '\\'
                        || c == ':'
                        || c == '='
                    {
                        '_'
                    } else {
                        c
                    }
                })
                .collect();
            // create a canonical human readable file name using the post's title
            // note that the name of the post is something of the form t3_<randomstring>
            let canonical_name: String =
                if index == "0" { String::from(name) } else { format!("{}_{}", name, index) }
                    .replace(".", "_");
            format!(
                "{}/{}/{}_{}.{}",
                self.data_directory, subreddit, canonical_title, canonical_name, extension
            )
        };
    }
}

/// Helper function that downloads and saves a single media from Reddit or Imgur
async fn save_or_skip(url: &str, file_name: &str) -> Result<MediaStatus, ReddSaverError> {
    if check_path_present(&file_name) {
        debug!("Media from url {} already downloaded. Skipping...", url);
        Ok(MediaStatus::Skipped)
    } else {
        let save_status = download_media(&file_name, &url).await?;
        if save_status {
            Ok(MediaStatus::Downloaded)
        } else {
            Ok(MediaStatus::Skipped)
        }
    }
}

/// Download media from the given url and save to data directory. Also create data directory if not present already
async fn download_media(file_name: &str, url: &str) -> Result<bool, ReddSaverError> {
    // create directory if it does not already exist
    // the directory is created relative to the current working directory
    let mut status = false;
    let directory = Path::new(file_name).parent().unwrap();
    match fs::create_dir_all(directory) {
        Ok(_) => (),
        Err(_e) => return Err(ReddSaverError::CouldNotCreateDirectory),
    }
    let maybe_response: reqwest::Result<reqwest::Response>;
    if url.contains(REDGIFS_DOMAIN) {
        maybe_response = fetch_redgif_url(RG_TOKEN.get().await, url).await;
    } else {
        maybe_response = reqwest::get(url).await;
    };
    if let Ok(response) = maybe_response {
        debug!("URL Response: {:#?}", response);
        let maybe_data = response.bytes().await;
        if let Ok(data) = maybe_data {
            debug!("Bytes length of the data: {:#?}", data.len());
            let maybe_output = File::create(&file_name);
            match maybe_output {
                Ok(mut output) => {
                    debug!("Created a file: {}", file_name);
                    match io::copy(&mut data.as_ref(), &mut output) {
                        Ok(_) => {
                            info!("Successfully saved media: {} from url {}", file_name, url);
                            status = true;
                        }
                        Err(_e) => {
                            error!("Could not save media from url {} to {}", url, file_name);
                        }
                    }
                }
                Err(_) => {
                    warn!("Could not create a file with the name: {}. Skipping", file_name);
                }
            }
        }
    }

    Ok(status)
}

/// Convert Gfycat/Redgifs GIFs into mp4 URLs for download
async fn gfy_to_mp4(url: &str) -> Result<Option<SupportedMedia>, ReddSaverError> {
    let api_prefix =
        if url.contains(GFYCAT_DOMAIN) { GFYCAT_API_PREFIX } else { REDGIFS_API_PREFIX };
    let maybe_media_id = url.split("/").last();

    if let Some(media_id) = maybe_media_id {
        let api_url = format!("{}/{}", api_prefix, media_id);
        debug!("GFY API URL: {}", api_url);
        let client = reqwest::Client::new();

        // talk to gfycat API and get GIF information
        let response = client.get(&api_url).send().await?;
        // if the gif is not available anymore, Gfycat might send
        // a 404 response. Proceed to get the mp4 URL only if the
        // response was HTTP 200
        if response.status() == StatusCode::OK {
            let data = response.json::<GfyData>().await?;
            let supported_media = SupportedMedia {
                components: vec![data.gfy_item.mp4_url],
                media_type: MediaType::GfycatGif,
            };
            Ok(Some(supported_media))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

// Get reddit video information and optionally the audio track if it exists
async fn get_reddit_video(url: &str) -> Result<Option<SupportedMedia>, ReddSaverError> {
    let maybe_dash_video = url.split("/").last();
    if let Some(dash_video) = maybe_dash_video {
        let present = dash_video.contains("DASH");
        // todo: find exhaustive collection of these, or figure out if they are (x, x*2) pairs
        let dash_video_only = vec!["DASH_1_2_M", "DASH_2_4_M", "DASH_4_8_M"];
        if present {
            return if dash_video_only.contains(&dash_video) {
                let supported_media = SupportedMedia {
                    components: vec![String::from(url)],
                    media_type: MediaType::RedditVideoWithoutAudio,
                };
                Ok(Some(supported_media))
            } else {
                let all = url.split("/").collect::<Vec<&str>>();
                let mut result = all.split_last().unwrap().1.to_vec();
                let dash_audio = "DASH_audio.mp4";
                result.push(dash_audio);

                // dynamically generate audio URLs for reddit videos by changing the video URL
                let audio_url = result.join("/");
                // Check the mime type to see the generated URL contains an audio file
                // This can be done by checking the content type header for the given URL
                // Reddit API response does not seem to expose any easy way to figure this out
                if let Some(audio_present) = check_url_is_mp4(&audio_url).await? {
                    if audio_present {
                        debug!("Found audio at URL {} for video {}", audio_url, dash_video);
                        let supported_media = SupportedMedia {
                            components: vec![String::from(url), audio_url],
                            media_type: MediaType::RedditVideoWithAudio,
                        };
                        Ok(Some(supported_media))
                    } else {
                        debug!(
                            "URL {} doesn't seem to have any associated audio at {}",
                            dash_video, audio_url
                        );
                        let supported_media = SupportedMedia {
                            components: vec![String::from(url)],
                            media_type: MediaType::RedditVideoWithoutAudio,
                        };
                        Ok(Some(supported_media))
                    }
                } else {
                    // todo: collapse this else block by removing the bool check
                    let supported_media = SupportedMedia {
                        components: vec![String::from(url)],
                        media_type: MediaType::RedditVideoWithoutAudio,
                    };
                    Ok(Some(supported_media))
                }
            };
        }
    }

    Ok(None)
}

/// Check if a particular URL contains supported media.
async fn get_media(data: &PostData) -> Result<Vec<SupportedMedia>, ReddSaverError> {
    let original = data.url.as_ref().unwrap();
    let mut media: Vec<SupportedMedia> = Vec::new();

    if let Ok(u) = Url::parse(original) {
        let mut parsed = u.clone();

        match parsed.path_segments_mut() {
            Ok(mut p) => p.pop_if_empty(),
            Err(_) => return Ok(media),
        };

        let url = &parsed[..Position::AfterPath];
        let gallery_info = data.gallery_data.borrow();

        // reddit images and gifs
        if url.contains(REDDIT_IMAGE_SUBDOMAIN) {
            // if the URL uses the reddit image subdomain and if the extension is
            // jpg, png or gif, then we can use the URL as is.
            if url.ends_with(JPG_EXTENSION) || url.ends_with(PNG_EXTENSION) {
                let translated = String::from(url);
                let supported_media = SupportedMedia {
                    components: vec![translated],
                    media_type: MediaType::RedditImage,
                };
                media.push(supported_media);
            }
            if url.ends_with(GIF_EXTENSION) {
                let translated = String::from(url);
                let translated = SupportedMedia {
                    components: vec![translated],
                    media_type: MediaType::RedditGif,
                };
                media.push(translated);
            }
        }

        // reddit mp4 videos
        if url.contains(REDDIT_VIDEO_SUBDOMAIN) {
            // if the URL uses the reddit video subdomain and if the extension is
            // mp4, then we can use the URL as is.
            if url.ends_with(MP4_EXTENSION) {
                let video_url = String::from(url);
                if let Some(supported_media) = get_reddit_video(&video_url).await? {
                    media.push(supported_media);
                }
            } else {
                // if the URL uses the reddit video subdomain, but the link does not
                // point directly to the mp4, then use the fallback URL to get the
                // appropriate link. The video quality might range from 96p to 720p
                if let Some(m) = &data.media {
                    if let Some(v) = &m.reddit_video {
                        let fallback_url =
                            String::from(&v.fallback_url).replace("?source=fallback", "");
                        if let Some(supported_media) = get_reddit_video(&fallback_url).await? {
                            media.push(supported_media);
                        }
                    }
                }
            }
        }

        // reddit image galleries
        if url.contains(REDDIT_DOMAIN) && url.contains(REDDIT_GALLERY_PATH) {
            if let Some(gallery) = gallery_info {
                // collect all the URLs for the images in the album
                let mut image_urls = Vec::new();
                for item in gallery.items.iter() {
                    // extract the media ID from each gallery item and reconstruct the image URL
                    let image_url = format!(
                        "https://{}/{}.{}",
                        REDDIT_IMAGE_SUBDOMAIN, item.media_id, JPG_EXTENSION
                    );
                    image_urls.push(image_url);
                }
                let supported_media =
                    SupportedMedia { components: image_urls, media_type: MediaType::RedditImage };
                media.push(supported_media);
            }
        }

        // gfycat
        if url.contains(GFYCAT_DOMAIN) {
            // if the Gfycat/Redgifs URL points directly to the mp4, download as is
            if url.ends_with(MP4_EXTENSION) {
                let supported_media = SupportedMedia {
                    components: vec![String::from(url)],
                    media_type: MediaType::GfycatGif,
                };
                media.push(supported_media);
            } else {
                // if the provided link is a gfycat post link, use the gfycat API
                // to get the URL. gfycat likes to use lowercase names in their posts
                // but the ID for the GIF is Pascal-cased. The case-conversion info
                // can only be obtained from the API at the moment
                if let Some(supported_media) = gfy_to_mp4(url).await? {
                    media.push(supported_media);
                }
            }
        }

        // split redgifs to its own handler
        if url.contains(REDGIFS_DOMAIN) {
            debug!("Found RG url {}", url);
            let supported_media = SupportedMedia {
                components: vec![String::from(url)],
                media_type: MediaType::RedgifsVideo,
            };
            media.push(supported_media);
            // we're going to pull the 'hd' link no matter what, so the extension doesn't matter
            // if url.contains(MP4_EXTENSION) {
            //     let supported_media = SupportedMedia {
            //         components: vec![String::from(url)],
            //         media_type: MediaType::RedgifsVideo,
            //     };
            //     media.push(supported_media);
            // } else {
            //     // if the provided link is a gfycat post link, use the gfycat API
            //     // to get the URL. gfycat likes to use lowercase names in their posts
            //     // but the ID for the GIF is Pascal-cased. The case-conversion info
            //     // can only be obtained from the API at the moment
            //     if let Some(supported_media) = gfy_to_mp4(url).await? {
            //         media.push(supported_media);
            //     }
            // }
        }

        // giphy
        if url.contains(GIPHY_DOMAIN) {
            // giphy has multiple CDN networks named {media0, .., media5}
            // links can point to the canonical media subdomain or any content domains
            if url.contains(GIPHY_MEDIA_SUBDOMAIN)
                || url.contains(GIPHY_MEDIA_SUBDOMAIN_0)
                || url.contains(GIPHY_MEDIA_SUBDOMAIN_1)
                || url.contains(GIPHY_MEDIA_SUBDOMAIN_2)
                || url.contains(GIPHY_MEDIA_SUBDOMAIN_3)
                || url.contains(GIPHY_MEDIA_SUBDOMAIN_4)
            {
                // if we encounter gif, mp4 or gifv - download as is
                if url.ends_with(GIF_EXTENSION)
                    || url.ends_with(MP4_EXTENSION)
                    || url.ends_with(GIFV_EXTENSION)
                {
                    let supported_media = SupportedMedia {
                        components: vec![String::from(url)],
                        media_type: MediaType::GiphyGif,
                    };
                    media.push(supported_media);
                }
            } else {
                // if the link points to the giphy post rather than the media link,
                // use the scheme below to get the actual URL for the gif.
                let path = &parsed[Position::AfterHost..Position::AfterPath];
                let media_id = path.split("-").last().unwrap();
                let supported_media = SupportedMedia {
                    components: vec![format!(
                        "https://{}/media/{}.gif",
                        GIPHY_MEDIA_SUBDOMAIN, media_id
                    )],
                    media_type: MediaType::GiphyGif,
                };
                media.push(supported_media);
            }
        }

        // imgur
        // NOTE: only support direct links for gifv and images
        // *No* support for image and gallery posts.
        if url.contains(IMGUR_DOMAIN) {
            if url.contains(IMGUR_SUBDOMAIN) && url.ends_with(GIFV_EXTENSION) {
                // if the extension is gifv, then replace gifv->mp4 to get the video URL
                let supported_media = SupportedMedia {
                    components: vec![url.replace(GIFV_EXTENSION, MP4_EXTENSION)],
                    media_type: MediaType::ImgurGif,
                };
                media.push(supported_media);
            }
            if url.contains(IMGUR_SUBDOMAIN)
                && (url.ends_with(PNG_EXTENSION) || url.ends_with(JPG_EXTENSION))
            {
                let supported_media = SupportedMedia {
                    components: vec![String::from(url)],
                    media_type: MediaType::ImgurImage,
                };
                media.push(supported_media);
            }
        }
    }

    Ok(media)
}
