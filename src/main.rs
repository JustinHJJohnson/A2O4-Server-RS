mod ao3;

use ao3::series::Series;
use ao3::work::Work;
use ao3::common::DownloadFormat;
use ao3::user::User;

use std::env::current_dir;
use anyhow::Result;

fn main() -> Result<()> {
    //TODO setup config system and load user details from it
    let user = User::new("username", "password");
    
    let download_path = current_dir().unwrap().join("downloads");
    let series = Series::parse_series("12345678", Some(&user)).unwrap(); //restricted series
    let _ = series.download(download_path, DownloadFormat::EPUB);
    //println!();
    println!("{}", series);
    //println!("{}", work);

    Ok(())
}
