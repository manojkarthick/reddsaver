use crate::errors::ReddSaverError;
use crate::structures::UserSaved;

use image::DynamicImage;

use log::info;
use std::fs;

#[allow(dead_code)]
// this method has the same outcome as the `get_images_parallel` method
// this was an initial implementation and is left here for benchmarking purposes
pub async fn get_images(saved: &UserSaved) -> Result<(), ReddSaverError> {
    for child in saved.data.children.iter() {
        let child_cloned = child.clone();
        if let Some(url) = child_cloned.data.url {
            let extension = String::from(url.split('.').last().unwrap_or("unknown"));
            let subreddit = child_cloned.data.subreddit;
            if extension == "jpg" || extension == "png" {
                info!("Downloading image from URL: {}", url);
                let image_bytes = reqwest::get(&url).await?.bytes().await?;
                let image = match image::load_from_memory(&image_bytes) {
                    Ok(image) => image,
                    Err(_e) => return Err(ReddSaverError::CouldNotCreateImageError),
                };
                let file_name = save_image(&image, &url, &subreddit, &extension)?;
                info!("Successfully saved image: {}", file_name);
            }
        }
    }

    Ok(())
}

/// Takes a binary image blob and save it to the filesystem
fn save_image(
    image: &DynamicImage,
    url: &str,
    subreddit: &str,
    extension: &str,
) -> Result<String, ReddSaverError> {
    // create directory if it does not already exist
    // the directory is created relative to the current working directory
    match fs::create_dir_all(format!("data/{}", subreddit)) {
        Ok(_) => (),
        Err(_e) => return Err(ReddSaverError::CouldNotCreateDirectory),
    }

    // create a hash for the image using the URL the image is located at
    // this helps to make sure the image download always writes the same file
    // name irrespective of how many times it's run. If run more than once, the
    // image is overwritten by this method
    let hash = md5::compute(url);
    let file_name = format!("data/{}/img-{:x}.{}", subreddit, hash, extension);
    match image.save(&file_name) {
        Ok(_) => (),
        Err(_e) => return Err(ReddSaverError::CouldNotSaveImageError),
    }

    Ok(file_name)
}

pub async fn get_images_parallel(saved: &UserSaved) -> Result<(), ReddSaverError> {
    let tasks: Vec<_> = saved
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
            // since the latency for downloading an image from the network is unpredictable
            // we spawn a new async task using tokio for the each of the images to be downloaded
            tokio::spawn(async {
                let url = item.data.url.unwrap();
                let extension = String::from(url.split('.').last().unwrap_or("unknown"));
                let subreddit = item.data.subreddit;
                info!("Downloading image from URL: {}", url);
                let image_bytes = reqwest::get(&url).await?.bytes().await?;
                let image = match image::load_from_memory(&image_bytes) {
                    Ok(image) => image,
                    Err(_e) => return Err(ReddSaverError::CouldNotCreateImageError),
                };
                let file_name = save_image(&image, &url, &subreddit, &extension)?;
                info!("Successfully saved image: {}", file_name);
                Ok::<(), ReddSaverError>(())
            })
        })
        .collect();

    // wait for all the images to be downloaded and saved to disk before exiting the method
    for task in tasks {
        if let Err(e) = task.await? {
            return Err(e);
        }
    }

    Ok(())
}
