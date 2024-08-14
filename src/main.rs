mod utils;

use std::collections::HashSet;
use std::str::FromStr;
use std::{collections::HashMap, fs::File};
use std::io::prelude::*;
use std::path::PathBuf;
use scraper::{Html, Selector};
use anyhow::Result;
use strum_macros::EnumString;


#[derive(Debug)]
struct Work {
    id: String,
    title: String,
    author: String,
    download_links: HashMap<DownloadFormat, String>,
    fandoms: Vec<String>,
    relationships: Vec<String>,
    characters: Vec<String>,
    additional_tags: Vec<String>
}

impl std::fmt::Display for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "id: {},\ntitle: {},\nauthor: {},\ndownload_links: {:?},\nfandoms: {:?},\nrelationships: {:?},\ncharacters: {:?},\nadditional_tags: {:?}",
            self.id,
            self.title,
            self.author,
            self.download_links,
            self.fandoms,
            self.relationships,
            self.characters,
            self.additional_tags
        )
    }
}

struct Series {
    id: String,
    title: String,
    creator: String,
    series_begun: String, //TODO make some sort of date type
    series_updated: String, //TODO make some sort of date type
    description: String,
    num_words: u32,
    num_works: u32,
    is_completed: bool,
    num_bookmarks: u32,

    //These are gotten from parsing all the works in the series
    works: Vec<Work>,
    authors: HashSet<String>,
    fandoms: HashSet<String>
}

impl std::fmt::Display for Series {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "id: {}\ntitle: {}\ncreator: {}\nseries_begun: {}\nseries_updated: {}\ndescription: {}\nnum_words: {}\nnum_works: {}\nis_completed: {}\nnum_bookmarks: {}\nworks: {:?}\nauthors: {:?}\nfandoms: {:?}",
            self.id,
            self.title,
            self.creator,
            self.series_begun,
            self.series_updated,
            self.description,
            self.num_words,
            self.num_works,
            self.is_completed,
            self.num_bookmarks,
            self.works,
            self.authors,
            self.fandoms
        )
    }
}

#[derive(Debug, EnumString, PartialEq, Eq, Hash)]
enum DownloadFormat {
    AZW3,
    EPUB,
    MOBI,
    PDF,
    HTML
}

fn main() -> Result<()> {
    //let work = parse_work("6142216").unwrap();
    //let work = parse_work("33158737").unwrap(); //restricted work
    //let work = parse_work("555227").unwrap();
    //let series = parse_series("2796217").unwrap();
    let series = parse_series("25849").unwrap();
    println!();
    println!("{}", series);
    //println!("{}", work);

    Ok(())
}

fn write_to_file(content: String, file: PathBuf) -> std::io::Result<()> {
    let mut file = File::create(file)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn get_page(id: &str, page:Option<u8>) -> Result<Html> {
    let url = if page.unwrap_or_default() == 0 {
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

fn parse_work(id: &str) -> Result<Work> {
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

    if document.select(&error_header_selector).next().unwrap().text().collect::<String>() == "Sorry!" {
        println!("Error\n{}", document.select(&error_message_selector).next().unwrap().text().collect::<String>());
    }
    
    let title: String = document.select(&title_selector).next().unwrap().text().collect();
    let author: String = document.select(&author_selector).next().unwrap().text().collect();
    let downloads_popup = document.select(&downloads_selector);
    let download_links: HashMap<DownloadFormat, String> = downloads_popup
        .map(|link| (
            DownloadFormat::from_str(&link.text().collect::<String>()).expect("Failed to parse download format enum"),
            link.value().attr("href").unwrap().to_owned()
        ))
        .collect();
    let fandoms: Vec<String> = document.select(&fandoms_selector).map(|x| x.text().collect()).collect();
    let relationships: Vec<String> = document.select(&relationships_selector).map(|x| x.text().collect()).collect();
    let characters: Vec<String> = document.select(&characters_selector).map(|x| x.text().collect()).collect();
    let additional_tags: Vec<String> = document.select(&additional_tags_selector).map(|x| x.text().collect()).collect();

    Ok(Work {
        id: id.to_owned(),
        title: title.trim().to_owned(),
        author,
        download_links,
        fandoms,
        relationships,
        characters,
        additional_tags, 
    })
}

fn parse_series(id: &str) -> Result<Series> {
    let mut document = get_page(id, Some(1)).expect("Failed to get the requested page");
    
    let error_header_selector = Selector::parse("h3.heading").unwrap();
    let error_message_selector = Selector::parse("div#signin>p").unwrap();

    let pagination_selector = Selector::parse("ol.pagination.actions>li").unwrap();
    let pagination_elements = document.select(&pagination_selector).count() as u8;
    let num_series_pages = if pagination_elements > 0 { pagination_elements / 2 - 2 } else { 1 };
    
    let title_selector = Selector::parse("h2.heading").unwrap();
    let creator_selector = Selector::parse("dl.series.meta.group>dd>a").unwrap();
    let series_date_selector = Selector::parse("dl.series.meta.group>dd").unwrap();
    let description_selector = Selector::parse("blockquote.userstuff>p").unwrap();
    let words_selector = Selector::parse("dd.words").unwrap();
    let works_selector = Selector::parse("dd.works").unwrap();
    let completed_selector = Selector::parse("dl.stats>dd").unwrap();
    let bookmarks_selector = Selector::parse("dd.bookmarks>a").unwrap();
    let work_selector = Selector::parse("li.work.blurb").unwrap();

    if document.select(&error_header_selector).next().unwrap().text().collect::<String>() == "Sorry!" {
        println!("Error\n{}", document.select(&error_message_selector).next().unwrap().text().collect::<String>());
    }

    let mut series_date_select = document.select(&series_date_selector);
    series_date_select.next(); //Skip creator field to be picked up by different selector
    
    let title: String = document.select(&title_selector).next().unwrap().text().collect::<String>().trim().to_owned();
    let creator: String = document.select(&creator_selector).next().unwrap().text().collect();
    let series_begun: String = series_date_select.next().unwrap().text().collect();
    let series_updated: String = series_date_select.next().unwrap().text().collect();
    let description: String = document.select(&description_selector).next().unwrap().text().collect();
    let raw_num_words: String = document.select(&words_selector).next().unwrap().text().collect();
    let raw_num_works: String = document.select(&works_selector).next().unwrap().text().collect();
    let raw_is_completed: String = document.select(&completed_selector).skip(2).next().unwrap().text().collect();
    let raw_num_bookmarks: String = document.select(&bookmarks_selector).next().unwrap().text().collect::<String>().trim().parse().expect("could not parse num bookmarks");

    let num_words: u32 = raw_num_words.replace(&[',', '.'][..], "").parse().expect(format!("Failed to convert {} to u32", raw_num_words).as_str());
    let num_works: u32 = raw_num_works.replace(&[',', '.'][..], "").parse().expect(format!("Failed to convert {} to u32", raw_num_works).as_str());
    let is_completed: bool = match raw_is_completed.as_str() {
        "Yes" => true,
        "No" => false,
        _ => false
    };
    let num_bookmarks: u32 = raw_num_bookmarks.replace(&[',', '.'][..], "").parse().expect(format!("Failed to convert {} to u32", raw_num_bookmarks).as_str());

    let mut works = Vec::new();
    let mut authors = HashSet::new();
    let mut fandoms = HashSet::new();
    
    for page in 1..=num_series_pages {
        if page > 1 { document = get_page(id, Some(page)).unwrap() };
        for work in document.select(&work_selector) {
            let work_id = work.value().attr("id").unwrap().chars().skip(5).collect::<String>();
            println!("loading work {work_id}");
            let parsed_work = parse_work(&work_id).unwrap();
            fandoms.extend(parsed_work.fandoms.clone());
            authors.insert(parsed_work.author.clone());
            works.push(parsed_work);
        }
    }
    
    Ok(Series {
        id: id.to_owned(),
        title,
        creator,
        series_begun,
        series_updated,
        description,
        num_words,
        num_works,
        is_completed,
        num_bookmarks,
        works,
        authors,
        fandoms
    })
}
