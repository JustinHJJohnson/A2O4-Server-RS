use crate::ao3::common::{filter_fandoms, get_page, DownloadFormat};
use crate::ao3::user::User;
use crate::config::Config;

use anyhow::Result;
use scraper::{ElementRef, Selector};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug)]
pub struct Work {
    id: String,
    title: String,
    author: String,
    download_links: HashMap<DownloadFormat, String>,
    fandoms: Vec<String>,
    pub filtered_fandom: String,
    relationships: Vec<String>,
    characters: Vec<String>,
    additional_tags: Vec<String>,
    series: HashMap<String, SeriesLink>,
}

impl std::fmt::Display for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "id: {},\ntitle: {},\nauthor: {},\ndownload_links: {:?},\nfandoms: {:?},\nfiltered_fandoms: {:?},\nrelationships: {:?},\ncharacters: {:?},\nadditional_tags: {:?}\nseries: {:?}",
            self.id,
            self.title,
            self.author,
            self.download_links,
            self.fandoms,
            self.filtered_fandom,
            self.relationships,
            self.characters,
            self.additional_tags,
            self.series
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SeriesLink {
    pub series_id: String,
    pub series_name: String,
    pub part_in_series: u8,
}

impl Work {
    pub fn fandoms(&self) -> Vec<String> {
        self.fandoms.clone()
    }

    pub fn author(&self) -> String {
        self.author.clone()
    }

    pub fn get_series_link(&self, series_id: &String) -> Option<&SeriesLink> {
        self.series.get(series_id)
    }

    pub fn get_filename(&self, format: DownloadFormat, series_id: Option<&String>) -> String {
        if series_id != None && self.get_series_link(series_id.unwrap()) != None {
            format!(
                "{} - {}.{}",
                self.get_series_link(series_id.unwrap())
                    .unwrap()
                    .part_in_series,
                self.title,
                format.to_string().to_lowercase()
            )
        } else {
            format!("{}.{}", self.title, format.to_string().to_lowercase())
        }
    }

    pub fn parse_work(id: &str, user: Option<&User>, config: &Config) -> Result<Work> {
        println!("loading work {}", id);
        let document = get_page(id, None, user).expect("Failed to get the requested page");

        let title_selector = Selector::parse("h2.title.heading").expect("Error parsing title");
        let author_selector = Selector::parse("h3.byline.heading>a").expect("Error parsing author");
        let downloads_selector =
            Selector::parse("li.download>ul>li>a").expect("Error parsing download links");
        let fandoms_selector =
            Selector::parse("dd.fandom.tags>ul>li>a").expect("Error parsing fandom tags");
        let relationships_selector = Selector::parse("dd.relationship.tags>ul>li>a")
            .expect("Error parsing relationship tags");
        let characters_selector =
            Selector::parse("dd.character.tags>ul>li>a").expect("Error parsing character tags");
        let additional_tags_selector =
            Selector::parse("dd.freeform.tags>ul>li>a").expect("Error parsing additional tags");
        let part_in_series_selector = Selector::parse("dd.series>span.series>span.position")
            .expect("Error parsing part in series");

        let title: String = document
            .select(&title_selector)
            .next()
            .unwrap()
            .text()
            .collect();
        let author: String = document
            .select(&author_selector)
            .next()
            .unwrap()
            .text()
            .collect();
        let downloads_popup = document.select(&downloads_selector);
        let download_links: HashMap<DownloadFormat, String> = downloads_popup
            .map(|link| {
                (
                    DownloadFormat::from_str(&link.text().collect::<String>())
                        .expect("Failed to parse download format enum"),
                    format!(
                        "https://archiveofourown.org{}",
                        link.value().attr("href").unwrap()
                    ),
                )
            })
            .collect();
        let fandoms: Vec<String> = document
            .select(&fandoms_selector)
            .map(|x| x.text().collect())
            .collect();
        let relationships: Vec<String> = document
            .select(&relationships_selector)
            .map(|x| x.text().collect())
            .collect();
        let characters: Vec<String> = document
            .select(&characters_selector)
            .map(|x| x.text().collect())
            .collect();
        let additional_tags: Vec<String> = document
            .select(&additional_tags_selector)
            .map(|x| x.text().collect())
            .collect();
        let series_element = document.select(&part_in_series_selector);
        let series_links: HashMap<String, SeriesLink> = series_element
            .map(|series| {
                let series_name_element = series.child_elements().next().unwrap();
                let series_id = series_name_element
                    .value()
                    .attr("href")
                    .unwrap()
                    .split_terminator("/")
                    .skip(2)
                    .next()
                    .unwrap()
                    .to_owned();

                (
                    series_id.clone(),
                    SeriesLink {
                        series_name: series_name_element
                            .text()
                            .collect::<String>()
                            .split_whitespace()
                            .filter(|chunk| *chunk != "series")
                            .collect::<Vec<&str>>()
                            .join(" "),
                        series_id,
                        part_in_series: series
                            .text()
                            .collect::<String>()
                            .split_whitespace()
                            .skip(1)
                            .next()
                            .unwrap()
                            .parse::<u8>()
                            .unwrap(),
                    },
                )
            })
            .collect();

        println!("Work loaded");

        Ok(Work {
            id: id.to_owned(),
            title: title.trim().to_owned(),
            author,
            download_links,
            fandoms: fandoms.clone(),
            filtered_fandom: filter_fandoms(&fandoms, config),
            relationships,
            characters,
            additional_tags,
            series: series_links,
        })
    }

    pub fn parse_work_from_blurb(
        blurb: ElementRef,
        series_name: &String,
        config: &Config,
    ) -> Result<Work> {
        let heading_selector = Selector::parse("h4.heading>a").expect("Error parsing heading");
        let fandoms_selector =
            Selector::parse("h5.fandoms.heading>a.tag").expect("Error parsing fandom tags");
        let relationships_selector =
            Selector::parse("li.relationships>a.tag").expect("Error parsing relationship tags");
        let characters_selector =
            Selector::parse("li.characters>a.tag").expect("Error parsing character tags");
        let additional_tags_selector =
            Selector::parse("li.freeforms>a.tag").expect("Error parsing additional tags");
        let series_selector = Selector::parse("ul.series>li").expect("Error parsing series");

        let mut heading = blurb.select(&heading_selector);
        let title_element = heading.next().unwrap();
        let id: String = title_element
            .attr("href")
            .unwrap()
            .split_terminator("/")
            .skip(2)
            .next()
            .unwrap()
            .to_owned();
        let title: String = title_element.text().collect();

        println!("  Parsing work {} - {}", id, title);

        let author: String = heading.next().unwrap().text().collect();
        let download_links: HashMap<DownloadFormat, String> =
            enum_iterator::all::<DownloadFormat>()
                .map(|download_format| {
                    (
                        download_format,
                        format!(
                            "https://download.archiveofourown.org/downloads/{}/work.{}",
                            id,
                            download_format.to_string().to_lowercase()
                        ),
                    )
                })
                .collect();
        let fandoms: Vec<String> = blurb
            .select(&fandoms_selector)
            .map(|fandom| fandom.text().collect())
            .collect();
        let relationships: Vec<String> = blurb
            .select(&relationships_selector)
            .map(|relationship| relationship.text().collect())
            .collect();
        let characters: Vec<String> = blurb
            .select(&characters_selector)
            .map(|character| character.text().collect())
            .collect();
        let additional_tags: Vec<String> = blurb
            .select(&additional_tags_selector)
            .map(|tag| tag.text().collect())
            .collect();
        let series_element = blurb.select(&series_selector);
        let series_links: HashMap<String, SeriesLink> = series_element
            .map(|series| {
                let mut elements = series.child_elements();
                let part_in_series = elements
                    .next()
                    .unwrap()
                    .text()
                    .collect::<String>()
                    .parse::<u8>()
                    .unwrap();
                let series_id = elements
                    .next()
                    .unwrap()
                    .value()
                    .attr("href")
                    .unwrap()
                    .split_terminator("/")
                    .skip(2)
                    .next()
                    .unwrap()
                    .to_owned();

                (
                    series_id.clone(),
                    SeriesLink {
                        series_name: series_name.clone(),
                        series_id,
                        part_in_series,
                    },
                )
            })
            .collect();

        println!("  Work parsed\n");

        Ok(Work {
            id: id.to_owned(),
            title: title.trim().to_owned(),
            author,
            download_links,
            fandoms: fandoms.clone(),
            filtered_fandom: filter_fandoms(&fandoms, config),
            relationships,
            characters,
            additional_tags,
            series: series_links,
        })
    }

    pub fn download(
        &self,
        download_folder: &Path,
        format: DownloadFormat,
        series_id: Option<&String>,
    ) -> std::io::Result<()> {
        let download_link = self.download_links[&format].clone();
        println!("Download link: {}", download_link);

        let work = reqwest::blocking::get(download_link)
            .unwrap()
            .bytes()
            .unwrap();
        let download_path = download_folder.join(self.get_filename(format, series_id));

        println!("Downloading to: {}", download_folder.to_str().unwrap());

        let mut work_file = File::create(download_path)?;
        work_file.write_all(&work)?;
        Ok(())
    }
}
