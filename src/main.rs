mod ao3;
mod config;
mod sftp;


use crate::ao3::common::{parse_url, DownloadFormat};
use crate::ao3::series::Series;
use crate::ao3::user;
use crate::ao3::work::{test_work, Work};
use crate::config::read_config;
use crate::sftp::{upload_series, upload_work};

use std::path::Path;
use rocket::http::Status;
use rocket::serde::json::Json;
use serde::Deserialize;
use rocket::State;
use url::Url;

#[macro_use]
extern crate rocket;

#[derive(Deserialize)]
struct DownloadRequest<'r> {
    url: &'r str,
    device: Option<&'r str>,
    fandom_override: Option<&'r str>,
}

#[post("/download", format = "json", data = "<request>")]
async fn download(request: Json<DownloadRequest<'_>>, user: &State<user::User>) -> (Status, String) {
    let Ok(url) = Url::parse(request.url) else {
        return (Status::BadRequest, String::from("Could not parse provided URL"));
    };
    
    let (id, url_type) = parse_url(&url);

    if url.host_str().unwrap() != "archiveofourown.org" {
        return (Status::BadRequest, String::from("URL has invalid host"));
    }
    if url_type != "works" && url_type != "series" {
        return (Status::BadRequest, String::from("URL is not for a series or work"));
    }

    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (Status::InternalServerError, format!("Config Error: {}", error))
        } 
    };
    
    let device = config.get_device_by_name_or_first(request.device);

    //TODO actually check for download errors
    match url_type.as_str() {
        "works" => {
            let work = Work::parse_work(&id, user, &config, request.fandom_override)
                .await
                .unwrap();
            let _ = work
                .download(Path::new(&config.download_path), DownloadFormat::EPUB, None, user)
                .await;
            upload_work(&work, device, &config, DownloadFormat::EPUB, None, None).await
        }
        "series" => {
            let series = Series::parse_series(&id, user, &config)
                .await
                .unwrap();
            let _ = series
                .download(Path::new(&config.download_path), DownloadFormat::EPUB, user)
                .await;
            upload_series(&series, device, &config, DownloadFormat::EPUB).await
        }
        _ => unreachable!(),
    };

    match user.write_cookies() {
        Ok(_) => {},
        Err(error) => {
            return (Status::InternalServerError, format!("File error while writing cookies: {}", error))
        }
    }

    (Status::Ok, format!("Successfully downloaded {url_type} with id {id}"))
}

#[derive(Deserialize)]
struct UploadRequest<'r> {
    work: &'r str,
    fandom: &'r str,
    series: Option<&'r str>,
    part_in_series: Option<&'r str>,
    device: Option<&'r str>,
}

#[post("/upload", format = "json", data = "<request>")]
async fn upload(request: Json<UploadRequest<'_>>) -> (Status, String) {
    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (Status::InternalServerError, format!("Config Error: {}", error))
        }
    };
    
    let device = config.get_device_by_name_or_first(request.device);
    
    let work = test_work(request.work.to_owned(), request.fandom.to_owned(), request.series, request.part_in_series);
    
    match request.series {
        None => { upload_work(&work, device, &config, DownloadFormat::EPUB, None, None).await }
        Some(_) => { upload_work(&work, device, &config, DownloadFormat::EPUB, None, Some(&"1".to_owned())).await }
    }

    (Status::Ok, format!("Successfully uploaded {} to {}", request.work, request.device.unwrap()))
}

#[launch]
async fn rocket() -> _ {
    match read_config().await {
        Ok(config) => {
            // if need to sort out CORS https://github.com/lawliet89/rocket_cors/blob/master/examples/fairing.rs
            let port = config.port;
            let user = match user::get_user(config).await {
                Ok(user) => user,
                Err(error) => {
                    eprintln!("User Error: {}", error);
                    std::process::exit(1);
                }
            };
            
            rocket::build()
                .configure(
                    rocket::Config::figment()
                        .merge(("port", port))
                        .merge(("address", "0.0.0.0"))
                )
                .manage(user)
                .mount("/", routes![download])
                .mount("/", routes![upload])
        },
        Err(error) => {
            eprintln!("Config Error: {}", error);
            std::process::exit(1)
        }
    }
}
