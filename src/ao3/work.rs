use crate::ao3::common::{get_page, DownloadFormat};

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::path::PathBuf;
use scraper::{ElementRef, Selector};
use reqwest;
use anyhow::{Error, Result};

#[derive(Debug)]
pub struct Work {
    id: String,
    title: String,
    author: String,
    download_links: HashMap<DownloadFormat, String>,
    fandoms: Vec<String>,
    relationships: Vec<String>,
    characters: Vec<String>,
    additional_tags: Vec<String>,
    series: HashMap<String, u8>
    //series: Option<Vec<SeriesLink>>
}

impl std::fmt::Display for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "id: {},\ntitle: {},\nauthor: {},\ndownload_links: {:?},\nfandoms: {:?},\nrelationships: {:?},\ncharacters: {:?},\nadditional_tags: {:?}\nseries: {:?}",
            self.id,
            self.title,
            self.author,
            self.download_links,
            self.fandoms,
            self.relationships,
            self.characters,
            self.additional_tags,
            self.series
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct SeriesLink {
    pub series_id: String,
    pub part_in_series: u8
}

impl Work {
    pub fn fandoms(&self) -> Vec<String> {
        self.fandoms.clone()
    }

    pub fn author(&self) -> String {
        self.author.clone()
    }

    pub fn get_part_in_series(&self, series_id: &String) -> Option<&u8> {
        self.series.get(series_id)
    }

    pub fn parse_work(&self, id: &str) -> Result<Work> {
        let document = get_page(id,None).expect("Failed to get the requested page");
        
        let error_header_selector = Selector::parse("h3.heading").unwrap();
        let error_message_selector = Selector::parse("div#signin>p").unwrap();
        
        let title_selector = Selector::parse("h2.title.heading").unwrap();
        let author_selector = Selector::parse("h3.byline.heading>a").unwrap();
        let downloads_selector = Selector::parse("li.download>ul>li>a").unwrap();
        let fandoms_selector = Selector::parse("dd.fandom.tags>ul>li>a").unwrap();
        let relationships_selector = Selector::parse("dd.relationship.tags>ul>li>a").unwrap();
        let characters_selector = Selector::parse("dd.character.tags>ul>li>a").unwrap();
        let additional_tags_selector = Selector::parse("dd.freeform.tags>ul>li>a").unwrap();
        let part_in_series_selector = Selector::parse("dd.series>span.series>span.position").unwrap();
    
        if document.select(&error_header_selector).next().unwrap().text().collect::<String>() == "Sorry!" {
            eprintln!("Error\n{}", document.select(&error_message_selector).next().unwrap().text().collect::<String>());
            return Err(Error::msg("Error loading work"));
        }
        
        let title: String = document.select(&title_selector).next().unwrap().text().collect();
        let author: String = document.select(&author_selector).next().unwrap().text().collect();
        let downloads_popup = document.select(&downloads_selector);
        let download_links: HashMap<DownloadFormat, String> = downloads_popup
            .map(|link| (
                DownloadFormat::from_str(&link.text().collect::<String>()).expect("Failed to parse download format enum"),
                format!("https://archiveofourown.org{}", link.value().attr("href").unwrap())
            ))
            .collect();
        let fandoms: Vec<String> = document.select(&fandoms_selector).map(|x| x.text().collect()).collect();
        let relationships: Vec<String> = document.select(&relationships_selector).map(|x| x.text().collect()).collect();
        let characters: Vec<String> = document.select(&characters_selector).map(|x| x.text().collect()).collect();
        let additional_tags: Vec<String> = document.select(&additional_tags_selector).map(|x| x.text().collect()).collect();
        let series_element = document.select(&part_in_series_selector);
        let series_links: HashMap<String, u8> = series_element
            .map(|series| (
                series
                    .child_elements().next().unwrap()
                    .value().attr("href").unwrap()
                    .split_terminator("/").skip(2).next().unwrap().to_owned(),
                series
                    .text()
                    .collect::<String>()
                    .split_whitespace()
                    .skip(1).next().unwrap()
                    .parse::<u8>().unwrap()
            )).collect();
    
        Ok(Work {
            id: id.to_owned(),
            title: title.trim().to_owned(),
            author,
            download_links,
            fandoms,
            relationships,
            characters,
            additional_tags,
            series: series_links
        })
    }

    pub fn parse_work_from_blurb(blurb: ElementRef) -> Result<Work> {
        let heading_selector = Selector::parse("h4.heading>a").unwrap();
        let fandoms_selector = Selector::parse("h5.fandoms.heading>a.tag").unwrap();
        let relationships_selector = Selector::parse("li.relationships>a.tag").unwrap();
        let characters_selector = Selector::parse("li.characters>a.tag").unwrap();
        let additional_tags_selector = Selector::parse("li.freeforms>a.tag").unwrap();
        let series_selector = Selector::parse("ul.series>li").unwrap();
        
        let mut heading = blurb.select(&heading_selector);
        let title_element = heading.next().unwrap();
        let id: String = title_element.attr("href").unwrap().split_terminator("/").skip(2).next().unwrap().to_owned();
        let title: String = title_element.text().collect();
        let author: String = heading.next().unwrap().text().collect();
        let download_links: HashMap<DownloadFormat, String> = enum_iterator::all::<DownloadFormat>()
            .map(|download_format| (
                (
                    download_format,
                    format!("https://download.archiveofourown.org/downloads/{}/work.{}", id, download_format.to_string().to_lowercase())
                )
            )).collect();
        let fandoms: Vec<String> = blurb.select(&fandoms_selector)
            .map(|fandom| fandom.text().collect()).collect();
        let relationships: Vec<String> = blurb.select(&relationships_selector)
            .map(|relationship| relationship.text().collect()).collect();
        let characters: Vec<String> = blurb.select(&characters_selector)
            .map(|character| character.text().collect()).collect();
        let additional_tags: Vec<String> = blurb.select(&additional_tags_selector)
            .map(|tag| tag.text().collect()).collect();
        let series_element = blurb.select(&series_selector);
        let series_links: HashMap<String, u8> = series_element
            .map(|series| ({
                let mut elements = series.child_elements();
                let part_in_series = elements.next().unwrap()
                    .text()
                    .collect::<String>()
                    .parse::<u8>().unwrap();
                let series_id = elements.next().unwrap()
                    .value().attr("href").unwrap()
                    .split_terminator("/").skip(2).next().unwrap().to_owned();
                (series_id, part_in_series)
            })).collect();
    
        Ok(Work {
            id: id.to_owned(),
            title: title.trim().to_owned(),
            author,
            download_links,
            fandoms,
            relationships,
            characters,
            additional_tags,
            series: series_links
        })
    }

    pub fn download_work(&self, mut file: PathBuf, format: DownloadFormat, series: bool, series_id: &String) -> std::io::Result<()> {
        let download_link = self.download_links[&format].clone();
        println!("Download link: {}", download_link);
        let work_file = reqwest::blocking::get(download_link).unwrap().bytes().unwrap();
        if series {
            file.push(format!("{} - {}.{}", self.get_part_in_series(series_id).unwrap(), self.title, format.to_string().to_lowercase()))
        } else {
            file.push(format!("{}.{}", self.title, format.to_string().to_lowercase()));
        }
        
        println!("Downloading to: {}", file.to_str().unwrap());
        
        let mut file = File::create(file)?;
        file.write_all(&work_file)?;
        Ok(())
    }
}
