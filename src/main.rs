mod ao3;
mod config;

use ao3::series::Series;
use ao3::work::Work;
use ao3::common::DownloadFormat;
use ao3::user::User;
use config::read_config;

use std::env::current_dir;
use anyhow::Result;

fn main() -> Result<()> {
    let config = read_config();

    let user: Option<User> = if let (Some(username), Some(password)) = (config.ao3_username, config.ao3_password) {
        Some(User::new(&username, &password))
    } else {
        None
    };
    
    //let download_path = current_dir().unwrap().join("downloads");
    //let work = Work::parse_work("12", user.as_ref()).unwrap();
    let series = Series::parse_series("12345", user.as_ref()).unwrap();
    //let _ = series.download(download_path, DownloadFormat::EPUB);
    //let _ = work.download(download_path, DownloadFormat::EPUB);

    println!("{}", series);
    //println!("{}", work);

    Ok(())
}
