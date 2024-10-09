use crate::{ao3::user::User, config::Config};

use anyhow::{Error, Result};
use enum_iterator::Sequence;
use reqwest;
use scraper::{Html, Selector};
use std::collections::HashSet;
use strum_macros::{Display, EnumString};

#[derive(Debug, EnumString, PartialEq, Eq, Hash, Display, Sequence, Clone, Copy)]
pub enum DownloadFormat {
    AZW3,
    EPUB,
    MOBI,
    PDF,
    HTML,
}

pub fn get_page(id: &str, page: Option<u8>, user: Option<&User>) -> Result<Html> {
    let url = if let Some(i) = page {
        format!("https://archiveofourown.org/series/{}?page={}", id, i)
    } else {
        format!("https://archiveofourown.org/works/{id}")
    };

    let response = if let Some(i) = user {
        i.client.get(url).send()
    } else {
        reqwest::blocking::get(url)
    }
    .unwrap();

    if response.url().as_str() == "https://archiveofourown.org/users/login?restricted=true" {
        eprint!("This work/series is restricted and requires an AO3 account");
        return Err(Error::msg("Restricted Error"));
    }

    let html_content = Html::parse_document(&response.text()?);

    let error_404_selector = Selector::parse("h2.heading").unwrap();

    let unwrapped_html_content = html_content
        .select(&error_404_selector)
        .next()
        .unwrap()
        .text()
        .collect::<String>();

    if unwrapped_html_content == "Error 404" {
        eprintln!("This url does not lead to a valid work/series");
        return Err(Error::msg("URL Error"));
    }

    Ok(html_content)
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

    for fandom in &mapped_fandoms {
        if config.fandom_filter.contains_key(fandom) {
            if let Some(filter) = config.fandom_filter.get(fandom) {
                for fandom_to_remove in filter {
                    if mapped_fandoms.contains(fandom_to_remove) {
                        mapped_and_filtered_fandoms.remove(fandom_to_remove);
                    }
                }
            }
        }
    }

    return if mapped_and_filtered_fandoms.len() > 1 {
        "Multiple".to_string()
    } else {
        mapped_and_filtered_fandoms
            .iter()
            .next()
            .unwrap()
            .to_string()
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn map() {
        let config = Config {
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: HashMap::new(),
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
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: HashMap::new(),
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
    fn filter() {
        let config = Config {
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            devices: Vec::new(),
            fandom_map: HashMap::new(),
            fandom_filter: HashMap::from([
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
    fn recursive_filter() {
        let config = Config {
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            devices: Vec::new(),
            fandom_map: HashMap::new(),
            fandom_filter: HashMap::from([
                ("Fandom 1".to_owned(), vec!["Fandom 2".to_owned()]),
                ("Fandom 2".to_owned(), vec!["Fandom 3".to_owned()]),
            ]),
        };

        assert_eq!(
            filter_fandoms(
                &vec![
                    "Fandom 1".to_owned(),
                    "Fandom 2".to_owned(),
                    "Fandom 3".to_owned()
                ],
                &config
            ),
            "Fandom 1"
        );
    }

    #[test]
    fn map_and_filter() {
        let config = Config {
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
            ]),
            fandom_filter: HashMap::from([
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

    #[test]
    fn map_and_filter_recursive() {
        let config = Config {
            download_path: "some folder/some file".to_owned(),
            ao3_username: Some("test".to_owned()),
            ao3_password: Some("test".to_owned()),
            devices: Vec::new(),
            fandom_map: HashMap::from([
                ("Fandom 1 the big boy".to_owned(), "Fandom 1".to_owned()),
                ("Fandom 1 TBB".to_owned(), "Fandom 1".to_owned()),
                (
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 2".to_owned(),
                ),
                (
                    "Fandom 3 god lord big boy is back".to_owned(),
                    "Fandom 3".to_owned(),
                ),
            ]),
            fandom_filter: HashMap::from([
                ("Fandom 1".to_owned(), vec!["Fandom 2".to_owned()]),
                ("Fandom 2".to_owned(), vec!["Fandom 3".to_owned()]),
            ]),
        };

        assert_eq!(
            filter_fandoms(
                &vec![
                    "Fandom 1 the big boy".to_owned(),
                    "Fandom 1 TBB".to_owned(),
                    "Fandom 2 the big boy returns".to_owned(),
                    "Fandom 3 god lord big boy is back".to_owned()
                ],
                &config
            ),
            "Fandom 1"
        );
    }
}
