mod ao3;
mod config;
mod sftp;


use std::ffi::OsStr;
use crate::ao3::common::{parse_url, DownloadFormat, PageType};
use crate::ao3::series::Series;
use crate::ao3::user;
use crate::ao3::work::Work;
use crate::config::read_config;
use crate::sftp::{upload_series, upload_work};

use epub::doc::EpubDoc;
use rocket::http::{Header, Status};
use rocket::response::content;
use rocket::serde::json::Json;
use rocket::{State, Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use serde::Deserialize;
use serde_json::to_string_pretty;
use std::io::prelude::*;
use std::fs::{File, read_dir};
use std::path::Path;
use url::Url;

#[macro_use]
extern crate rocket;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[derive(Deserialize)]
struct DownloadRequest<'r> {
    url: &'r str,
    device: Option<&'r str>,
    fandom_override: Option<&'r str>,
}

#[get("/")]
fn index() -> content::RawHtml<&'static str> {
    content::RawHtml("Hello 👋")
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

    //TODO should also log any errors
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
struct UploadWorkRequest<'r> {
    work: &'r str,
    fandom: &'r str,
    series: Option<&'r str>,
    part_in_series: Option<&'r str>,
    device: Option<&'r str>,
}

#[post("/upload/work", format = "json", data = "<request>")]
async fn upload_work_api(request: Json<UploadWorkRequest<'_>>) -> (Status, String) {
    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (Status::InternalServerError, format!("Config Error: {error}"))
        }
    };
    
    let device = config.get_device_by_name_or_first(request.device);
    
    let work = Work::test_work(
        request.work,
        request.fandom,
        request.series,
        Some(request.part_in_series.unwrap().parse::<u8>().unwrap()),
    );
    
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

#[derive(Deserialize)]
struct UploadSeriesRequest<'r> {
    series: &'r str,
    fandom: &'r str,
    device: Option<&'r str>,
}

#[post("/upload/series", format = "json", data = "<request>")]
async fn upload_series_api(request: Json<UploadSeriesRequest<'_>>) -> (Status, String) {
    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (Status::InternalServerError, format!("Config Error: {error}"))
        }
    };

    let device = config.get_device_by_name_or_first(request.device);

    let series = match Series::test_series(request.series, request.fandom, &config) {
        Ok(series) => series,
        Err(error) => {
            return (Status::InternalServerError, format!("Series Upload Error: {error}"))
        }
    };

    let upload_result = upload_series(&series, device, &config, DownloadFormat::EPUB).await;
    let Ok(()) = upload_result else {
        let error = upload_result.err().unwrap();
        println!("{}", error.root_cause());
        return (Status::BadRequest, error.to_string());
    };

    (Status::Ok, format!("Successfully uploaded {} to {}", request.series, request.device.unwrap()))
}

#[get("/meta")]
fn meta() -> (Status, String) {
    let files = read_dir("downloads").unwrap();
    for file in files {
        let file = file.unwrap();
        let path = file.path();
        if !path.is_dir() && path.extension() == Some(OsStr::new("epub")) {
            let mut doc = EpubDoc::new(&path).unwrap();
            //doc.go_next();
            //doc.go_next();
            let test = doc.get_current().unwrap();
            //let mut metadata_file = File::create(path.with_extension("json")).unwrap();
            //metadata_file.write_all(&to_string_pretty(&doc.metadata).unwrap().into_bytes()).unwrap();
            println!("{}", String::from_utf8(test.0).unwrap());
        }
    }

    (Status::Ok, "Ok".to_string())
}

#[get("/healthcheck")]
fn healthcheck() -> (Status, String) {
    (Status::Ok, "A2O4 is running".to_string())
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
                .attach(CORS)
                .mount("/", routes![index])
                .mount("/", routes![download])
                .mount("/", routes![upload_series_api])
                .mount("/", routes![upload_work_api])
                .mount("/", routes![meta])
                .mount("/", routes![healthcheck])
        },
        Err(error) => {
            eprintln!("Config Error: {error}");
            std::process::exit(1)
        }
    }
}

/*fn write_epub_metadata_to_json() {
    let files = read_dir("downloads").unwrap();
    for file in files {
        let file = file.unwrap();
        let path = file.path();
        if !path.is_dir() && path.extension() == Some(OsStr::new("epub")) {
            let doc = EpubDoc::new(&path).unwrap();
            //doc.metadata.insert()
            let mut metadata_file = File::create(path.with_extension("json")).unwrap();
            metadata_file.write_all(&to_string_pretty(&doc.metadata).unwrap().into_bytes()).unwrap();
        }
    }
}*/
