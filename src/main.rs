mod ao3;
mod config;
mod sftp;


use crate::ao3::common::{parse_url, DownloadFormat, PageType};
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
    
    let url_info = match parse_url(&url) {
        Ok(url_info) => url_info,
        Err(error) => return (Status::BadRequest, error.to_string())
    };

    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (Status::InternalServerError, format!("Config Error: {error}"))
        }
    };
    
    let device = config.get_device_by_name_or_first(request.device);

    match url_info.page_type {
        PageType::Work => {
            let work_result = Work::parse_work(&url_info.id, user, &config, request.fandom_override).await;
            let Ok(work) = work_result else {
                return (Status::BadRequest, work_result.err().unwrap().to_string());
            };
            let download_result = work
                .download(Path::new(&config.download_path), DownloadFormat::EPUB, None, user)
                .await;
            let Ok(()) = download_result else {
              return (Status::BadRequest, download_result.err().unwrap().to_string());  
            };
            let upload_result = upload_work(&work, device, &config, DownloadFormat::EPUB, None, None).await;
            let Ok(()) = upload_result else {
                return (Status::BadRequest, upload_result.err().unwrap().to_string());
            };
            
        },
        PageType::Series => {
            let series_result = Series::parse_series(&url_info.id, user, &config).await;
            let Ok(series) = series_result else {
                return (Status::BadRequest, series_result.err().unwrap().to_string());
            };
            let download_result = series
                .download(Path::new(&config.download_path), DownloadFormat::EPUB, user)
                .await;
            let Ok(()) = download_result else {
                return (Status::BadRequest, download_result.err().unwrap().to_string());
            };
            let upload_result = upload_series(&series, device, &config, DownloadFormat::EPUB).await;
            let Ok(()) = upload_result else {
                return (Status::BadRequest, upload_result.err().unwrap().to_string());
            };
        }
    }

    match user.write_cookies() {
        Ok(()) => {},
        Err(error) => {
            return (Status::InternalServerError, format!("File error while writing cookies: {error}"))
        }
    }

    (Status::Ok, format!("Successfully downloaded {} with id {}", url_info.page_type, url_info.id))
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
            return (Status::InternalServerError, format!("Config Error: {error}"))
        }
    };
    
    let device = config.get_device_by_name_or_first(request.device);
    
    let work = test_work(request.work.to_owned(), request.fandom.to_owned(), request.series, request.part_in_series);
    
    if request.series.is_some() {
        let upload_result = upload_work(&work, device, &config, DownloadFormat::EPUB, None, Some(&"1".to_owned())).await;
        let Ok(()) = upload_result else {
            return (Status::BadRequest, upload_result.err().unwrap().to_string());
        };
    } else {
        let upload_result = upload_work(&work, device, &config, DownloadFormat::EPUB, None, None).await;
        let Ok(()) = upload_result else {
            let error = upload_result.err().unwrap();
            println!("{}", error.root_cause());
            return (Status::BadRequest, error.to_string());
        };
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
                    eprintln!("User Error: {error}");
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
            eprintln!("Config Error: {error}");
            std::process::exit(1)
        }
    }
}
