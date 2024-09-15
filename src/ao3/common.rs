use crate::ao3::user::User;

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

pub fn get_page(id: &str, page: Option<u8>, user: Option<&User>) -> Result<Html> {
    let url = if let Some(i) = page {
        format!(
            "https://archiveofourown.org/series/{}?page={}",
            id,
            i
        )
    } else {
        format!("https://archiveofourown.org/works/{id}")
    };

    let response = if let Some(i) = user {
        i.client.get(url).send()
    } else {
        reqwest::blocking::get(url)
    };

    let html_content = response?.text()?;
    Ok(Html::parse_document(&html_content))
}
