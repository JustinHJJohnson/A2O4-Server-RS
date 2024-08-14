mod ao3;

use anyhow::Result;

fn main() -> Result<()> {
    //let work = ao3::parse_work("31751449").unwrap();
    //let work = ao3::parse_work("33158737").unwrap(); //restricted work
    let work = ao3::parse_work("555227").unwrap();
    //let series = ao3::parse_series("2796217").unwrap();
    //let series = ao3::parse_series("25849").unwrap();
    //println!();
    //println!("{}", series);
    println!("{}", work);

    //let _ = ao3::download_work(work, std::env::current_dir()?, ao3::DownloadFormat::EPUB);

    Ok(())
}
