use crate::crawler::Crawler;
use lazy_static::lazy_static;
use regex::Regex;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub fn crawl_posts(crawler: &Crawler, output: PathBuf, posts: Vec<String>) {
    // Crawl the info json.
    let post_urls: Vec<_> = posts
        .iter()
        .map(|id| format!("https://api.fanbox.cc/post.info?postId={id}"))
        .collect();
    let post_requests = post_urls
        .iter()
        .map(|url| (url.as_str(), Vec::new()))
        .collect();
    let post_results = crawler.get_json("Post Infos", post_requests);
    let posts = posts
        .iter()
        .zip(post_results)
        .filter_map(|(id, info)| match info {
            Ok(info) => Some((id, info)),
            Err(err) => {
                println!("Fail to crawl the info json for Post {id}: {err}");
                None
            }
        });

    for (id, info) in posts {
        // Extract basic information.
        let user = info["body"]["user"]["name"].as_str().unwrap();
        let date = {
            let date = info["body"]["publishedDatetime"].as_str().unwrap();
            lazy_static! {
                static ref DATE_REGEX: Regex =
                    Regex::new(r"([0-9]{2})-([0-9]{2})-([0-9]{2})").unwrap();
            }
            let caps = DATE_REGEX.captures(date).unwrap();
            format!("{}{}{}", &caps[1], &caps[2], &caps[3])
        };
        let title = info["body"]["title"].as_str().unwrap();
        let name = sanitize_filename::sanitize(format!("[{user}] [{date}] {title}"));
        let mut output = output.join(&name);

        // Create a directory if there is more than one image.
        let image_map = info["body"]["body"]["imageMap"].as_object().unwrap();
        let image_urls: Vec<_> = info["body"]["body"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|blk| {
                let blk = blk.as_object().unwrap();
                if blk["type"] == "image" {
                    let image_id = blk["imageId"].as_str().unwrap();
                    Some((
                        image_map[image_id]["originalUrl"].as_str().unwrap(),
                        image_map[image_id]["extension"].as_str().unwrap(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        if image_urls.len() > 1 {
            fs::create_dir(&output).unwrap();
        };

        // Crawl images in this post.
        let image_requests = image_urls
            .iter()
            .map(|(url, _)| (*url, Vec::new()))
            .collect();
        let image_results = crawler.get_byte(&name, image_requests);

        // Write images to local files.
        for (i, ((_, ext), image)) in image_urls.iter().zip(image_results).enumerate() {
            let image = match image {
                Ok(image) => image,
                Err(err) => {
                    println!("Fail to crawl Image {} for Post {id}: {err}", i + 1);
                    continue;
                }
            };
            if image_urls.len() == 1 {
                output.set_extension(ext);
                let mut file = File::create(&output).unwrap();
                file.write_all(&image).unwrap();
            } else {
                let mut path = output.clone();
                path.push(format!("{:0>4}.{ext}", i + 1));
                let mut file = File::create(path).unwrap();
                file.write_all(&image).unwrap();
            }
        }
    }
}
