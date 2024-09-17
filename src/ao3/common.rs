use crate::ao3::user::User;

use scraper::{Html, Selector};
use anyhow::{Error, Result};
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
    }.unwrap();

    if response.url().as_str() == "https://archiveofourown.org/users/login?restricted=true" {
        eprint!("This work/series is restricted and requires an AO3 account");
        return Err(Error::msg("Restricted Error"));
    }

    let html_content = Html::parse_document(&response.text()?);

    let error_404_selector = Selector::parse("h2.heading").unwrap();

    if html_content.select(&error_404_selector).next().unwrap().text().collect::<String>() == "Error 404" {
        eprintln!("This url does not lead to a valid work/series");
        return Err(Error::msg("URL Error"));
    }

    Ok(html_content)
}
