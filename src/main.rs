mod ao3;
mod config;
mod sftp;

use rocket::http::Status;
use rocket::serde::json::Json;
use serde::Deserialize;
use std::path::Path;
use url::Url;

use crate::ao3::common::{get_series_pages, DownloadFormat};
use crate::ao3::series::Series;
use crate::ao3::user;
use crate::ao3::work::Work;
use crate::config::check_config;
use crate::sftp::{upload_series, upload_work};

#[macro_use]
extern crate rocket;

#[derive(Deserialize)]
struct DownloadRequest<'r> {
    url: &'r str,
    device: Option<&'r str>,
}

#[get("/")]
fn index() -> &'static str { "Hello, world!" }

#[get("/test")]
async fn test() -> (Status, String) {
    (Status::Ok, String::from("Hello, world!"))
}

#[post("/download", format = "json", data = "<request>")]
async fn download(request: Json<DownloadRequest<'_>>) -> (Status, String) {
    let Ok(url) = Url::parse(request.url) else {
        return (Status::BadRequest, String::from("Could not parse provided URL"));
    };

    let mut url_path_segments = url.path_segments().unwrap();
    let url_type = url_path_segments.next().unwrap();
    let id = url_path_segments.next().unwrap();

    if url.host_str().unwrap() != "archiveofourown.org" {
        return (Status::BadRequest, String::from("URL has invalid host"));
    }
    if url_type != "works" && url_type != "series" {
        return (Status::BadRequest, String::from("URL is not for a series or work"));
    }

    let config = match config::read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (Status::InternalServerError, format!("Config Error: {}", error))
        }
    };
    let user = user::get_user(&config);
    let device = if let Some(device_name) = request.device {
        config.get_device_by_name(device_name).unwrap() //TODO error checking on this
    } else {
        config.devices.first().unwrap()
    };

    //TODO actually check for download errors
    match url_type {
        "works" => {
            let work = Work::parse_work(id, user.await.as_ref(), &config).await.unwrap();
            let _ = work.download(Path::new(&config.download_path), DownloadFormat::EPUB, None).await;
            //upload_work(&work, device, &config, DownloadFormat::EPUB, None, None)
        }
        "series" => {
            let series = Series::parse_series(id, user.await.as_ref(), &config).await.unwrap();
            let _ = series.download(Path::new(&config.download_path), DownloadFormat::EPUB).await;
            //upload_series(&series, device, &config, DownloadFormat::EPUB);
        }
        _ => unreachable!(),
    };

    (Status::Ok, format!("Successfully downloaded {url_type} with id {id}"))
}

#[launch]
async fn rocket() -> _ {
    check_config().await;
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![download])
        .mount("/", routes![test])
}
