#[macro_use]
extern crate rocket;

mod clients;
mod common;
mod config;
mod db;
mod domain;
mod routes;

use epub::doc::EpubDoc;
use rocket::{
    error,
    fairing::{self, Fairing, Info, Kind},
    http::{Header, Status},
    info,
    response::content,
    Build, Request, Response, Rocket,
};
use rocket_db_pools::{sqlx, Database};
use serde_json::to_string_pretty;
use std::{
    ffi::OsStr,
    fs::{read_dir, File},
    io::prelude::*,
    path::{Path, PathBuf},
};
use url::Url;

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

/*#[get("/meta")]
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
}*/

#[get("/healthcheck")]
fn healthcheck() -> (Status, String) {
    (Status::Ok, "A2O4 is running".to_string())
}

#[derive(Database)]
#[database("sqlite")]
pub struct A2O4Db(sqlx::SqlitePool);

async fn run_migrations(rocket: Rocket<Build>) -> rocket::fairing::Result {
    match A2O4Db::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("./migrations").run(&**db).await {
            Ok(_) => {
                info!("SQLite database migrations completed successfully.");
                Ok(rocket)
            }
            Err(e) => {
                error!("SQLite database migration failed: {}", e);
                Err(rocket)
            }
        },
        None => {
            error!("Failed to fetch the A2O4Db database pool from Rocket state.");
            Err(rocket)
        }
    }
}

#[launch]
async fn rocket() -> _ {
    let config = match config::read_config().await {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Config Error: {error}");
            std::process::exit(1)
        }
    };
    let port = config.port;
    let user = match domain::user::get_user(config.ao3_username, config.ao3_password).await {
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
                .merge((
                    "databases.sqlite.url",
                    format!("sqlite://{}", config.db_path),
                )),
        )
        .manage(user)
        .attach(CORS)
        .attach(A2O4Db::init())
        .attach(fairing::AdHoc::try_on_ignite(
            "SQLx Migrations",
            run_migrations,
        ))
        .mount("/", routes![index])
        .mount("/", routes![routes::download::download])
        .mount("/", routes![routes::upload::upload_work])
        .mount("/", routes![routes::upload::upload_series])
        //.mount("/", routes![meta])
        .mount("/", routes![healthcheck])
        .mount("/", routes![routes::devices::get_devices])
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
