mod ao3;
mod config;
mod sftp;

use std::path::Path;
use serde::Deserialize;
use rocket::http::{Status, ContentType};
use rocket::response::{status, Response};
use rocket::serde::json::Json;
use url::Url;
use crate::ao3::common::DownloadFormat;
use crate::ao3::series::Series;
use crate::ao3::user;
use crate::ao3::work::Work;

#[macro_use] extern crate rocket;

#[derive(Deserialize)]
struct DownloadRequest<'r> {
    url: &'r str
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/download", format = "json", data = "<request>")]
fn download(request: Json<DownloadRequest<'_>>) -> (Status, String) {
    let Ok(url) = Url::parse(request.url) else {
        return (Status::BadRequest, String::from("Could not parse provided URL"))
    };

    let mut url_path_segments = url.path_segments().unwrap();
    let url_type = url_path_segments.next().unwrap();
    let id = url_path_segments.next().unwrap();

    if url.host_str().unwrap() != "archiveofourown.org" {
        return (Status::BadRequest, String::from("URL has invalid host"))
    }
    if url_type != "works" && url_type != "series" {
        return (Status::BadRequest, String::from("URL is not for a series or work"))
    }
    
    let config = config::read_config();
    let user = user::get_user(&config);

    let _ = match url_type {
        "works" => Work::parse_work(id, user.as_ref(), &config)
            .unwrap()
            .download(Path::new(&config.download_path), DownloadFormat::EPUB, None),
        "series" => Series::parse_series(id, user.as_ref(), &config)
            .unwrap()
            .download(Path::new(&config.download_path), DownloadFormat::EPUB),
        _ => unreachable!()
    };
    
    let string = String::from("test");
    
    let test = "test".to_string();

    println!("type: {}, id: {}", url_type, id);
    (Status::Ok, format!("Successfully downloaded {url_type} with id {id}"))
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![download])
}
