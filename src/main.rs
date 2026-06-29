mod clients;
mod common;
mod config;
mod domain;

use crate::{
    clients::client::Client,
    common::{parse_url, DownloadFormat, PageType},
    config::read_config,
    domain::{series::Series, user, work::Work},
};

use epub::doc::EpubDoc;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Header, Status},
    response::content,
    serde::json::Json,
    Request, Response, State,
};
use serde::Deserialize;
use serde_json::to_string_pretty;
use std::{
    ffi::OsStr,
    fs::{read_dir, File},
    io::prelude::*,
    path::{Path, PathBuf},
};
use url::Url;

#[macro_use]
extern crate rocket;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[get("/")]
fn index() -> content::RawHtml<&'static str> {
    content::RawHtml("Hello 👋")
}

type JsonResponse<T> = Result<Json<T>, (Status, String)>;

#[get("/devices")]
async fn get_devices() -> JsonResponse<Vec<String>> {
    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return Err((
                Status::InternalServerError,
                format!("Config Error: {error}"),
            ))
        }
    };

    Ok(Json(
        config.devices.iter().map(|x| x.name.clone()).collect(),
    ))
}

#[derive(Deserialize)]
struct DownloadRequest {
    url: String,
    devices: Vec<String>,
    fandom_override: Option<String>,
    format: Option<DownloadFormat>,
}

#[post("/download", format = "json", data = "<request>")]
async fn download(request: Json<DownloadRequest>, user: &State<user::User>) -> (Status, String) {
    let Ok(url) = Url::parse(&request.url) else {
        return (
            Status::BadRequest,
            String::from("Could not parse provided URL"),
        );
    };

    let url_info = match parse_url(&url) {
        Ok(url_info) => url_info,
        Err(error) => return (Status::BadRequest, error.to_string()),
    };

    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (
                Status::InternalServerError,
                format!("Config Error: {error}"),
            )
        }
    };

    let devices = match config.get_devices(request.devices.clone()) {
        Ok(devices) => devices,
        Err(device) => {
            return (
                Status::BadRequest,
                format!("Could not find device {}", device),
            )
        }
    };

    let download_format = request.format.unwrap_or(config.default_format);

    match user.write_cookies() {
        Ok(()) => {}
        Err(error) => {
            return (
                Status::InternalServerError,
                format!("File error while writing cookies: {error}"),
            )
        }
    }

    match url_info.page_type {
        PageType::Work => {
            let work_result =
                Work::parse_work(&url_info.id, user, &config, request.fandom_override.clone())
                    .await;
            let Ok(work) = work_result else {
                return (Status::BadRequest, work_result.err().unwrap().to_string());
            };
            let download_result = work
                .download(
                    Path::new(&config.download_path),
                    download_format,
                    None,
                    user,
                )
                .await;
            let Ok(()) = download_result else {
                return (
                    Status::BadRequest,
                    download_result.err().unwrap().to_string(),
                );
            };
            let upload_result = config
                .upload_work_to_devices(&work, devices, download_format)
                .await;
            if let Some(error) = upload_result.failure {
                return (
                    Status::BadGateway,
                    format!(
                        "'{}'\nSuccessfully uploaded to device(s) '{}'",
                        error,
                        upload_result.successes.join(",")
                    ),
                );
            };
        }
        PageType::Series => {
            let series_result = Series::parse_series(&url_info.id, user, &config).await;
            let Ok(series) = series_result else {
                return (Status::BadRequest, series_result.err().unwrap().to_string());
            };
            let download_result = series
                .download(Path::new(&config.download_path), download_format, user)
                .await;
            let Ok(()) = download_result else {
                return (
                    Status::BadRequest,
                    download_result.err().unwrap().to_string(),
                );
            };
            let upload_result = config
                .upload_series_to_devices(&series, devices, download_format)
                .await;
            if let Some(error) = upload_result.failure {
                return (
                    Status::BadGateway,
                    format!(
                        "'{}'\nSuccessfully uploaded to device(s) '{}'",
                        error,
                        upload_result.successes.join(",")
                    ),
                );
            };
        }
    }

    (
        Status::Ok,
        format!(
            "Successfully downloaded {} with id {}",
            url_info.page_type, url_info.id
        ),
    )
}

#[derive(Clone, Deserialize)]
struct UploadWorkSeries {
    title: String,
    fandom: String,
}

#[derive(Deserialize)]
struct UploadWorkRequest {
    work: String,
    fandom: String,
    series: Option<UploadWorkSeries>,
    part_in_series: Option<u8>,
    device: String,
}

#[post("/upload/work", format = "json", data = "<request>")]
async fn upload_work_api(request: Json<UploadWorkRequest>) -> (Status, String) {
    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (
                Status::InternalServerError,
                format!("Config Error: {error}"),
            )
        }
    };

    let Some(device) = config.get_device_by_name(&request.device) else {
        return (
            Status::BadRequest,
            format!("Could not find device {}", request.device),
        );
    };

    let work = Work::test_work(
        request.work.clone(),
        request.fandom.clone(),
        request.series.clone().map(|x| x.title),
        request.part_in_series,
    );

    if let Some(unwrapped_series) = &request.series {
        let dummy_series = Series::test_series(
            &unwrapped_series.title,
            unwrapped_series.fandom.clone(),
            &config,
        )
        .unwrap();
        let upload_result = device
            .client
            .upload_work(
                &work,
                device,
                &config,
                DownloadFormat::EPUB,
                Some(&dummy_series),
            )
            .await;
        let Ok(()) = upload_result else {
            return (Status::BadRequest, upload_result.err().unwrap().to_string());
        };
    } else {
        let upload_result = device
            .client
            .upload_work(&work, device, &config, DownloadFormat::EPUB, None)
            .await;
        let Ok(()) = upload_result else {
            let error = upload_result.err().unwrap();
            println!("{}", error.root_cause());
            return (Status::BadRequest, error.to_string());
        };
    }

    (
        Status::Ok,
        format!(
            "Successfully uploaded {} to {}",
            request.work, request.device
        ),
    )
}

#[derive(Deserialize)]
struct UploadSeriesRequest {
    series: String,
    fandom: String,
    device: String,
}

#[post("/upload/series", format = "json", data = "<request>")]
async fn upload_series_api(request: Json<UploadSeriesRequest>) -> (Status, String) {
    let config = match read_config().await {
        Ok(config) => config,
        Err(error) => {
            return (
                Status::InternalServerError,
                format!("Config Error: {error}"),
            )
        }
    };

    let Some(device) = config.get_device_by_name(&request.device) else {
        return (
            Status::BadRequest,
            format!("Could not find device {}", request.device),
        );
    };

    let series = match Series::test_series(&request.series, request.fandom.clone(), &config) {
        Ok(series) => series,
        Err(error) => {
            return (
                Status::InternalServerError,
                format!("Series Upload Error: {error}"),
            )
        }
    };

    let upload_result = device
        .client
        .upload_series(&series, device, &config, DownloadFormat::EPUB)
        .await;
    let Ok(()) = upload_result else {
        let error = upload_result.err().unwrap();
        println!("{}", error.root_cause());
        return (Status::BadRequest, error.to_string());
    };

    (
        Status::Ok,
        format!(
            "Successfully uploaded {} to {}",
            request.series, request.device
        ),
    )
}

#[get("/meta")]
fn meta() -> (Status, String) {
    let mut doc = EpubDoc::new(PathBuf::from(
        "downloads/Horny on Main Nikke/1 - I'll be by your side..epub",
    ))
    .unwrap();
    //doc.go_next();
    let test = doc.get_current().unwrap();
    let a = test.0;
    println!("{}", String::from_utf8(a).unwrap());
    //println!("{}", doc.mdata("creator").unwrap().value);
    //println!("{:?}", doc.metadata);

    /*let files = read_dir("downloads").unwrap();
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
    }*/

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
                        .merge(("address", "0.0.0.0")),
                )
                .manage(user)
                .attach(CORS)
                .mount("/", routes![index])
                .mount("/", routes![download])
                .mount("/", routes![upload_series_api])
                .mount("/", routes![upload_work_api])
                .mount("/", routes![meta])
                .mount("/", routes![healthcheck])
                .mount("/", routes![get_devices])
        }
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
