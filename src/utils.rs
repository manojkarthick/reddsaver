use crate::errors::ReddSaverError;
use crate::structures::{Summary, UserSaved};

use futures::stream::{FuturesUnordered, TryStreamExt};

use image::DynamicImage;
use rand;

use log::{debug, error, info, warn};
use rand::Rng;
use random_names::RandomName;
use std::borrow::Borrow;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

static URL_EXTENSION_JPG: &str = "jpg";
static URL_EXTENSION_PNG: &str = "png";
static URL_PREFIX_REDDIT_GALLERY: &str = "reddit.com/gallery";
static URL_PREFIX_REDDIT_GALLERY_ITEM: &str = "https://i.redd.it";

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

// this method had the same outcome as the `get_images_parallel` method initially
// this was a naive implementation and is left here for reference
// pub async fn get_images(saved: &UserSaved) -> Result<(), ReddSaverError> {
//     for child in saved.data.children.iter() {
//         let child_cloned = child.clone();
//         if let Some(url) = child_cloned.data.url {
//             let extension = String::from(url.split('.').last().unwrap_or("unknown"));
//             let subreddit = child_cloned.data.subreddit;
//             if extension == "jpg" || extension == "png" {
//                 info!("Downloading image from URL: {}", url);
//                 let image_bytes = reqwest::get(&url).await?.bytes().await?;
//                 let image = match image::load_from_memory(&image_bytes) {
//                     Ok(image) => image,
//                     Err(_e) => return Err(ReddSaverError::CouldNotCreateImageError),
//                 };
//                 let file_name = save_image(&image, &url, &subreddit, &extension)?;
//                 info!("Successfully saved image: {}", file_name);
//             }
//         }
//     }
//
//     Ok(())
// }

/// Takes a binary image blob and save it to the filesystem
fn save_image(image: &DynamicImage, file_name: &str, url: &str) -> Result<bool, ReddSaverError> {
    // create directory if it does not already exist
    // the directory is created relative to the current working directory
    let directory = Path::new(file_name).parent().unwrap();
    match fs::create_dir_all(directory) {
        Ok(_) => (),
        Err(_e) => return Err(ReddSaverError::CouldNotCreateDirectory),
    }

    match image.save(&file_name) {
        Ok(_) => info!("Successfully saved image: {} from url {}", file_name, url),
        Err(_e) => {
            error!("Could not save image from url {} to {}", url, file_name);
            return Ok(false);
        }
    }

    Ok(true)
}

/// Check if a particular path is present on the filesystem
pub fn check_path_present(file_path: &str) -> bool {
    Path::new(file_path).exists()
}

/// Generate a file name in the right format that Reddsaver expects
fn generate_file_name(url: &str, data_directory: &str, subreddit: &str, extension: &str) -> String {
    // create a hash for the image using the URL the image is located at
    // this helps to make sure the image download always writes the same file
    // name irrespective of how many times it's run. If run more than once, the
    // image is overwritten by this method
    let hash = md5::compute(url);
    format!(
        "{}/{}/img-{:x}.{}",
        data_directory, subreddit, hash, extension
    )
}

/// Status of image processing
enum ImageStatus {
    /// If we are able to successfully download the image
    Downloaded,
    /// If we skipping downloading the image due to it already being present
    /// or because we could not find the image or because we are unable to decode
    /// the image
    Skipped,
}

/// Helper function that downloads and saves a single image from Reddit or Imgur
async fn process_single_image(url: &str, file_name: &str) -> Result<ImageStatus, ReddSaverError> {
    if check_path_present(&file_name) {
        warn!("Image from url {} already downloaded. Skipping...", url);
        return Ok(ImageStatus::Skipped);
    // summary_arc.lock().unwrap().images_skipped += 1;
    } else {
        let image_bytes = reqwest::get(url).await?.bytes().await?;
        match image::load_from_memory(&image_bytes) {
            Ok(image) => {
                let save_status = save_image(&image, &file_name, &url)?;
                if save_status {
                    return Ok(ImageStatus::Downloaded);
                } else {
                    return Ok(ImageStatus::Skipped);
                }
            }
            Err(_e) => {
                error!(
                    "Encoding/Decoding error. Could not save create image from url {}",
                    url
                );
                return Ok(ImageStatus::Skipped);
            }
        };
    }
}

/// Download and save images from Reddit in parallel
pub async fn get_images_parallel(
    saved: &UserSaved,
    data_directory: &str,
) -> Result<Summary, ReddSaverError> {
    let summary = Arc::new(Mutex::new(Summary {
        images_supported: 0,
        images_downloaded: 0,
        images_skipped: 0,
    }));

    saved
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

                // the set of images in this post is collected into this vector
                let mut image_urls: Vec<String> = Vec::new();

                // if the prefix is present in the URL we know that it's a reddit image gallery
                if url.contains(URL_PREFIX_REDDIT_GALLERY) {
                    if let Some(gallery) = gallery_info {
                        for item in &gallery.items {
                            // assemble the image URL from the media ID for the gallery item
                            let _image_url = format!(
                                "{}/{}.{}",
                                URL_PREFIX_REDDIT_GALLERY_ITEM, item.media_id, URL_EXTENSION_JPG
                            );
                            debug!("Image URL from Gallery: {:#?}", _image_url);
                            // push individual image URLs into the vector
                            image_urls.push(_image_url);
                        }
                    } else {
                        // empty galleries may be present when a user deletes the images present
                        // in the gallery or the mods remove it
                        warn!("Gallery at {} seems to be empty. Ignoring", url)
                    }
                } else {
                    // these URLs are for posts that directly contain a single image
                    image_urls.push(String::from(url))
                }

                // every entry in this vector is a valid image
                summary_arc.lock().unwrap().images_supported += image_urls.len() as i32;

                for image_url in &image_urls {
                    let extension = String::from(image_url.split(".").last().unwrap_or("unknown"));
                    let file_name =
                        generate_file_name(&image_url, &data_directory, &subreddit, &extension);
                    let image_status = process_single_image(image_url, &file_name);
                    // update the summary statistics based on the status
                    match image_status.await? {
                        ImageStatus::Downloaded => {
                            summary_arc.lock().unwrap().images_downloaded += 1
                        }
                        ImageStatus::Skipped => summary_arc.lock().unwrap().images_skipped += 1,
                    }
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

    let x = Ok(local_summary);
    x
}
