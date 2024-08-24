use crate::ao3::work::Work;
use crate::ao3::common::{get_page, DownloadFormat};

use std::collections::HashSet;
use std::fs::create_dir;
use std::path::PathBuf;
use scraper::Selector;
use anyhow::Result;

pub struct Series {
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

impl Series {
    pub fn parse_series(id: &str) -> Result<Series> {
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
        let raw_num_bookmarks: String = document.select(&bookmarks_selector).next().unwrap().text().collect::<String>().trim().parse()
            .expect("could not parse num bookmarks");
    
        let num_words: u32 = raw_num_words.replace(&[',', '.'][..], "").parse()
            .expect(format!("Failed to convert {} to u32", raw_num_words).as_str());
        let num_works: u32 = raw_num_works.replace(&[',', '.'][..], "").parse()
            .expect(format!("Failed to convert {} to u32", raw_num_works).as_str());
        let is_completed: bool = match raw_is_completed.as_str() {
            "Yes" => true,
            "No" => false,
            _ => false
        };
        let num_bookmarks: u32 = raw_num_bookmarks.replace(&[',', '.'][..], "").parse()
            .expect(format!("Failed to convert {} to u32", raw_num_bookmarks).as_str());
    
        let mut works = Vec::new();
        let mut authors = HashSet::new();
        let mut fandoms = HashSet::new();
        
        for page in 1..=num_series_pages {
            if page > 1 { document = get_page(id, Some(page)).unwrap() };
            for work in document.select(&work_selector) {
                let work_id = work.value().attr("id").unwrap().chars().skip(5).collect::<String>();
                println!("loading work {work_id}");
                let parsed_work = Work::parse_work_from_blurb(work).unwrap();
                fandoms.extend(parsed_work.fandoms());
                authors.insert(parsed_work.author());
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
    
    pub fn download(&self, path: PathBuf, format: DownloadFormat) -> std::io::Result<()> {
        let series_path = path.join(&self.title);
        create_dir(&series_path)?;
        Ok(for work in &self.works {
            let _ = work.download_work(series_path.clone(), format, true, &self.id);
        })
    }
}
