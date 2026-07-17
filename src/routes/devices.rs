use rocket::{http::Status, serde::json::Json};

use crate::config;

type JsonResponse<T> = Result<Json<T>, (Status, String)>;

#[get("/devices")]
pub async fn get_devices() -> JsonResponse<Vec<String>> {
    let config = match config::read_config().await {
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
