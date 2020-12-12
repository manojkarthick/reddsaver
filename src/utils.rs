use crate::errors::ReddSaverError;
use crate::structures::{Summary, UserSaved};

use futures::stream::{FuturesUnordered, TryStreamExt};

use image::DynamicImage;
use rand;

use log::{debug, error, info, warn};
use rand::Rng;
use random_names::RandomName;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Generate user agent string of the form <name>:<version>.
///If no arguments passed generate random name and number
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

// this method has the same outcome as the `get_images_parallel` method
// this was an initial implementation and is left here for benchmarking purposes
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
            error!("Could not save image {} from url {}", file_name, url);
            // return Err(ReddSaverError::CouldNotSaveImageError(String::from(file_name)))
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
            url_unwrapped.ends_with("jpg") || url_unwrapped.ends_with("png")
        })
        .map(|item| {
            let summary_arc = summary.clone();
            // every entry in this closure is a valid image
            summary_arc.lock().unwrap().images_supported += 1;
            // since the latency for downloading an image from the network is unpredictable
            // we spawn a new async task for the each of the images to be downloaded
            async move {
                let url = item.data.url.unwrap();
                let extension = String::from(url.split('.').last().unwrap_or("unknown"));
                let subreddit = item.data.subreddit;

                let file_name = generate_file_name(&url, &data_directory, &subreddit, &extension);
                if check_path_present(&file_name) {
                    warn!("Image from url {} already downloaded. Skipping...", url);
                    summary_arc.lock().unwrap().images_skipped += 1;
                } else {
                    let image_bytes = reqwest::get(&url).await?.bytes().await?;
                    match image::load_from_memory(&image_bytes) {
                        Ok(image) => {
                            let save_status = save_image(&image, &file_name, &url)?;
                            if save_status {
                                summary_arc.lock().unwrap().images_downloaded += 1;
                            } else {
                                summary_arc.lock().unwrap().images_skipped += 1;
                            }
                        }
                        Err(_e) => {
                            error!(
                                "Encoding/Decoding error. Could not save create image from url {}",
                                url
                            );
                            summary_arc.lock().unwrap().images_skipped += 1;
                            // return Err(ReddSaverError::CouldNotCreateImageError(url, file_name))
                        }
                    };
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
