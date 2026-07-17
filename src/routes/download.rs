use std::path::Path;

use rocket::{http::Status, serde::json::Json, State};
use rocket_db_pools::Connection;
use serde::Deserialize;
use url::Url;

use crate::{
    common::{self, DownloadFormat, PageType},
    config, db,
    domain::{series::Series, user, work::Work},
    A2O4Db,
};

#[derive(Deserialize)]
pub struct DownloadRequest {
    url: String,
    devices: Vec<String>,
    fandom_override: Option<String>,
    format: Option<DownloadFormat>,
}

#[post("/download", format = "json", data = "<request>")]
pub async fn download(
    request: Json<DownloadRequest>,
    mut db: Connection<A2O4Db>,
    user: &State<user::User>,
) -> (Status, String) {
    let Ok(url) = Url::parse(&request.url) else {
        return (
            Status::BadRequest,
            String::from("Could not parse provided URL"),
        );
    };

    let url_info = match common::parse_url(&url) {
        Ok(url_info) => url_info,
        Err(error) => return (Status::BadRequest, error.to_string()),
    };

    let config = match config::read_config().await {
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
            let db_insert_result = db::insert_work(&mut **db, &work).await;
            let Ok(()) = db_insert_result else {
                return (
                    Status::InternalServerError,
                    db_insert_result.err().unwrap().to_string(),
                );
            };
            let upload_result = work
                .upload_to_devices(&config, devices, download_format)
                .await;
            if let Err(error) = upload_result {
                return (Status::BadGateway, error.to_response_string());
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
            let db_insert_result = db::insert_series(db, &series).await;
            let Ok(()) = db_insert_result else {
                return (
                    Status::InternalServerError,
                    db_insert_result.err().unwrap().to_string(),
                );
            };
            let upload_result = series
                .upload_to_devices(&config, devices, download_format)
                .await;
            if let Err(error) = upload_result {
                return (Status::BadGateway, error.to_response_string());
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
