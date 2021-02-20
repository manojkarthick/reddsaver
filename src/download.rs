use std::borrow::Borrow;
use std::fs::File;
use std::ops::Add;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use log::{debug, error, info, warn};
use url::{Position, Url};

use crate::errors::ReddSaverError;
use crate::structures::{GfyData, PostData};
use crate::structures::{Summary, UserSaved};
use crate::user::User;
use crate::utils::check_path_present;
use reqwest::StatusCode;

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

/// Status of media processing
enum MediaStatus {
    /// If we are able to successfully download the media
    Downloaded,
    /// If we are skipping downloading the media due to it already being present
    /// or because we could not find the media or because we are unable to decode
    /// the media
    Skipped,
}

#[derive(Debug)]
pub struct Downloader<'a> {
    user: &'a User<'a>,
    saved: &'a Vec<UserSaved>,
    data_directory: &'a str,
    subreddits: &'a Option<Vec<&'a str>>,
    should_download: bool,
    use_human_readable: bool,
    unsave: bool,
}

impl<'a> Downloader<'a> {
    pub fn new(
        user: &'a User,
        saved: &'a Vec<UserSaved>,
        data_directory: &'a str,
        subreddits: &'a Option<Vec<&'a str>>,
        should_download: bool,
        use_human_readable: bool,
        unsave: bool,
    ) -> Downloader<'a> {
        Downloader {
            user,
            saved,
            data_directory,
            subreddits,
            should_download,
            use_human_readable,
            unsave,
        }
    }

    pub async fn run(self) -> Result<(), ReddSaverError> {
        let mut full_summary = Summary {
            media_downloaded: 0,
            media_skipped: 0,
            media_supported: 0,
        };

        for collection in self.saved {
            full_summary = full_summary.add(self.download_collection(collection).await?);
        }

        info!("#####################################");
        info!("Download Summary:");
        info!(
            "Number of supported media: {}",
            full_summary.media_supported
        );
        info!(
            "Number of media downloaded: {}",
            full_summary.media_downloaded
        );
        info!("Number of media skipped: {}", full_summary.media_skipped);
        info!("#####################################");
        info!("FIN.");

        Ok(())
    }

    /// Download and save medias from Reddit in parallel
    async fn download_collection(&self, collection: &UserSaved) -> Result<Summary, ReddSaverError> {
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

                        let media = get_media(item.data.borrow()).await?;
                        // every entry in this vector is valid media
                        summary_arc.lock().unwrap().media_supported += media.len() as i32;

                        for (index, url) in media.iter().enumerate() {
                            let extension =
                                String::from(url.split('.').last().unwrap_or("unknown"));
                            let file_name = self.generate_file_name(
                                &url,
                                &subreddit,
                                &extension,
                                &post_name,
                                &post_title,
                                &index,
                            );

                            if self.should_download {
                                let status = save_or_skip(url, &file_name);
                                // update the summary statistics based on the status
                                match status.await? {
                                    MediaStatus::Downloaded => {
                                        summary_arc.lock().unwrap().media_downloaded += 1;
                                    }
                                    MediaStatus::Skipped => {
                                        summary_arc.lock().unwrap().media_skipped += 1
                                    }
                                }
                            } else {
                                info!("Media available at URL: {}", &url);
                                summary_arc.lock().unwrap().media_skipped += 1;
                            }
                        }
                    } else {
                        debug!(
                            "Subreddit INVALID!: {} NOT present in {:#?}",
                            subreddit, self.subreddits
                        );
                    }

                    if self.unsave {
                        self.user.unsave(post_name).await?;
                    }

                    Ok::<(), ReddSaverError>(())
                }
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<()>()
            .await?;

        let local_summary = *summary.lock().unwrap();

        debug!("Collection statistics: ");
        debug!(
            "Number of supported media: {}",
            local_summary.media_supported
        );
        debug!(
            "Number of media downloaded: {}",
            local_summary.media_downloaded
        );
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
        index: &usize,
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
                    if c.is_whitespace() || c == '/' || c == '\\' {
                        '_'
                    } else {
                        c
                    }
                })
                .collect();
            // create a canonical human readable file name using the post's title
            // note that the name of the post is something of the form t3_<randomstring>
            let canonical_name: String = if *index == 0 {
                String::from(name)
            } else {
                format!("{}_{}", name, index)
            };

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
        debug!("Image from url {} already downloaded. Skipping...", url);
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

    let maybe_response = reqwest::get(url).await;
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
                    warn!(
                        "Could not create a file with the name: {}. Skipping",
                        file_name
                    );
                }
            }
        }
    }

    Ok(status)
}

/// Convert Gfycat/Redgifs GIFs into mp4 URLs for download
async fn gfy_to_mp4(url: &str) -> Result<Option<String>, ReddSaverError> {
    let api_prefix = if url.contains(GFYCAT_DOMAIN) {
        GFYCAT_API_PREFIX
    } else {
        REDGIFS_API_PREFIX
    };
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
            Ok(Some(data.gfy_item.mp4_url))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Check if a particular URL contains supported media.
async fn get_media(data: &PostData) -> Result<Vec<String>, ReddSaverError> {
    let original = data.url.as_ref().unwrap();
    let mut media: Vec<String> = Vec::new();

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
            if url.ends_with(JPG_EXTENSION)
                || url.ends_with(PNG_EXTENSION)
                || url.ends_with(GIF_EXTENSION)
            {
                let translated = String::from(url);
                media.push(translated);
            }
        }

        // reddit mp4 videos
        if url.contains(REDDIT_VIDEO_SUBDOMAIN) {
            // if the URL uses the reddit video subdomain and if the extension is
            // mp4, then we can use the URL as is.
            if url.ends_with(MP4_EXTENSION) {
                let translated = String::from(url);
                media.push(translated);
            } else {
                // if the URL uses the reddit video subdomain, but the link does not
                // point directly to the mp4, then use the fallback URL to get the
                // appropriate link. The video quality might range from 96p to 720p
                if let Some(m) = &data.media {
                    if let Some(v) = &m.reddit_video {
                        let translated =
                            String::from(&v.fallback_url).replace("?source=fallback", "");
                        media.push(translated);
                    }
                }
            }
        }

        // reddit image galleries
        if url.contains(REDDIT_DOMAIN) && url.contains(REDDIT_GALLERY_PATH) {
            if let Some(gallery) = gallery_info {
                for item in gallery.items.iter() {
                    // extract the media ID from each gallery item and reconstruct the image URL
                    let translated = format!(
                        "https://{}/{}.{}",
                        REDDIT_IMAGE_SUBDOMAIN, item.media_id, JPG_EXTENSION
                    );
                    media.push(translated);
                }
            }
        }

        // gfycat and redgifs
        if url.contains(GFYCAT_DOMAIN) || url.contains(REDGIFS_DOMAIN) {
            // if the Gfycat/Redgifs URL points directly to the mp4, download as is
            if url.ends_with(MP4_EXTENSION) {
                let translated = String::from(url);
                media.push(translated);
            } else {
                // if the provided link is a gfycat post link, use the gfycat API
                // to get the URL. gfycat likes to use lowercase names in their posts
                // but the ID for the GIF is Pascal-cased. The case-conversion info
                // can only be obtained from the API at the moment
                if let Some(mp4_url) = gfy_to_mp4(url).await? {
                    media.push(mp4_url);
                }
            }
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
                    let translated = String::from(url);
                    media.push(translated);
                }
            } else {
                // if the link points to the giphy post rather than the media link,
                // use the scheme below to get the actual URL for the gif.
                let path = &parsed[Position::AfterHost..Position::AfterPath];
                let media_id = path.split("-").last().unwrap();
                let translated =
                    format!("https://{}/media/{}.gif", GIPHY_MEDIA_SUBDOMAIN, media_id);
                media.push(translated);
            }
        }

        // imgur
        // NOTE: only support direct links for gifv and images
        // *No* support for image and gallery posts.
        if url.contains(IMGUR_DOMAIN) {
            if url.contains(IMGUR_SUBDOMAIN) && url.ends_with(GIFV_EXTENSION) {
                // if the extension is gifv, then replace gifv->mp4 to get the video URL
                let translated = url.replace(GIFV_EXTENSION, MP4_EXTENSION);
                media.push(translated);
            }
            if url.contains(IMGUR_SUBDOMAIN)
                && (url.ends_with(PNG_EXTENSION) || url.ends_with(JPG_EXTENSION))
            {
                let translated = String::from(url);
                media.push(translated);
            }
        }
    }

    Ok(media)
}
