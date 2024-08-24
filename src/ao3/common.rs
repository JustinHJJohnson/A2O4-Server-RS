use scraper::Html;
use anyhow::Result;
use reqwest;
use strum_macros::{Display, EnumString};
use enum_iterator::Sequence;

#[derive(Debug, EnumString, PartialEq, Eq, Hash, Display, Sequence, Clone, Copy)]
pub enum DownloadFormat {
    AZW3,
    EPUB,
    MOBI,
    PDF,
    HTML
}

pub fn get_page(id: &str, page:Option<u8>) -> Result<Html> {
    let url = if page == None {
        format!("https://archiveofourown.org/works/{id}")
    } else {
        format!(
            "https://archiveofourown.org/series/{}?page={}",
            id,
            page.unwrap()
        )
    };
    let response = reqwest::blocking::get(url);
    let html_content = response?.text()?;
    Ok(Html::parse_document(&html_content))
}