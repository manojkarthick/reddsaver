use std::borrow::Borrow;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use log::{debug, error, info, warn};

use crate::errors::ReddSaverError;
use crate::structures::{Summary, UserSaved};
use crate::user::User;
use crate::utils::check_path_present;
use std::ops::Add;

static URL_EXTENSION_JPG: &str = "jpg";
static URL_EXTENSION_PNG: &str = "png";
static URL_PREFIX_REDDIT_GALLERY: &str = "reddit.com/gallery";
static URL_PREFIX_REDDIT_GALLERY_ITEM: &str = "https://i.redd.it";

/// Status of image processing
enum ImageStatus {
    /// If we are able to successfully download the image
    Downloaded,
    /// If we are skipping downloading the image due to it already being present
    /// or because we could not find the image or because we are unable to decode
    /// the image
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
            images_downloaded: 0,
            images_skipped: 0,
            images_supported: 0,
        };

        for collection in self.saved {
            full_summary = full_summary.add(self.download_collection(collection).await?);
        }

        info!("#####################################");
        info!("Download Summary:");
        info!(
            "Number of supported images: {}",
            full_summary.images_supported
        );
        info!(
            "Number of images downloaded: {}",
            full_summary.images_downloaded
        );
        info!("Number of images skipped: {}", full_summary.images_skipped);
        info!("#####################################");
        info!("FIN.");

        Ok(())
    }

    /// Download and save images from Reddit in parallel
    async fn download_collection(&self, collection: &UserSaved) -> Result<Summary, ReddSaverError> {
        let summary = Arc::new(Mutex::new(Summary {
            images_supported: 0,
            images_downloaded: 0,
            images_skipped: 0,
        }));

        collection
            .data
            .children
            .clone()
            .into_iter()
            // filter out the posts where a URL is present
            // not that this application cannot download URLs linked within the text of the post
            .filter(|item| item.data.url.is_some())
            .filter(|item| {
                let url_unwrapped = item.data.url.as_ref().unwrap();
                // currently the supported image hosting sites (reddit, imgur) use an image extension
                // at the end of the URLs. If the URLs end with jpg/png it is assumed to be an image
                url_unwrapped.ends_with(URL_EXTENSION_JPG)
                    || url_unwrapped.ends_with(URL_EXTENSION_PNG)
                    || url_unwrapped.contains(URL_PREFIX_REDDIT_GALLERY)
            })
            .map(|item| {
                let summary_arc = summary.clone();
                // since the latency for downloading an image from the network is unpredictable
                // we spawn a new async task for the each of the images to be downloaded
                async move {
                    let url = item.data.url.borrow().as_ref().unwrap();
                    let subreddit = item.data.subreddit.borrow();
                    let gallery_info = item.data.gallery_data.borrow();
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

                        // the set of images in this post is collected into this vector
                        let mut image_info: Vec<(u32, String)> = Vec::new();

                        // if the prefix is present in the URL we know that it's a reddit image gallery
                        if url.contains(URL_PREFIX_REDDIT_GALLERY) {
                            if let Some(gallery) = gallery_info {
                                for (index, item) in gallery.items.iter().enumerate() {
                                    // assemble the image URL from the media ID for the gallery item
                                    let img_url = format!(
                                        "{}/{}.{}",
                                        URL_PREFIX_REDDIT_GALLERY_ITEM,
                                        item.media_id,
                                        URL_EXTENSION_JPG
                                    );
                                    debug!("Image URL from Gallery: {:#?}", img_url);
                                    // push individual image URLs into the vector
                                    image_info.push((index as u32, img_url));
                                }
                            } else {
                                // empty galleries may be present when a user deletes the images present
                                // in the gallery or the mods remove it
                                warn!("Gallery at {} seems to be empty. Ignoring", url);
                            }
                        } else {
                            // these URLs are for posts that directly contain a single image
                            image_info.push((0, String::from(url)));
                        }

                        // every entry in this vector is a valid image
                        summary_arc.lock().unwrap().images_supported += image_info.len() as i32;

                        for (index, image_url) in &image_info {
                            let extension =
                                String::from(image_url.split('.').last().unwrap_or("unknown"));
                            let file_name = self.generate_file_name(
                                &image_url,
                                &subreddit,
                                &extension,
                                &post_name,
                                &post_title,
                                &index,
                            );
                            if self.should_download {
                                let image_status = save_or_skip(image_url, &file_name);
                                // update the summary statistics based on the status
                                match image_status.await? {
                                    ImageStatus::Downloaded => {
                                        summary_arc.lock().unwrap().images_downloaded += 1;
                                    }
                                    ImageStatus::Skipped => {
                                        summary_arc.lock().unwrap().images_skipped += 1
                                    }
                                }
                            } else {
                                info!("Image available at URL: {}", &image_url);
                                summary_arc.lock().unwrap().images_skipped += 1;
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
            "Number of supported images: {}",
            local_summary.images_supported
        );
        debug!(
            "Number of images downloaded: {}",
            local_summary.images_downloaded
        );
        debug!("Number of images skipped: {}", local_summary.images_skipped);

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
        index: &u32,
    ) -> String {
        return if !self.use_human_readable {
            // create a hash for the image using the URL the image is located at
            // this helps to make sure the image download always writes the same file
            // name irrespective of how many times it's run. If run more than once, the
            // image is overwritten by this method
            let hash = md5::compute(url);
            format!(
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

/// Helper function that downloads and saves a single image from Reddit or Imgur
async fn save_or_skip(url: &str, file_name: &str) -> Result<ImageStatus, ReddSaverError> {
    if check_path_present(&file_name) {
        debug!("Image from url {} already downloaded. Skipping...", url);
        Ok(ImageStatus::Skipped)
    } else {
        let save_status = download_image(&file_name, &url).await?;
        if save_status {
            Ok(ImageStatus::Downloaded)
        } else {
            Ok(ImageStatus::Skipped)
        }
    }
}

/// Download image from the given url and save to data directory. Also create data directory if not present already
async fn download_image(file_name: &str, url: &str) -> Result<bool, ReddSaverError> {
    // create directory if it does not already exist
    // the directory is created relative to the current working directory
    let directory = Path::new(file_name).parent().unwrap();
    match fs::create_dir_all(directory) {
        Ok(_) => (),
        Err(_e) => return Err(ReddSaverError::CouldNotCreateDirectory),
    }

    let data = reqwest::get(url).await?.bytes().await?;
    let mut output = File::create(&file_name)?;
    match io::copy(&mut data.as_ref(), &mut output) {
        Ok(_) => info!("Successfully saved image: {} from url {}", file_name, url),
        Err(_e) => {
            error!("Could not save image from url {} to {}", url, file_name);
            return Ok(false);
        }
    }

    Ok(true)
}
