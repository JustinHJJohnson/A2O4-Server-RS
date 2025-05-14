use crate::{ao3::user::User, config::Config};

use anyhow::{Error, Result};
use enum_iterator::Sequence;
use reqwest;
use scraper::{Html, Selector};
use std::collections::HashSet;
use regex::Regex;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use url::Url;

#[derive(
    Debug,
    EnumString,
    PartialEq,
    Eq,
    Hash,
    Display,
    Sequence,
    Clone,
    Copy,
    Serialize,
    Deserialize
)]
pub enum DownloadFormat {
    AZW3,
    EPUB,
    MOBI,
    PDF,
    HTML,
}

//TODO check for SSL error page, proxy error page, timeout page
pub async fn get_page(id: &str, page: Option<u8>, user: &User) -> Result<Html> {
    let url = if let Some(i) = page {
        format!("https://archiveofourown.org/series/{id}?page={i}")
    } else {
        format!("https://archiveofourown.org/works/{id}")
    };

    let response = request_with_user(url, user).await;

    if response.url().as_str() == "https://archiveofourown.org/users/login?restricted=true" {
        eprint!("This work/series is restricted and requires an AO3 account");
        return Err(Error::msg("Restricted Error"));
    }

    let html_content = Html::parse_document(&response.text().await?);

    let error_404_selector = Selector::parse("div.errors").unwrap();
    
    let error_check = html_content.select(&error_404_selector).next();
    
    if error_check.is_some() {
        let error = error_check.unwrap()
            .text()
            .collect::<String>();

        if error == "Error 404" {
            eprintln!("This url does not lead to a valid work/series");
            return Err(Error::msg("URL Error"));
        }
    }

    Ok(html_content)
}

pub async fn get_series_pages(id: &str, user: &User) -> Result<Vec<Html>> {
    let url = format!("https://archiveofourown.org/series/{id}");
    let response = request_with_user(url, user).await;
    
    if response.status() == StatusCode::NOT_FOUND {
        eprintln!("This url does not lead to a valid work/series");
        return Err(Error::msg("URL Error"));
    }

    let response_text = response.text().await.unwrap();
    let num_pages: u8 = if response_text.contains("Pages Navigation") {
        let response_substring = response_text
            .split("Pages Navigation")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();

        Regex::new(r">\d+<")?.captures_iter(response_substring).count() as u8
    } else {
        1
    };
    
    let mut raw_html: Vec<String> = vec![response_text];

    //TODO should probably check all these responses are successes
    for page in 2..=num_pages {
        let url = format!("https://archiveofourown.org/series/{id}?page={page}");
        let response = request_with_user(url, user).await;
        raw_html.push(response.text().await?);
    }

    Ok(raw_html.iter().map(|a| Html::parse_document(a)).collect())
}

pub fn filter_fandoms(fandoms: &Vec<String>, config: &Config) -> String {
    let mut mapped_fandoms: HashSet<String> = HashSet::from_iter(fandoms.to_owned());
    
    for fandom in fandoms {
        if config.fandom_map.contains_key(fandom) {
            mapped_fandoms.remove(fandom);
            mapped_fandoms.insert(config.fandom_map.get(fandom).unwrap().to_string());
        }
    }

    let mut mapped_and_filtered_fandoms = mapped_fandoms.clone();

    for filter in &config.fandom_filter {
        if mapped_fandoms.contains(filter.0) & mapped_and_filtered_fandoms.contains(filter.0) {
            for fandom_to_remove in filter.1 {
                if fandom_to_remove == "*" {
                    mapped_and_filtered_fandoms = HashSet::from_iter([filter.0.clone()]);
                } else if mapped_fandoms.contains(fandom_to_remove) {
                    mapped_and_filtered_fandoms.remove(fandom_to_remove);
                }
            }
        }
    }

    if mapped_and_filtered_fandoms.len() > 1 {
        "Multiple".to_string()
    } else {
        mapped_and_filtered_fandoms
            .iter()
            .next()
            .unwrap()
            .to_string()
    }
}

//TODO setup rate limit of 12 per minute
async fn request_with_user(url: String, user: &User) -> Response {
    user.client.get(url).send().await.unwrap() //TODO do error handling, maybe with passed in error message
}

//TODO use regex on the raw string instead
pub fn parse_url(url: &Url) -> (String, String) {
    let mut url_path_segments = url.path_segments().unwrap().rev();
    (url_path_segments.next().unwrap().into(), url_path_segments.next().unwrap().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use indexmap::IndexMap;

    #[test]
    fn map() {
        let config = Config {
            port: 1,
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            default_format: DownloadFormat::EPUB,
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: IndexMap::new(),
        };

        assert_eq!(
            filter_fandoms(
                &vec!["Fandom 1 the big boy".to_owned(), "Fandom 1 TBB".to_owned()],
                &config
            ),
            "Fandom 1"
        );
    }

    #[test]
    fn map_lets_unmatched_fandoms_through() {
        let config = Config {
            port: 1,
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            default_format: DownloadFormat::EPUB,
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: IndexMap::new(),
        };

        assert_eq!(
            filter_fandoms(
                &vec!["Fandom 4 how is big boy possibly back once again".to_owned()],
                &config
            ),
            "Fandom 4 how is big boy possibly back once again"
        );
    }

    #[test]
    fn map_removes_all() {
        let config = Config {
            port: 1,
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            default_format: DownloadFormat::EPUB,
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: IndexMap::from([("Fandom 1".to_owned(), vec!["*".to_owned()])]),
        };

        assert_eq!(
            filter_fandoms(
                &vec![
                    "Fandom 1".to_owned(),
                    "Fandom 2".to_owned(),
                    "Fandom 3".to_owned(),
                    "Fandom 4".to_owned(),
                ],
                &config
            ),
            "Fandom 1"
        );
    }

    #[test]
    fn map_applies_in_order() {
        let config = Config {
            port: 1,
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            default_format: DownloadFormat::EPUB,
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: IndexMap::from([
                ("Fandom 1".to_owned(), vec!["Fandom 2".to_owned()]),
                ("Fandom 2".to_owned(), vec!["Fandom 1".to_owned()]),
            ]),
        };

        assert_eq!(
            filter_fandoms(
                &vec![
                    "Fandom 1".to_owned(),
                    "Fandom 2".to_owned(),
                ],
                &config
            ),
            "Fandom 1"
        );
    }

    #[test]
    fn filter() {
        let config = Config {
            port: 1,
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            default_format: DownloadFormat::EPUB,
            devices: Vec::new(),
            fandom_map: HashMap::new(),
            fandom_filter: IndexMap::from([
                ("Fandom 1".to_owned(), vec!["Fandom 2".to_owned()]),
                ("Fandom 2".to_owned(), vec!["Fandom 3".to_owned()]),
            ]),
        };

        assert_eq!(
            filter_fandoms(&vec!["Fandom 1".to_owned(), "Fandom 2".to_owned()], &config),
            "Fandom 1"
        );
    }

    #[test]
    fn map_and_filter() {
        let config = Config {
            port: 1,
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            default_format: DownloadFormat::EPUB,
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: IndexMap::from([
                ("Fandom 1".to_owned(), vec!["Fandom 2".to_owned()]),
                ("Fandom 2".to_owned(), vec!["Fandom 3".to_owned()]),
            ]),
        };

        assert_eq!(
            filter_fandoms(
                &vec![
                    "Fandom 1 the big boy".to_owned(),
                    "Fandom 1 TBB".to_owned(),
                    "Fandom 2 the big boy returns".to_owned()
                ],
                &config
            ),
            "Fandom 1"
        );
    }
}
