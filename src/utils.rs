use crate::saved::UserSaved;
use reqwest;
use image;
use std::fs;
use image::DynamicImage;
use md5;

pub async fn get_images(saved: &UserSaved) -> Result<(), Box<dyn std::error::Error>>{

    for child in saved.data.children.iter() {
        let child_cloned = child.clone();
        match child_cloned.data.url {
            Some(url) => {
                let extension = String::from(url.split(".").last().unwrap_or("unknown"));
                let subreddit = child_cloned.data.subreddit;
                if extension == "jpg" || extension == "png" {
                    println!("Downloading image from URL: {}", url);
                    let image_bytes = reqwest::get(&url)
                        .await?
                        .bytes()
                        .await?;
                    let image = image::load_from_memory(&image_bytes)?;
                    save_image(image, &url, &subreddit, &extension)?;
                    println!("Saved!");
                }
            },
            None => ()
        }
    }

    Ok(())
}

fn save_image(image: DynamicImage, url: &String, subreddit: &str, extension: &str) -> Result<(), Box<dyn std::error::Error>>{

    fs::create_dir_all(subreddit)?;

    let hash = md5::compute(url);
    image.save(format!("{}/img-{:x}.{}", subreddit, hash, extension))?;

    Ok(())
}
