mod ao3;
mod config;
mod sftp;

use rocket::http::uri::Error;
use rocket::tokio::io::split;
use serde::Deserialize;
use rocket::http::{Status, ContentType};
use rocket::response::{status, Response};
use rocket::serde::json::Json;

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
fn download(request: Json<DownloadRequest<'_>>) -> (Status, &'static str) {
    let split_url: Vec<&str> = request.url.split('/').collect();
    println!("{:?}", split_url);

    if split_url.len() > 5 {
        return (Status::BadRequest, "That was a dogshit request")
    }

    let url_type = *split_url.get(3).unwrap();
    let id = *split_url.get(4).unwrap();
    println!("type: {}, id: {}", url_type, id);
    (Status::Ok, "")
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![download])
}
